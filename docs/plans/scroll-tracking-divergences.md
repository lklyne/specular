# Scroll tracking — divergences from the plan

Running log of where the implementation departs from `docs/plans/scroll-tracking.md`.
Each entry: what the plan said, what we did instead, and why.

## Status

- [x] Phase 1 — Broadcast the page's absolute scroll offset
- [x] Phase 2 — The transform learns about scroll
- [x] Phase 3 — The document-anchored region variant
- [ ] Phase 4 — Clicking a comment scrolls its page to it
- [~] Docs — ADR 0029 amended, CONTEXT.md updated (Phase 3). file-formats.md
  `docRect` shape still owed (defer to the docs pass with Phase 4).

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
