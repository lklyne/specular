# Plan — camera-local projection and the runtime-store deletion pass

Status: proposed. Stacks on `perf/frame-clock-b` (PR for `989d8656`), which itself stacks on [PR #404](https://github.com/lklyne/specular/pull/404) (ADR 0036). Read that PR's "Next steps" section and `docs/plans/diffed-runtime-store.md` first; this plan is the follow-up it describes, plus the fix for the zoom regression that `989d8656` exposed.

Author handoff: this is a from-scratch spec. Read `src/main/runtime/CLAUDE.md` (two-layer state model), `docs/adr/0036-diffed-runtime-store.md` (the store, the bus, the snapshot invariant in §3), and `docs/interaction-layer.md` §6 before touching code.

## Why this exists

### The regression

On `perf/frame-clock-b`, pan is smooth and zoom is not: the native page views track a pinch exactly while every DOM element around them (selection rings, handles, group titles, edges, sticky bodies, the grid) lags a few frames behind.

The two gestures take different paths through `setViewportCamera` (`src/main/runtime/viewport-control.ts`):

- **Pan** changes the camera, calls `layoutAllViews()` (native `setBounds` per page), and sends one ~40-byte `viewportNudge`. The renderers apply the nudge as a CSS `translate3d` over an unchanged scene (`useSceneCameraTransform`, `src/renderer/shared/hooks/useScenePanOffset.ts`). Nothing is rebuilt.
- **Zoom** does all of the above and also `markDirty('toolbar', 'canvas')` (`viewport-control.ts:111`), so the same `layoutAllViews()` call runs `buildCanvasLayoutData()` and `broadcastSceneUpdate()` (`layout-engine.ts:349–354`, `:587`): a full scene rebuild, diff, and patch fan-out, once per wheel event.

`989d8656` deleted the 16ms input bucket and the trailing `requestLayout()` (the `setImmediate` coalescer). Those two were the only things bounding zoom's rebuild rate to roughly one per frame. With them gone, a trackpad pinch delivering several wheel events per frame produces several full scene rebuilds per frame. The page views are positioned natively in main and keep up; the DOM chrome waits on a backlog of scene payloads and paints a stale `layoutData.zoom`.

Restoring the coalescer would mask this. It would not remove the reason zoom needs a rebuild at all.

### The reason zoom rebuilds

The comment at `viewport-control.ts:106–110` says it: screen-constant chrome (handles, popups, outlines) is sized against `layoutData.zoom`, and the CSS scene transform alone would scale it with the scene. That is true but it is the symptom. The underlying cause is that **the scene is projected in main**. Every `CanvasSceneEntity` carries `screenX / screenY / screenWidth / screenHeight` (`src/shared/types.ts:128–135` and the six other entity kinds), computed from `canvasX * zoom + pan` inside `buildCanvasLayoutData`. The renderers never project; they place DOM at the screen coordinates main hands them.

So a zoom, which changes no entity, invalidates every entity's screen geometry. That is why `markDirty('canvas')` is on the zoom path, why PR #404 measured ~130KB/s of entity patches during drag (item 4 of its follow-up: "screen-geometry churn on every entity per pass"), and why `viewportNudge` had to be carved out as a special channel with a reconcile step. Pan only gets away with it because translation commutes with the CSS transform; scale does not.

### The end state

**Main owns canvas-space geometry and the camera. Renderers project.**

- Scene entities carry canvas-space geometry only (`canvasX`, `canvasY`, `width`, `height`, plus the device-shell inset where it applies). No `screen*` fields.
- The `camera` slice (`zoom`, `pan`, `cameraTransitionStartedAt`, already in `src/shared/runtime-store.ts:21`) is the single source of the projection. A camera change is a `camera` slice patch and nothing else.
- Each renderer projects locally with one pure helper, `projectToScreen(entity, camera, sceneOrigin)`, and sizes screen-constant chrome with `1 / camera.zoom` (the pattern `GroupBoundsLayer.tsx:32` and `AgentCursorLayer.tsx:111` already use).
- `viewportNudge` is deleted. ADR 0036 §6 kept it as the one honest exception because "a pan is a camera transform over an unchanged scene." Once renderers project, a camera patch *is* that transform. The exception no longer needs its own channel or its own reconcile logic; `useSceneCameraTransform` and `cameraAfterSceneTransform` (`src/shared/scene-camera-transform.ts`) go with it.
- Zoom and pan become the same cost: `setViewportCamera` → `layoutAllViews()` for the native views → one `camera` patch. No rebuild, no dirty flag, no coalescer needed because there is nothing left to coalesce.

The native page views still get `setBounds` from main on every camera tick, as today. That is the one projection main keeps, because Chromium positions a `WebContentsView` in window pixels and nothing else can.

## Current-state code map

Main, the camera path:
- `src/main/runtime/viewport-control.ts` — `setViewportCamera` (line ~85). The `markDirty` at 111 and the comment above it are what this plan removes. `markZoomMotion` / `markPanMotion` settle callbacks (118–130) stay; the settle re-layout re-captures frozen page rasters and re-baselines, and is unrelated to chrome.
- `src/main/runtime/viewport-input.ts` — `applyViewportInputDelta`; unchanged by this plan.
- `src/main/runtime/viewport-nudge.ts` — `broadcastViewportNudge`; deleted in phase 2.
- `src/main/runtime/layout-engine.ts` — `layoutAllViews()` (line ~328). The `consumeDirty('canvas')` block (349) is where the rebuild happens. `requestLayout()` (727) runs the pass on `setImmediate`.
- `src/main/runtime/canvas-layout-data.ts` — `buildCanvasLayoutData()`. This is where `screen*` is computed; search for `* zoom` and `pan.x`.
- `src/main/runtime/runtime-patch-broadcast.ts` — the bus, `broadcastSceneUpdate`, `broadcastRuntimePatch`, the 1s snapshot (`SNAPSHOT_INTERVAL_MS`, line 42), `recordWireBytes` (TEMP).
- `src/main/runtime/layout-dirty.ts` — three-bucket dirty set.

Main, the 82 `markDirty('canvas')` sites (count at time of writing; `grep -rn "markDirty('canvas'" src/main`). Ranked by file:
- `interaction-state.ts` 13 — ephemeral state only; the main target of phase 3.
- `page-factory.ts` 5, `window-init.ts` 4, `page-anchor-state.ts` 4, `entity-order-state.ts` 4, `ui-state.ts` 4 — mixed.
- `viewport-control.ts` 3 — one is the zoom site above; the other two are settle/fit paths.
- `*-entity-state.ts` (text, shape, group, file, drawing) 3 each — geometry mutators; these keep the pass.
- `tool-mode.ts` 2, `document-commands.ts` 3, `managed-layout.ts` 3, the IPC registrars, and the singletons.

Renderers, the projection consumers (32 `.zoom` reads across these files):
- `src/renderer/canvas-bg/App.tsx:40–47` — `useSceneCameraTransform` + `cameraAfterSceneTransform`; the scene container transform.
- `src/renderer/above-view/App.tsx:382`, `:1086` — same pair; the `translate3d … scale()` container.
- `src/renderer/canvas-bg/canvasBgSelectors.ts`, `CanvasGridSurface.tsx:169`, `AgentCursorLayer.tsx:111`.
- `src/renderer/above-view/GroupBoundsLayer.tsx:32`, `DrawingsLayer.tsx`, `ShapeBodyLayer.tsx`, `StickyBodyLayer.tsx`, `EdgeDragLayer.tsx`, `EdgePopup.tsx`, `FocusedNoteLayer.tsx`, `useCanvasPointerRouter.ts`, `optionDragCopy.ts`, `contentHeightPreview.ts`.
- `src/renderer/shared/runtime-store.ts` — `useSlice`, the renderer store.
- Every layer that reads `entity.screenX` etc. — `grep -rn "screenX\|screenWidth" src/renderer` before phase 2; expect ~60 sites.

Shared:
- `src/shared/types.ts` — `CanvasSceneEntity` variants (lines ~119–400), `LayoutUpdateData` (~557), `ViewportNudge`.
- `src/shared/runtime-store.ts` — slice definitions.
- `src/shared/scene-camera-transform.ts` — deleted in phase 2.
- `src/shared/ipc-contract.ts` — `viewportNudge` channel, deleted in phase 2.

TEMP instruments (from PR #404, each tagged `// TEMP instrument (plan: diffed-runtime-store)`):
- `src/renderer/above-view/ipc-tally.ts` + wiring in `above-view/App.tsx`
- the `requestLayout` cause histogram in `layout-engine.ts` (`recordLayoutCause`, ~696)
- `recordWireBytes` in `runtime-patch-broadcast.ts`
- `src/main/alloc-profile.ts` + its perf route

## Build sequence

One PR per phase into `perf/camera-local-projection`, then one integration PR into `main` once `perf/frame-clock-b` has merged. Per-phase gate: `pnpm typecheck` + `pnpm test:unit` + `pnpm test:integration`. One manual smoke per phase branch before merge. Phases 1 and 2 are the fix; 3–5 are PR #404's follow-up list, reordered so the deletion happens after the projection move (which is what makes most of the `canvas` sites unnecessary in the first place).

### Phase 0 — measure, and stop the bleeding on this branch

Small, and it gives the baseline the rest is judged against.

1. Run the pan-zoom perf test (`src/main/pan-zoom-perf-test.ts`, View menu) on `perf/frame-clock-b` and record `buildStats` (n, mean, p95) for the zoom phases. `n` is the number of `buildCanvasLayoutData` calls during the gesture; it should be close to the number of wheel events. Record `requestLayout-causes` and `runtime-wire-bytes` from `errors.log` for the same run.
2. Until phase 2 lands, bound the damage: in `setViewportCamera`, keep `markDirty('canvas')` on zoom but coalesce it to one pass per `setImmediate` turn (`requestLayout()`), the way `989d8656`'s parent did, while leaving the native `layoutAllViews()` synchronous. This is a two-line change; it keeps the pages on-arrival and returns chrome to roughly one rebuild per frame. It is a stopgap and is deleted in phase 2. Do not spend time tuning it.

Acceptance: zoom `buildStats.n` drops to about one per frame; chrome no longer visibly trails pages during a pinch on a 120Hz display.

### Phase 1 — renderers project from the camera slice

Add the projection helper and make every renderer layer compute screen geometry from canvas geometry + the `camera` slice, while main *still sends* `screen*` fields. The two must agree; this phase proves they do before anything is deleted.

1. `src/shared/scene-projection.ts`: `projectToScreen(rect: CanvasRect, camera: {zoom, pan}, sceneOrigin: {x, y}): ScreenRect` and `unprojectFromScreen`. Pure, unit-tested against the formulas in `buildCanvasLayoutData`. Include the device-shell inset case (`contentScreen*` on page entities).
2. A `useProjectedEntity(id)` hook (or a selector in `canvasBgSelectors.ts` / the above-view equivalent) that reads the entity from the store and the `camera` slice and returns screen geometry. Memoize per entity on `(entity, camera)` identity so a camera patch re-projects without re-rendering layers whose output does not depend on the camera.
3. Migrate layers from `entity.screenX` to the hook, one layer per commit. Screen-constant chrome reads `camera.zoom` directly for its `1 / zoom` counter-scale.
4. Dev-only drift check: while `screen*` fields still arrive from main, assert `projectToScreen(entity, camera) ≈ entity.screen*` within 0.5px under the drift-watchdog flag. Any line in `errors.log` is a projection bug.

The scene container transform (`useSceneCameraTransform`) stays in this phase; nudges still arrive and still reconcile to identity when the next layout lands. The point is that after phase 1, a layer's position no longer depends on which of the two the store last received.

Acceptance: zero projection drift lines over a stress session (pan, zoom, drag, resize, device shells on, group titles, edges, sticky notes). No visible change.

### Phase 2 — camera is a patch; delete the nudge and the screen fields

The fix proper.

1. `setViewportCamera`: replace `markDirty('toolbar', 'canvas')` + `broadcastViewportNudge()` with `broadcastRuntimePatch({ slice: 'camera', ... })`. Keep `layoutAllViews()` synchronous for the native views, but the pass must no longer rebuild the scene on a camera-only change: since `canvas` is not dirtied, `consumeDirty('canvas')` is false and the block at `layout-engine.ts:349` is skipped. Keep `markDirty('toolbar')` on zoom change for the zoom percentage readout, or better, have the toolbar read the `camera` slice too and drop the flag.
2. Delete `src/main/runtime/viewport-nudge.ts`, the `viewportNudge` channel, `ViewportNudge` type, `useSceneCameraTransform`, `cameraAfterSceneTransform`, `src/shared/scene-camera-transform.ts`, and the container `translate3d … scale()` in both renderers. The scene container is now at the origin; every layer is positioned by projection.
3. Delete `screenX / screenY / screenWidth / screenHeight` (and `contentScreen*`) from every `CanvasSceneEntity` variant and from `buildCanvasLayoutData`. The entity patch for a drag shrinks to canvas-space deltas. `layout-structural-share.ts` gets simpler because nothing under `entities` changes on a camera tick.
4. Delete the phase-0 stopgap.
5. Update `docs/adr/0036-diffed-runtime-store.md` §6: the exception is gone and why. Update `CONTEXT.md` entries for *viewport nudge* and *scene entity*. Update the comment block in `viewport-control.ts`.

Things that read screen geometry in main and must keep doing so (they are not renderers): native `setBounds` for page/component views in `layoutAllViews`, the above-view gate bounds, `setSelectionOverlayRect`, the cursor overlay window, agent screenshot crop rects, HTTP-API responses that report screen rects. Give them a main-side `projectToScreen` call against the same helper; do not keep a second formula.

Acceptance: `buildStats.n` during zoom phases is 0 (no scene builds during a pure camera gesture). Wire bytes per zoom tick is the size of one `camera` patch. Chrome and pages move in the same frame, by construction. Drift watchdog clean. `pnpm test:integration` `viewport-input.test.ts` extended to assert no `canvas` dirty on camera change.

### Phase 3 — route ephemeral mutators to slice patches (PR #404 items 1–2)

With projection local, most `markDirty('canvas')` sites exist only to ship a changed cell, not changed geometry.

1. Rank the 82 sites from the cause histogram of a real session (PR #404's run 3 already has: `commitSelection` 29, `setSelectionOverlayRect` 18, `clearInteractionState` 14, `beginDraggingEntities` 11).
2. Migrate `interaction-state.ts` (13), `tool-mode.ts`, `ui-state.ts`, and the selection mutators to `broadcastRuntimePatch` slice patches. Rule: a mutator keeps `markDirty('canvas') + requestLayout()` if and only if it changes entity canvas geometry, entity membership, or z-order. Everything else is a patch.
3. The risk is a mutator that changes a cell *and* geometry (e.g. `beginDraggingEntities` both marks the interaction and may reorder). The drift watchdog catches it as "UI wrong for up to 1s, then the snapshot fixes it." Keep the watchdog on for the whole phase.

Acceptance: `requestLayout-causes` over a stress session shows only geometry mutators; selection/hover/tool changes produce zero passes.

### Phase 4 — shrink `buildCanvasLayoutData` (PR #404 item 3)

After phases 2 and 3 the builder is producing canvas-space geometry plus fields that only main can read (`isLoading`, `canGoBack`, page titles, device presets). Split it:

1. Geometry and membership → the `scene` slice + `entities` map, rebuilt only on geometry passes.
2. `webContents`-read fields → a per-page `pageChrome` patch emitted from the page lifecycle hooks that already know when they change (`did-navigate`, `page-title-updated`, load state), not from the pass.
3. The `canvas` bucket in `layout-dirty.ts` narrows to "geometry pass needed." The file stays; it still gates `sidebar` and `toolbar`.

Acceptance: a snapshot at rest is smaller than PR #404's 22KB; a geometry pass's wall time (`buildMs`) drops measurably; no `layoutUpdate` field is computed that no renderer reads.

### Phase 5 — delete instruments and cleanups (PR #404 items 5–6)

1. Delete `ipc-tally.ts` and its App wiring, `alloc-profile.ts` and its perf route.
2. Keep `recordLayoutCause` and `recordWireBytes` as permanent dev-only diagnostics, gated on the drift-watchdog flag. Drop the `TEMP instrument` tags and write their "why" comments as permanent.
3. `runtime-context.ts`: delete the write-only `selectionOverlayActive`, `spaceModifierHeld`, `hoveringCanvasChrome` and their setters, or wire them to a reader if one was intended.
4. `snapshotToStore`: stop sending `selectedGroupId` to agent-layer.
5. Re-run phase 0's measurement and record before/after in the ADR.

## What stays out

- The settle pipeline (`zoom-snapshot-freeze.ts`, `settleZoomGesture`, hi-res re-capture) is untouched. It is about page rasters, not chrome, and it already runs on settle rather than per tick.
- `requestLayout()` stays on `setImmediate`. Phase 2 makes the camera path not call it; it remains the coalescer for geometry mutations.
- Drag freeze and the chrome canvas (PRs 400/401) are consumers of projection like any other layer; they migrate in phase 1 and need no special handling.

## Risks

- **Two formulas disagree** during phase 1. Mitigated by the drift assertion; do not skip it.
- **A main-side reader of `screen*` is missed** in phase 2 and silently reads `undefined`. Typecheck catches field reads; the risk is a `JSON`-shaped consumer (HTTP routes, agent snapshot). `grep -rn "screenX" src/main src/shared` before deleting the fields and give every hit a projection call.
- **Phase 3 migrates a geometry mutator to a patch.** Symptom and detection are as above; the fix is always "give it back its `requestLayout()`", never "send more in the patch."
- **Device-shell pages** have two rects (outer shell, inner content). Both are canvas-space with a fixed-pixel inset that is screen-constant, so the projection is `outer = project(rect)`, `inner = outer inset by shellInset / zoom`. Get this right in the helper's tests before phase 1.3.
