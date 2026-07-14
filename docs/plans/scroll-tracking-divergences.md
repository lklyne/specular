# Scroll tracking — divergences from the plan

Running log of where the implementation departs from `docs/plans/scroll-tracking.md`.
Each entry: what the plan said, what we did instead, and why.

## Status

- [x] Phase 1 — Broadcast the page's absolute scroll offset
- [x] Phase 2 — The transform learns about scroll
- [x] Phase 3 — The document-anchored region variant
- [x] Phase 4 — Clicking a comment scrolls its page to it
- [x] Docs — ADR 0029 amended + CONTEXT.md updated (Phase 3);
  `docs/file-formats.md` documents the `docRect` anchor shape (docs pass).

## Environment / verification baseline (pre-change)

- `node_modules/.bin` is empty (hoisted-linker quirk) and `pnpm <script>`
  triggers a deps-status `pnpm install` that fails on an egress-blocked
  `codeload.github.com` 403. **Workaround:** invoke binaries via node directly
  — `node node_modules/typescript/bin/tsc --noEmit -p tsconfig.node.json`,
  `node node_modules/vitest/vitest.mjs run --config vitest.<x>.config.ts`.
- Electron's binary download is egress-blocked, so `node_modules/electron` has
  no `path.txt`/`dist`. **5 unit suites fail** for this reason alone
  (`binding-handlers-focus-restore`, `claude-spawner`, `doc-restore-roundtrip`,
  `layer-stack`, `page-bounds` — all import real electron). Unrelated to this
  feature; recorded so a later reviewer doesn't attribute them to scroll work.
- **Commit signing is non-functional here:** the configured SSH signing key
  (`/home/claude/.ssh/commit_signing_key.pub`) is a 0-byte placeholder with no
  private counterpart and no ssh-agent, so `commit.gpgsign=true` silently
  produces unsigned commits. Disabled it locally to avoid the failing-sign
  noise. Commits are correctly authored (`noreply@anthropic.com` / `Claude`);
  GitHub's "Unverified" badge is an environment limitation, not a config error.
- **Green baseline confirmed:** node + web typecheck clean; unit 819 tests pass
  (79/84 suites; 5 electron-blocked); integration 135 tests pass (23/23 suites).
  Integration uses `tests/integration/electron-stub.ts`, so it is unaffected.

## Divergences

### Phase 1

1. **`CanvasScenePageEntity.scrollX/scrollY` are required `number`, not optional.**
   Plan (line 89) said "defaulting to 0". Implemented as required fields with the
   single default applied at the only production builder site
   (`backgroundPageOverlays`, `?? 0`). Cleaner for phase-2 consumers, which never
   have to null-check; no other production construction site exists.
2. **Load seeding uses `onDomReady()` only**, not a separate `did-finish-load`
   hook. Plan (line 82) offered the choice; the preload has no `did-finish-load`
   seam and `onDomReady` already fires on every navigation, covering scroll
   restore, so it stayed preload-side per the plan's "preload is simplest."
3. **`resolveScrollTarget()` returns a non-null `Element`** (fallback chain
   `elementFromPoint → scrollable ancestor → document.scrollingElement →
   document.documentElement`). This makes the pre-existing `if (!target)`
   no-scroll-target guard in the `dispatchScroll` handler unreachable. It was
   already effectively dead before this change (`documentElement` is non-null),
   so the guard was left untouched to keep the diff surgical and behavior
   identical — flagged, not a regression.
4. **Offset is rounded to integer CSS pixels** before dedupe/send
   (`Math.round`), mirroring `annotation-bbox-tracker`, so sub-pixel jitter
   doesn't spam IPC. Plan didn't specify; matches the sibling pattern.

_Plan line references were otherwise accurate._

### Phase 2

1. **`pageDocumentToScreen` takes a `frame: PageFrameKind = 'content'` param**
   the plan's pseudocode (line 114) omitted. Added so document-anchored callers
   can select the content vs entity (device-shell) frame exactly like
   `pageViewportToScreen` callers — otherwise a page-anchored popover in the
   device-shell case couldn't target the content frame. Passes straight through.
2. **`PageScreenFrame.scrollX/scrollY` are optional** (default-0 in the
   transform), so pre-existing `pageViewportToScreen` callers — which only ever
   hold viewport-space rects — are untouched. `CanvasScenePageEntity`
   (scroll fields required since Phase 1) satisfies it structurally.

### Phase 3

1. **No `pageId` in the region anchor (contra plan line 137's sketch).** The
   plan sketched the page-anchored arm as `{ type:'region'; docRect; pageAnchor:… }`.
   Per ADR 0029's Amendment, `Annotation.pageAnchor.pageId` is the single
   page-binding read, so storing a page id (or a whole pageAnchor) inside the
   anchor would duplicate it and invite divergence. The two arms are therefore
   `{ type:'region'; canvasRect }` and `{ type:'region'; docRect }`, narrowed by
   `'docRect' in anchor`; the docRect's page is `annotation.pageAnchor.pageId`.
   The plan's line-137 `pageAnchor:` was a conceptual note, confirmed.
2. **`regionCanvasRect(annotation)` runtime helper — plan didn't name it.** The
   plan named `canvasRectToPageDocRect` (creation-side) but left the inverse
   unnamed. Added `regionCanvasRect` in `page-anchor-state.ts` for main-side
   consumers needing a docRect region's *current* canvas rect (tracks page move
   + scroll): canvasRect variant returns as-is, docRect variant inverts through
   the live page body + scroll, null if the page is gone. Its param is widened
   to `Pick<Annotation, 'anchor' | 'pageAnchor'>` so the HTTP create path (no
   full annotation yet) can reuse it.
