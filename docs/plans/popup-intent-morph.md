# Plan — Model-driven (intent) popup morph

Make the page-toolbar popup morph between its page-anchored placement and the
viewport-pinned focus bar with **zero React re-renders in the popup subtree** and
**no `getBoundingClientRect` for positioning** — driven by the camera *intent*
(target + curve) broadcast once, not the stepped per-frame broadcast.

Read `docs/canvas-motion-research.md` first — this plan implements its §4 / Step 0.
Especially read **§7c**: a prior spike applied this same technique to the canvas
*scene* (grid/page/border) and it was a regression, because smoothing an overlay
that *borders a native page* makes the border detach from the still-stepping page.
**The popup is the one exempt case** — nothing native borders it — which is why we
apply intent here and nowhere else.

---

## Non-goals / guardrails (do not violate)

- **Touch only the popup + an additive main-side broadcast.** Do **not** change the
  canvas scene, `useCanvasLayoutState`, native page `setBounds`, or any
  page-bordering overlay. That path is the §7c regression; it is out of scope.
- Other popups (multi-select, non-focus placements) keep their current behavior.
  Only the `viewport-top` ↔ page-anchored morph goes on the intent path.
- The main-side change is purely additive (a new broadcast field + set/clear). If
  the renderer ignored it, behavior is unchanged.

---

## The core insight (why one WAAPI tween is exact)

During an animated camera move, `pan(t)` and `zoom(t)` interpolate by the **same**
spring progress `e = cameraSpring(t)` (see `src/shared/camera-transition.ts`). For a
fixed world point `P`, its screen position is:

```
screen(t) = P·zoom(t) + pan(t)
          = P·lerp(z0,z1,e) + lerp(p0,p1,e)
          = lerp(P·z0+p0, P·z1+p1, e)
          = lerp(screen0, screen1, e)
```

So the anchor's screen path is a **straight line parameterized by `e`**. A single
WAAPI keyframe tween `[start → end]` with `easing: CAMERA_SPRING_CSS_EASING`
reproduces the camera's motion **exactly** — no per-frame projection, just the two
endpoints. The popup width (`minWidth: rect.width = worldWidth·zoom`) also lerps by
`e`, so it rides the same tween. Constant-px chrome offsets (`CHROME_HEADER_HEIGHT`)
are camera-independent and identical at both endpoints.

**Caveat:** `popupStyle`'s edge-clamp (`Math.max/min` on `left`) is non-linear. For
the common focus case the popup isn't at a clamp boundary; clamp *both* endpoints and
lerp between them (good enough). Note it; don't over-engineer.

---

## Why the close jitters today (the specific bug)

`usePopupFlipAnimation` (in `CanvasItemPopup.tsx`) runs a FLIP: outer wrapper carries
the resting position (`popupStyle` → transform, recomputed from the **stepped
broadcast rect** every render), inner `motionRef` runs a WAAPI translate to zero.

- **Open** (→`viewport-top`): the end is the viewport bar — a **fixed** chrome-derived
  rect. Stable ⇒ smooth. (Leave this path essentially as-is.)
- **Close** (`viewport-top`→anchored): the end is the popup anchored to the page at
  the **restored** camera. But `nextRect` is measured (`getBoundingClientRect`) at
  close-start, when the camera is still zoomed *in* — so it's the wrong endpoint. The
  outer wrapper then **steps** toward the true final position as the broadcast
  restores frame-by-frame ⇒ jitter, worst at landing.

Fix: compute the close endpoint **analytically from the known target camera**, freeze
the outer wrapper there for the morph, and let the inner WAAPI slide in. The baseline
is now correct *and* stable ⇒ no steps, and no `getBoundingClientRect` for position.

---

## Design

Keep the two-element structure (outer wrapper positions, inner WAAPI slides). Two
changes:

1. **Outer wrapper holds the analytically-projected target during the morph**
   (frozen — ignores the stepped broadcast rect for the morph duration).
2. **The FLIP endpoint is the projected target rect**, not a `getBoundingClientRect`.

The freeze is *local to this popup* (a ref holding the target position for the morph
window) — not a global/scene freeze. Safe.

---

## Changes, file by file

### Main — additive camera-intent broadcast (safe)

Model on the existing `cameraTransitionStartedAt` plumbing (already broadcast + used
for phase-sync); add the destination + duration alongside it.

