# Page-anchoring refactor — deepening the annotation + anchor architecture

**Status:** planned
**Date:** 2026-07-13
**Branch context:** follows the two feature commits on `claude/annotation-anchoring-generalization-jvgx8a` — annotation hide-on-navigation + sidebar nesting (`7fde100`), and the generic `PageAnchor` utility for stickies/drawings (`cae3aa7`, [ADR 0029](../adr/0029-page-anchored-entities.md)).
**Source:** an `/improve-codebase-architecture` pass over the feature surface. Every file:line below was verified against this branch.

## Why

The anchoring feature works and is tested, but the audit found the architecture around it is shallower than it should be, in ways that compound if we build scroll tracking (ADR 0029's named follow-up) on top:

1. **A dead module still runs.** The preload badge renderer (`src/preload/comment-badges.ts`) has been behind `PAGE_COMMENT_BADGES_ENABLED = false` since the above-view badge layer replaced it. ~250 of its 430 lines are unreachable, yet it still injects three overlay divs into every page and burns a rAF per scroll frame recomputing an always-empty badge set — and main still fans an annotation payload to every page whose only consumer is this dead code.
2. **The document-binding gate is scattered.** "Is this item on its page's current document?" has one predicate (`matchesPageUrl`) but six wrappers, three page-lookup strategies, and an asymmetric seam: anchored *entities* are gated once, main-side, in the scene build; *annotations* ship raw in the layout broadcast and four above-view layers re-derive visibility independently.
3. **The same concept is modeled twice.** An entity's page binding is one `PageAnchor { pageId, pageUrl }`; an annotation's is a three-way lookup (`anchor.pageId` for element/page anchors, `metadata.regionComponents[0].pageId` for regions, URL stamped separately into `metadata.pageUrl`). Everything downstream doubles: two "which page owns this" accessors, two sidebar child-row builders, two row types.
4. **The page-viewport→screen transform is copy-pasted four times**, and one copy has already diverged. This is exactly the math scroll tracking must touch.

Vocabulary: **module / interface / seam / depth / locality / leverage** as defined in `.claude/skills/improve-codebase-architecture/LANGUAGE.md`; domain terms per `CONTEXT.md` (§Annotations, §Page anchoring).

## Findings census (evidence)

### Document-binding gate call sites

| Site | Wrapper | Page lookup | Action |
|---|---|---|---|
| `canvas-layout-data.ts:174` `annotationsForPage` | `annotationMatchesPageUrl` | `findPageById` + live `webContents.getURL()` | filters (feeds only the dead preload path) |
| `page-anchor-state.ts:139` `entityHiddenByPageAnchor` | inlines `matchesPageUrl` | `findPageById(...).url` | filters entities from scene (`canvas-layout-data.ts:335`) |
| `sidebar-builder.ts:131` (annotations) | `annotationMatchesPageUrl` | threaded `page.url` | dims (`onCurrentPage`) |
| `sidebar-builder.ts:162` (entities) | `matchesPageUrl` | threaded `page.url` | dims (`onCurrentPage`) |
| `CommentBadgesLayer.tsx:132` | `annotationMatchesPageUrl` | `pagesById` Map from `layoutData.entities` | filters badges |
| `AnnotationsLayer.tsx:11` `regionPageStillCurrent` | `annotationMatchesPageUrl` | inline `layoutData.entities.find` | filters region rects |
| `useAnnotationThreadState.ts:15` `annotationPageIsCurrent` | `annotationMatchesPageUrl` | inline `layoutData.entities.find` | closes thread; gates live-bbox subs (`App.tsx:572,584`) |

`regionPageStillCurrent` and `annotationPageIsCurrent` are near-verbatim duplicates. `pageAnchorOnCurrentUrl` (`page-anchor.ts:73`) is exported but has no production caller. `annotationsForPage` reads the live `webContents.getURL()` while every other site reads the cached `page.url` field — a subtle inconsistency.

### Dead preload badge module

- Kill switch: `comment-badges.ts:7` applied at `:227`; grouping/anchorKey (`:229-296`), badge DOM + listeners (`:353-412`), and the entire hover-preview machine (`:105-219`) are unreachable.
- Still live: `setPageAnnotations` (`:42`), the three injected overlay divs (`:54-103`, one referenced by `gesture-forwarding.ts:7`), and per-scroll-frame no-op renders (`page-content.ts:92,379,442,730,739,757`). `isCommentHoverActive()` at `page-content.ts:748` always returns false.
- Feeder: `layout-engine.ts:499-506` (`annotationsForPage` → `pageAnnotationsUpdate`) and the `did-finish-load` resend in `page-factory.ts:217-219`.
- **Not involved** in the focused-view "Show/Hide other items" eye (verified): that toggle lives in `layout-engine.ts:342-365` (hides other pages' native views) and `hideContext` gating in both renderer Apps (`above-view/App.tsx:624,1220`, `canvas-bg/App.tsx:59`), which gates the *live* `CommentBadgesLayer`. The eye actually gates the dead feeder (per-page loop `continue`s at `:353` before reaching `:499`), not the reverse.
- `comment-hover-overlay.ts` is a different, live concern (comment-tool pointer previews) — untouched, though it duplicates the dashed-outline styling that also appears in `CommentBadgesLayer.tsx:60` and `CommentsLayer.tsx:202`.

### Page-viewport→screen transform copies

`CommentBadgesLayer.tsx:226` (`elementAnnotationRect`), `annotationMath.ts:284` (`pendingElementScreenRect`), `useAnnotationDraftState.ts:205`, and `annotationMath.ts:199` (element branch of `annotationScreenPos` — the divergent one, with its own insets/clamping).

### Anchor lifecycle wiring (for reference; phase 5)

`reanchorEntityById`: `document-commands.ts:434` (nudge), `:540` (`finalizeDrag`), `:737` (`createTextEntity`), `:790` (`createDrawingEntity`). `withPageAnchoredEntityIds`: `document-commands.ts:426`, `register-canvas-drag-ipc.ts:161`. Opting a third kind (shape) in today = 3 hand-edited files, with the per-kind persisted-field lists mirrored in 4 places in `shape-entity-state.ts` alone. Sidebar nesting and scene filtering come free once the kind is in `anchorableEntities()`.

## Goals

- One authoritative document-binding gate per process concern, at the layout-broadcast seam (main owns truth → renderers display).
- One representation of "hooked to a page" across annotations and entities.
- One page-viewport→screen transform module — the seam scroll tracking will extend.
- Net-negative line count in phases 1–3.

## Non-goals

- **Scroll tracking itself.** This refactor is the prerequisite, not the feature. (ADR 0029 follow-ups.)
- **New anchorable kinds** (shape/file). Phase 5 is triggered by demand, not scheduled.
- **Changing anchoring semantics.** Placement-decides, center-inside-body, group-wins, hide-on-navigation all stay as shipped (ADR 0029).
- Re-litigating ADR 0006's resting-visual asymmetry or ADR 0021's binary show/hide.

## Phases

Ordered by leverage-per-risk; each phase lands independently green (typecheck + unit + integration) and is a separate commit/PR.

### Phase 1 — Delete the dead preload badge module

**Change**
- Delete the dead halves of `src/preload/comment-badges.ts`: grouping/anchorKey, badge DOM, hover preview, overlay-div injection, render keying. Expect the file to disappear entirely unless a live scrap remains — `setPageAnnotations` goes too once the feeder is gone.
- Delete the feeder: `layout-engine.ts:499-506`, the `did-finish-load` resend in `page-factory.ts:217-219`, `page.lastPageAnnotationsKey`, and the `pageAnnotationsUpdate` channel from `ipc-contract.ts` if no other sender/listener remains.
- Sweep: `queueRenderCommentBadges` call sites in `page-content.ts`, the always-false `isCommentHoverActive()` check at `page-content.ts:748`, the `#__canvas-comment-badges-layer` entry in `gesture-forwarding.ts:7`.
- Decide the fate of `annotationsForPage` (`canvas-layout-data.ts:174`): it loses its only consumer here, but phase 2 gives it a successor role — keep the function, delete only the fan-out.

**Risk / verification**
- Low. The live badge path (`CommentBadgesLayer.tsx`, gated by `hideContext`) is untouched — the focused-view eye behaves identically.
- Verify `comment-hover-overlay.ts` (comment-tool page paints) still works: activate comment tool over a page, hover elements, drag a marquee.
- Tests: suites stay green; no new tests (deletion). Grep proves no remaining reference to the deleted channel/ids. Run `pnpm test:integration` — `pages.test.ts` exercises `did-finish-load` side effects.

### Phase 2 — One document-binding gate at the layout-broadcast seam

**Change**
- `buildCanvasLayoutData` filters `annotations` the way it already filters anchored entities: an annotation whose context page exists and shows a different URL is dropped from the payload. One new main-side helper (natural home: extend `page-anchor-state.ts` or a small `document-binding.ts`) that both the annotation filter and `entityHiddenByPageAnchor` share, using **one** page-lookup strategy (`findPageById` + cached `page.url`; retire the live-`getURL()` variant in `annotationsForPage` for consistency).
- Renderer deletions that fall out:
  - URL filter in `commentBadgesForLayout` (`CommentBadgesLayer.tsx:129-133`) and its `pagesById` URL use.
  - `regionPageStillCurrent` (`AnnotationsLayer.tsx:11-19`).
  - `annotationPageIsCurrent` + the thread auto-close effect (`useAnnotationThreadState.ts:15-26,84-89`) — the pre-existing "open thread vanished from payload → close" effect (`:55-59` pattern) now covers navigation for free.
  - The `annotationPageIsCurrent` gates in the live-bbox subscription memo (`App.tsx:572,584`) — hidden annotations never reach the renderer, so they can't be subscribed.
  - Dead export `pageAnchorOnCurrentUrl` (`page-anchor.ts:73`) — delete or make it the shared helper's name.
- Consumers to re-point:
  - `canvas-bg/App.tsx:92` `annotationCount` (dev debug badge) — counts visible annotations after this change; acceptable, or add an explicit total if anyone cares.
  - Right-details panel is unaffected — it gets the full record from its own payload (`inspect-session.ts:446`), which stays unfiltered by design (the panel is the archive).
  - Sidebar keeps its own dim logic (it *shows* off-URL items dimmed rather than hiding them — different policy, same shared predicate).

**Semantics note** (unchanged, now enforced in one place): opening a thread from the panel for a navigated-away annotation zooms to the page but shows no popover — the annotation isn't in the overlay payload. Today the popover opens and instantly auto-closes; net behavior identical.

**Tests**
- Integration (`tests/integration/`, harness captures broadcasts): create page + element/page/region annotations → navigate (`page.url = ...`) → assert the captured `layoutUpdate` payload omits them; navigate back → present. Mutation: remove the new filter in `buildCanvasLayoutData` → test fails.
- Renderer unit tests shrink: the URL-gating cases in `tests/unit/annotation-url-visibility.test.ts` move from `commentBadgesForLayout` to the main-side helper; badge tests keep only layout math.
- Per `src/main/runtime/CLAUDE.md`, no `workspace-*.ts` behavior change here (read-side only), but run the sync/undo suites anyway.

### Phase 3 — One page-viewport→screen transform module

**Change**
- Extract `pageViewportToScreen(rect, page, layout)` (shared, pure — likely `src/shared/page-space.ts` or a widened `annotationMath.ts` if we accept above-view-only scope; prefer shared: main needs it too when scroll tracking lands).
- Replace the four copies: `elementAnnotationRect`, `pendingElementScreenRect`, `useAnnotationDraftState.ts:205`, and the divergent `annotationScreenPos` element branch — the divergence (insets + clamping) becomes explicit post-processing at that one call site, not a fork of the transform.
- Opportunistic: fold the duplicated `badgeOpacity` content-band math into the same module as the second visibility mode ADR 0029 discussion identified (point-fade vs rect-clip), if it stays under ~30 lines; otherwise leave.

**Tests**
- Unit: direct tests on `pageViewportToScreen` (contentScreen fallback to screen bounds, scale, canvasOrigin). Existing `annotation-live-bbox.test.ts` position expectations act as regression pins — they must not change. Mutation: break the scale term → both suites fail.

### Phase 4 — Annotations adopt `PageAnchor`

The heaviest phase; schedule when annotation persistence is next open, not before phases 1–3.

**Change**
- `Annotation` gains `pageAnchor?: PageAnchor`, written at creation by `enrichedAnnotationMetadata`'s successor (also the moment to split that function: URL/name context enrichment vs React-component enrichment are unrelated concerns fused today, `workspace-annotations.ts:47-98`).
- Read path prefers `annotation.pageAnchor`, falls back to legacy `metadata.pageUrl` + `annotationContextPageId` — old `.canvas` files keep working unmodified (same transparent-extension posture as `Annotation.kind` retirement in ADR 0006).
- Collapses: `annotationContextPageId` vs `anchoredPageIdFor` → one accessor; `sidebarAnnotationsForPage` + `sidebarAnchoredItemsForPage` → one "content belonging to a page" builder over a common item shape (annotation rows keep their thread-count/label projection; the `SidebarAnnotationItem` / `SidebarAnchoredEntityItem` split narrows to presentation).
- Decide and record: do we keep writing `metadata.pageUrl` for one release for downgrade tolerance? (Recommend yes, then drop.)

**Tests**
- Integration: legacy fixture (`metadata.pageUrl`, no `pageAnchor`) hides on navigation and nests in sidebar identically to a new-format annotation; round-trip write emits `pageAnchor`. Mutation: drop the legacy fallback → fixture test fails.
- `tests/unit/workspace-annotations.test.ts` and `annotation-url-visibility.test.ts` updated to the unified accessor.

**Docs:** amend ADR 0029 (annotations now consume the utility; the "annotations predate this" caveat in `page-anchor.ts` header goes away) and CONTEXT.md §Annotations/§Page anchoring.

### Phase 5 (triggered, not scheduled) — Anchorable opt-in via the entity-kind registry

When a third kind wants anchoring: add an `anchorable` capability to `EntityKindDefinition` so `anchorableEntities()`/`findAnchorableEntity` derive from the registry instead of hand-listed arrays, and the create-command reanchor hook applies per capability rather than per hand-edited wrapper. The 4-way persisted-field-list mirroring per kind is real friction but belongs to a broader ADR 0024 follow-up, not this plan. One adapter is a hypothetical seam — today's two kinds work; a third makes it real.

## Cross-cutting side quests (do opportunistically, in whichever phase touches the file)

- Rename `FocusPresentationData.annotationsVisible` → `contextVisible` (or similar): the eye now governs pages, files, groups *and* annotations ("Show/Hide other items"), and the stale name misleads — it made this refactor's blast-radius analysis harder than it needed to be. Update ADR 0021 references and CONTEXT.md when done.
- The dashed-outline style triple (`comment-hover-overlay.ts:82`, `CommentBadgesLayer.tsx:60`, `CommentsLayer.tsx:202`) — extract a constant if phase 1/2 touches all three anyway; don't force it.
- `sidebar-builder.ts:167` reimplements `sortSidebarItems` inline — use the helper (falls out of phase 4's builder merge).

## Sequencing vs. scroll tracking

Phases 1–3 are the prerequisite for scroll tracking, in order: the absolute scroll-offset broadcast and `docX/docY` anchor variant should be built against **one** transform (phase 3) and **one** visibility gate (phase 2), not four of each. Phase 4 is independent of scroll tracking but shrinks the surface it touches. Do not start scroll tracking mid-plan.

## Risks

| Risk | Mitigation |
|---|---|
| Phase 1 deletes something a page-side flow silently uses | Grep-proven zero references before delete; manual pass over comment-tool hover, inspect, and gesture-forwarding; `pnpm test:boot` before release as usual |
| Phase 2 hides annotations from a consumer that needed the full record | Verified: panel uses `inspect-session` payload; only other `layoutData.annotations` consumers are the four above-view layers + a dev debug count |
| Phase 2 changes thread-open-from-panel UX | It doesn't (open→instant auto-close today ≡ never-opens after); noted in the phase so the reviewer checks it deliberately |
| Phase 3 accidentally "fixes" the divergent inset math and moves popovers | Keep the divergence as explicit post-processing; position expectations in existing unit tests pin it |
| Phase 4 breaks old `.canvas` files | Legacy-fixture integration test is written first (red/green), fallback read path mandatory |

## Acceptance criteria

- `matchesPageUrl` has exactly one wrapper per concern: the main-side gate (broadcast filtering), the sidebar dim, and the annotation-metadata legacy read (until phase 4 retires it). No renderer re-derives document-binding visibility.
- `rg 'PAGE_COMMENT_BADGES_ENABLED|pageAnnotationsUpdate|__canvas-comment-badges'` returns nothing.
- One `pageViewportToScreen` definition; `rg 'contentScreenWidth / page.width'` (and variants) hits only it.
- After phase 4: one page-binding accessor, one sidebar child-row builder, `metadata.pageUrl` written only by the compat shim (then deleted).
- Every phase: `pnpm typecheck`, `test:unit`, `test:integration` green; new tests mutation-verified per `tests/README.md`, with the mutation named in the test header and commit message.
- Net LOC for phases 1–3 is negative.