3. **Consumers touched beyond the plan's enumerated list (D1–D5):**
   - `src/main/agent-fix/prompt-builder.ts` (region prompt line) — reads the
     region rect in canvas coords; switched to `regionCanvasRect`. The union
     split surfaced it in the node typecheck; the plan's D-list missed it.
   - `src/main/routes/annotations.ts` POST handler passes a synthetic
     `{ anchor }` (no full annotation exists pre-creation) to the now
     annotation-taking `annotationAnchorPosition`; the region canvasRect it
     carries is the marquee, so the cursor lands correctly.
4. **Conversion co-located in a `anchoredRequestAnchor` helper**, called from
   `createAnnotationInternal` right after `anchorPageId`/`pageAnchor` are
   computed (same `anchorPageId` gate feeds both), guaranteeing docRect and
   pageAnchor are written together and cannot diverge. `region-select.ts` still
   passes `{ type:'region', canvasRect }` — converted through the one gate.
5. **Two unit suites needed an electron-free stub** (`workspace-annotations.test.ts`,
   `prompt-builder.test.ts`): both now transitively import `page-anchor-state`
   (which imports `runtime-context` → electron), so each `vi.mock`s
   `page-anchor-state`. The real conversion is covered by the integration suite.
   No behavior change — restores the pre-Phase-3 unit baseline (only the 5
   electron-blocked suites fail).
6. **Renderer clip/edge-fade modeled, not shared.** `AnnotationsLayer.tsx` gets
   a local `regionContentOpacity` mirroring `CommentBadgesLayer`'s private
   `badgeOpacity` (same content-frame overflow math, `REGION_FADE_MARGIN=48`),
   rather than exporting the badge helper — the plan said "reuse/model". Inline
   opacity is applied only while a page-anchored region is fading (opacity < 1)
   so a fully-visible region keeps its class-based `opacity-50 hover:opacity-100`.
7. **`translateAnnotationsAnchoredToPage` deleted** along with both call sites
   (`applyDragDelta`, `nudgeSelection` in `document-commands.ts`) and its import.
   Grep confirms no remaining callers in `src/`. A docRect is page-relative, so
   drag/nudge move it via the transform — no anchor translation.

### Phase 4

1. **Target computation extracted as a pure function**, per the task's
   testability requirement (the plan didn't name it). `computeAnnotationScrollTarget(annotation)`
   in the new `src/main/runtime/annotation-scroll-target.ts` returns
   `{ pageId; documentY } | null` — documentY in the page's document CSS px, or
   null when there's nothing to reveal. Pure and synchronous, so it's assertable
   under the electron stub (the IPC send is not). The impure `dispatchScrollToAnnotation`
   in the same module layers the ~1/3-down offset, the current-scroll delta, and
   the `sendPageIpc(dispatchScroll)` ramp on top.
2. **Phase-1 offset broadcast extended to carry `scrollHeight`.** The page anchor
   case needs the document height (`offsetY × height`), which main didn't know.
   Added `scrollHeight` (of the same `resolveScrollTarget()` container) to
   `flushScrollOffset` in `page-content.ts`, to the `page-scroll-offset` IPC
   payload (`ipc-contract.ts`), stored on the runtime `Page` (`runtime-entities.ts`,
   initialized in `page-factory.ts`), and gated into the dedupe/store in
   `register-page-chrome-ipc.ts`. **Main-side only** — not added to
   `CanvasScenePageEntity`, since no renderer consumes it (plan permitted this).
3. **Dispatch hooked into the shared `annotation-open-thread` handler**
   (`register-annotation-inspection-ipc.ts`), fire-and-forget after the existing
   focus logic. **Ungated by surface:** the channel is invoked from the panel
   (`CommentsPane`), the sidebar (`SidebarCanvasTree`), and the canvas region
   overlay (`canvas-bg`). All three mean "open this comment", and revealing its
   content is the intended behavior in each — canvas-anchored/canvas-point
   comments already no-op (null target), so no surface produces a surprising
   scroll. Not carrying a surface flag through the payload keeps the handler and
   the IPC contract unchanged.
4. **Element anchor: reused `queryPageElements` for live re-resolution**, no new
   IPC. `dispatchScrollToAnnotation` re-resolves the anchor's selector against
   the live page and reads `inspectionPayload`'s `position.documentY`
   (`rect.top + window.scrollY`, already computed page-side). Falls back to the
   pure function's stored-bbox target (`boundingBox.y + page.scrollY`) when the
   selector is stale/empty or the query fails — never throws. Element anchors
   already scroll-follow, so this is the secondary/best-effort path the plan
   described.
5. **1/3-down + delta math** (in `dispatchScrollToAnnotation`):
   `targetScrollY = max(0, documentY − pageViewportHeight/3)`, `deltaY =
   targetScrollY − page.scrollY`, dispatched as `{ x: width/2, y: height/2,
   deltaX: 0, deltaY }`. `pageViewportHeight`/width come from
   `pageBodyCanvasBounds(page)` (the body is 1:1 with the page's CSS viewport at
   this layer); the x/y probe point is the viewport center so `resolveScrollTarget`
   finds the real (possibly inner) scroll container.
6. **Tests in a new file** `tests/integration/annotation-scroll-target.test.ts`
   (modeled on `annotation-page-anchor.test.ts`) rather than extending the anchor
   file — separate concern (scroll targeting vs. anchor lifecycle). Asserts the
   pure function across all anchor cases; the IPC dispatch is not asserted (not
   observable under the stub, and the element path's async query would risk a
   5 s stub timeout). Mutation-verified: page-anchored `docRect.y` target
   (revert the region branch to `return null`); canvas-anchored → null (drop the
   `'docRect' in anchor` gate).

### Docs still owed (unchanged from Phase 3)

`docs/file-formats.md` `docRect` anchor shape. Deferred to the docs pass; not a
code change.