- **`src/shared/types.ts`** — add:
  ```ts
  export interface CameraTransitionIntent {
    target: { pan: { x: number; y: number }; zoom: number }
    startedAt: number      // Date.now() wall clock, shared main↔renderer
    durationMs: number
  }
  ```
  and `cameraTransition: CameraTransitionIntent | null` on `LayoutUpdateData`.
  (`start` isn't needed — the morph's start endpoint is the popup's current position.)

- **`src/main/runtime/runtime-context.ts`** — `export let activeCameraTransition:
  CameraTransitionIntent | null = null` + `setActiveCameraTransition(v)`.

- **`src/main/runtime/viewport-control.ts`**, `moveCameraTo` animate branch: after
  `setCameraTransitionStartedAt(start)`, call `setActiveCameraTransition({ target:
  {pan: target.pan, zoom: target.zoom}, startedAt: start, durationMs: duration })`,
  then `requestLayout()` **once** so the exact start frame + intent broadcast
  immediately (before the first stepped tick). In `cancelCameraAnimation`, also
  `setActiveCameraTransition(null)`.

- **`src/main/runtime/canvas-layout-data.ts`** — import `activeCameraTransition`; add
  `cameraTransition: activeCameraTransition` to the built payload. (The payload
  already reaches aboveView via `sendAnnotationLayoutUpdate` → the popup's `layout`.)

- **`src/renderer/canvas-bg/canvasBgConstants.ts`** `EMPTY_LAYOUT` (and any other
  `LayoutUpdateData` literal — typecheck will list them) — add `cameraTransition:
  null`.

### Renderer — `src/renderer/above-view/CanvasItemPopup.tsx` (+ anchored helper)

- **Add an at-camera projection** (in `useAnchoredPosition.ts`): a variant of
  `anchoredSlotRect` that projects the entity's **world** rect through an explicit
  camera instead of reading the baked `entity.screenX`. The scene entity carries
  world coords (`canvasX/canvasY/width/height`); project with `coords.ts`
  (`x·zoom + pan.x + origin.x`, etc.), then feed `entityChromeSlots` + `toOverlayLocal`
  exactly as today. Signature e.g.
  `anchoredSlotRectAtCamera(layout, entityId, slot, camera)`.

- **Replace `usePopupFlipAnimation`'s endpoint** for the morph:
  - Detect the morph as today (placement change to/from `viewport-top`).
  - `pos0` = the popup's current position (its live `popupStyle` output at morph
    start — capture once).
  - `pos1`:
    - open (→`viewport-top`): the viewport-bar rect (fixed; as today).
    - close (→anchored): `popupStyle(anchoredSlotRectAtCamera(layout, entityId, slot,
      layout.cameraTransition.target), placement, …)` — the resting position at the
      **restored** camera.
  - During the morph, render the **outer wrapper at `pos1`** (frozen via a ref) so the
    stepped broadcast can't move it; run the **inner WAAPI** from `pos0−pos1` delta →
    0 on `CAMERA_SPRING_CSS_EASING`, `duration = cameraTransition.durationMs`,
    phase-synced (`currentTime = clamp(Date.now() − startedAt, 0, duration)`). Width
    tween for `viewport-top` stays.
  - On finish (or when `cameraTransition` goes null — interrupt), unfreeze: outer
    wrapper returns to the live broadcast rect (== target). Seamless, since frozen
    `pos1` already equals the settled broadcast position.

- **Delete** the re-measure-every-render effect and the `getBoundingClientRect`
  endpoint reads for *position* (the handoff's flagged debt). Measuring `frameSize`
  (width/height for clamp/centering) via `ResizeObserver` may stay — it's not
  per-frame and not on the position hot path.

---

## Edge cases

- **Interrupt** (user pans mid-morph): main `cancelCameraAnimation` clears intent →
  `cameraTransition` null next payload → cancel the WAAPI, unfreeze, settle to
  broadcast.
- **Multi-entity popups**: out of scope; the `viewport-top` morph is single-page
  focus. Guard the intent path on `placement` involving `viewport-top` + a single
  entity with world coords; everything else keeps current behavior.
- **No intent present** (non-animated placement flips): fall back to current FLIP.

---

## Verification

- `pnpm typecheck && pnpm test:unit && pnpm test:integration` (touches main
  broadcast + camera → integration matters).
- Add one unit test for the endpoint math / `anchoredSlotRectAtCamera` (pure).
- In-app: focus a page, then exit focus. The toolbar morph close should glide with no
  jitter and **no step at landing**. Confirm the page border / grid / scene look
  **identical to today** (they're untouched — if they changed, something leaked into
  the scene, which is the §7c mistake).
- `grep getBoundingClientRect src/renderer/above-view/CanvasItemPopup.tsx` — should
  not appear on the position path.

## Acceptance (north-star)

During the morph: zero React re-renders drive popup position (WAAPI only); both
endpoints computed analytically (no `getBoundingClientRect` for position); page /
scene / border behavior byte-for-byte unchanged.
