# Deepen the above-view interaction layer — one pointer authority + testable gestures

Track 3 of 3 from the deepen pass (`docs/audit/deepen-3673a35.md`, candidates
5–6). Self-contained: a fresh agent can build from this doc alone. Feature
branch off `main`; one PR per step; integration PR at the end.

## Goal

Restore "input has one authority" inside the renderer: the pointer router
becomes the single owner of canvas pointerdowns, App.tsx sheds its inline
gesture and forwarding effects, and the bug-bearing gesture decisions
(threshold promotion, commit/cancel, ordering) move behind a pure,
unit-testable seam.

## Ground truth (measured at 3673a35)

- `src/renderer/above-view/App.tsx` = 1,579 lines;
  `useCanvasPointerRouter.ts` = 1,068.
- Three parallel pointer-input adapters: the router
  (`useCanvasPointerRouter`), `useAnnotationDrawingGestures` (React handlers
  on the root div, App.tsx:1291-1294), and a hand-rolled window-capture
  placement/comment gesture inline at App.tsx:907-1044 (threshold check,
  `startPointerSession`, `squareConstrainedRect`, dispatch to
  `api.placePendingShape` / `api.commitCommentClickAt`).
- Arbitrated by five overlapping booleans: `routerOwnsCanvasPointers`,
  `toolGestureOwnsCanvasPointers`, `overlayInteractive`, `skipPointerCapture`,
  `commentToolBlocked` (App.tsx:667-673, 883-906).
- Page-input forwarding is triplicated: `hitTestHoverTarget` (App.tsx:807-826),
  hover-forward effect (834-881), wheel routing (`routeWheel`, 1058-1135),
  cursor-style mirror (1141-1145), no-button pointer-forward (1154-1196). The
  page content-bounds check (`contentScreenX ?? screenX` chain) is
  verbatim-identical at 1077-1082 and 1173-1178.
- The 8-popup mount mux (App.tsx:1486-1566) threads near-identical props
  (`api`, `isDark`, `layout`, selection, `interactionIdle`) into
  TextToolPopup ×2, ShapeToolPopup, DrawToolPopup, StickyNotePopover,
  GroupPopup, ShapePopup, DrawingPopup, FilePopup, PagePopup, plus 6 parallel
  `selectedXxxEntities` memos (443-461) and `sameKindSelectedEntities`
  (375-388).
- Already-deep seams to imitate, not refactor: `shared/hit-test.ts` (tested),
  `canvas-pointer-actions.ts` (tested classification), `pointer-session.ts`
  (listener/capture/teardown mechanics), `edge-drag-controller.ts`,
  `resize-accumulator.ts`, `useAnchoredPosition`.

## Constraints (load-bearing — read before coding)

- `docs/interaction-layer.md` §6 invariants, especially I2/I3 (one
  interaction token, begin/commit/cancel pairing enforced main-side), I8
  (pointer events only in renderer gesture code), I9 (coord math from
  `src/shared/coords.ts` / hit-test single-sourced).
- §4.6 per-handler divergences are intentional: `runForwardPointer`
  deliberately installs no blur listener; `runResize` calls `beginResize`
  BEFORE the first patch (focus-reconciler ordering); press-vs-drag
  `ignoreBlur: () => !dragging` phantom-blur guards. Preserve all of them —
  the refactor relocates decisions, it does not change them.
- The main/shared/renderer split for interaction modes is by design (main
  owns interaction state). Do not move mode state into the renderer.
- `CanvasItemPopup`'s ADR 0008 compound (positioning/animation) is deep —
  don't reimplement it; the popup step only tabulates the *mount* logic.

## Steps (one PR each)

### 1. Shared page hit-test + one forwarding hook

Extract `pointerOverPageContent(layout, {x,y}): { pageId } | null` into
`src/shared/` beside `hitTest` (I9), replacing the three inline variants.
Extract `usePageInputForwarding(api, layoutRef)` owning cursor mirror +
hover-forward + no-button forward + the wheel page-body check; App.tsx drops
those effects (~120 lines).

### 2. Placement + comment gestures move behind the router

Model them as `CanvasPointerAction` kinds with their own `run*` handlers —
peers of `runEntityDrag` (this is the documented shape,
`interaction-layer.md` §4.6). Delete the inline App.tsx:907-1044 handler.
Collapse the five arbitration booleans into one pure
`canvasPointerOwner(state)` selector with unit tests enumerating the
ownership matrix (router / annotation-gesture / none, per tool × focus ×
overlay state).

### 3. Popup mount table

A kind-keyed registry `{ kind → { selector, Component } }` drives one `.map`
for the 8 selection/tool popups; the `toolHasPopup`-vs-selection mutex and
the `interactionIdle`/150 ms-delay gating encode once. The 6
`selectedXxxEntities` memos collapse into the selector column. ~80 lines of
JSX out of App.tsx.

### 4. Pure gesture-decision reducer

Extract the decision logic out of the `run*` window listeners into pure
functions (e.g. `pressGestureStep(state, ev) →
{ promoteToDrag } | { commitEdit } | { ignore }`, and an outcome mapper for
resize/edge-drag commit-vs-cancel). The `run*` shells keep only IPC calls and
listener wiring via `startPointerSession`. Unit tests cover: press-vs-drag
threshold promotion, phantom-blur guard behavior, begin-before-patch resize
ordering, edge-drag commit/cancel outcomes — the documented bug class in
`interaction-layer.md` §9.

## Verification

Every PR: `pnpm typecheck` + `pnpm test:unit`. Steps 1–2 change input
routing: run `pnpm test:smoke` and manually exercise in the app — entity
drag, press-to-edit on sticky/shape, shape placement (incl. shift-square),
comment click + region drag, annotation pen drawing, hover/scroll forwarding
into a focused page, resize with focus handoff, edge drag commit and cancel.
