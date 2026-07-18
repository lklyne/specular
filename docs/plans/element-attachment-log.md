# Element attachment — build log (deviations from plan)

Orchestrated build of docs/plans/element-attachment.md. Terse notes only;
empty sections mean the step landed as planned.

## Step 1 — anchor shape + capture rule

## Step 2 — capture wiring

- Re-capture also fires on keyboard nudge (routes through `reanchorEntityById`),
  not just creation/drag-end. Correct behavior, slightly broader than the plan's
  wording; token-guarded.
- No undo-triggered re-capture: `syncDocToRuntime` never calls
  `reanchorEntityById`, so after undo the pre-undo `element` persists until the
  next placement re-captures. Plan's "re-capture re-derives on undo" is not
  wired; acceptable (attachment is never a visibility gate).
- Per-entity capture-token Map is never pruned (bounded, self-correcting);
  left unwired from `resetDocSync` to avoid an import cycle.

## Step 3 — reflow pipeline

- Tracker built as a focused sibling module (`element-position-tracker.ts`)
  instead of growing `annotation-bbox-tracker.ts`; bbox tracker untouched.
  Same discipline, two channels: `element-attachment-subscriptions` (main→page,
  `{selectors}`) and `element-attachment-positions` (page→main, batched).
- Subscription recompute rides `mutateWorkspace` (the single mutation seam)
  rather than the layout path — `layoutAllViews` early-returns in the harness.
  Stamps and page load/navigation refresh outside the seam by design.

## Step 4 — render correction + fold

- Shared formula lives in new `src/shared/element-attachment.ts` (not
  `page-space.ts` — distinct concern); renderer gets live positions via
  `CanvasScenePageEntity.elementPositions` on the layout broadcast.
- `rebaseAnchorScroll` no longer early-returns for frame-pinned anchors: it
  folds the element term for them too, leaving scroll refs untouched
  (`pageAnchorScrollShift` already zeroes for frame-pinned).

## Manual verification needed

Step 5's manual smoke (per plan) covers all of these; none are covered by
automated tests:

- End-to-end reflow on a live page: annotate + draw + sticky, switch device
  preset → fill in focus mode, resize — confirm ink/regions/stickies track
  the content. Exercises the real MutationObserver debounce and rAF flush
  timing (unit tests use a hand-rolled fake DOM).
- Renderer region paths (`AnnotationsLayer.tsx`, `annotationMath.ts`) — only
  typechecked; main-side `regionCanvasRect` is the integration-tested twin.
- Kill the dev server mid-session: items hold last geometry, nothing hides.
- Undo-then-reflow edge: after undo, the pre-undo `element` reference persists
  until the next placement re-captures (no undo-triggered re-capture wired).
