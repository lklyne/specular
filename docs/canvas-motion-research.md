# Canvas Motion — Greenfield Research

*Research note. How to animate canvas-anchored overlays (and every canvas item)
smoothly, with React only setting targets — never in the per-frame loop. Written
against the toolbar-morph jitter, but the answer is system-wide.*

Sibling to `docs/offscreen-rendering-research.md`. This is a design investigation,
not a committed plan — it ends with a sequenced set of shippable steps.

---

## 1. The one-sentence diagnosis

**Everything on the canvas moves on main's stepped, integer-rounded 16 ms clock,
delivered over IPC — and the only genuinely smooth motion in the system is the
handful of animations a renderer runs *locally* (WAAPI, the CSS pan nudge). Jitter
is the seam where those two meet mid-motion.**

The toolbar morph is not a special problem. It's the first place the seam became
visible.

---

## 2. How motion actually works today (ground truth)

Traced through `viewport-control.ts`, `camera-transition.ts`, `layout-engine.ts`,
`runtime-geometry.ts`, the `*-entity-state.ts` builders, and the canvas-bg /
above-view renderers.

### 2.1 The camera clock lives in main, and it steps

`moveCameraTo` (`viewport-control.ts:284`) runs a `setInterval` at
`CAMERA_TRANSITION_FRAME_MS` (16 ms). Each tick:

```
t = (Date.now() - start) / duration
applyCamera(interpolateCamera(startCamera, target, t))   // pan.x/y ROUNDED to int
requestLayout()                                          // 16 ms debounced pass
```

`interpolateCamera` (`camera-transition.ts:46`) **rounds pan to integers**. So even
in theory the camera samples are integer-quantized — 1 px jumps at low zoom, no
matter the frame rate.

### 2.2 Entity geometry is analytic, computed in MAIN, re-broadcast every tick

There is **no scaled CSS container** that canvas items ride under. Every entity's
screen rect is computed analytically in main from the camera —
`screenX = canvasOrigin.x + worldX * zoom + pan.x`, and `width * zoom` — in
`runtime-geometry.ts` and each `*-entity-state.ts` builder. The renderer drops the
pre-projected `screenX/screenY/screenWidth/screenHeight` straight into absolute
`left/top/width/height`.

Consequence: a camera move re-computes and re-broadcasts every entity's rect, 20×
over the move. The renderer coalesces to **one rAF `setState` per frame**
(`useCanvasLayoutState.ts`), and `React.memo` + hoisted slices keep most layers from
re-rendering — but the layout *object identity* changes each frame, so anything that
reads it (every `useAnchoredPosition` consumer, the popup wrapper) recomputes.

### 2.3 Native pages are set imperatively, on the same stepped clock

The live page is a native `WebContentsView`. Its bounds are set with `setBounds`
(integer) in the layout pass from the *same* main-side geometry
(`layout-engine.ts:427`). So the native page and the DOM chrome share endpoints and
stay locked — but both **step** at 16 ms integer positions.

### 2.4 The two things that are already smooth — and why

- **The pan nudge** (`useScenePanOffset.ts`): during a pan gesture, one wrapper gets
  a transient `translate3d(nudge − broadcastPan)` that self-reconciles to zero when
  the next broadcast lands. Zero re-render, subpixel. **Zoom is explicitly excluded**
  (`if (nudge.zoom !== payload.zoom) return ZERO`).
- **The toolbar open morph** (`CanvasItemPopup.tsx`): a pure WAAPI tween toward a
  *fixed* viewport rect, on the `CAMERA_SPRING_CSS_EASING` `linear()` curve, phase-
  synced to `cameraTransitionStartedAt`. The broadcast never touches it.

Both are **local, renderer-owned, target-declared** animations. That is the entire
recipe for smoothness in this codebase — and it's already sitting in two places.

### 2.5 Does the 16 ms throttle affect our animations?

Depends which animation — and the distinction *is* the whole story.

**Locally-run WAAPI/CSS animations (the toolbar-open tween, the pan nudge): no.**
They run on the compositor's own vsync clock, independent of all three timers. The
throttle never gates their frame rate. It only touches their *inputs* — the start
phase (`cameraTransitionStartedAt`) and, for the close morph, the target rect they
chase. That's exactly why fixed-target = smooth and moving-target = jitter: the tween
is fine; the *value fed to it* arrives stepped.

**Broadcast-driven motion (React-re-rendered entity rects, native page `setBounds`,
the outer popup wrapper): yes — and worse than "capped at 60fps."** Three
unsynchronized ~16 ms clocks sit in series:

1. main camera `setInterval(16 ms)` — ~20 samples over a 320 ms move
2. `requestLayout()` 16 ms `setTimeout` debounce
3. renderer `requestAnimationFrame` coalescing

plus native `setBounds` on the same layout pass. The failure modes compound:

- **Hard ~60 fps ceiling.** Both main timers are 16 ms, so broadcast-driven motion
  can't exceed ~60 fps *even on a 120 Hz / ProMotion display* — it renders half-rate.
- **No phase alignment with each other or with display vsync.** A broadcast landing
  just after a rAF waits a full frame. A 16 ms sample cadence beating against a
  ~16.6 ms display cadence produces periodic doubled/dropped frames — judder, not
  merely "slower."
- **Timer drift.** `setTimeout`/`setInterval` in main aren't precise and get delayed
  under load (GC, IPC, the `buildMs` layout rebuild). `interpolateCamera` is
  time-based, so *positions* are correct-for-the-timestamp, but *delivery spacing* is
  uneven → judder even with analytically-correct values.

So the throttle doesn't make animations run at the wrong *speed* — it makes the
**broadcast-fed** ones arrive on a stepped, vsync-unaligned, drift-prone cadence while
the **local WAAPI** ones run vsync-smooth. The jitter is those two cadences visible in
the same frame. This is the mechanical case for §5 / Step 1: broadcasting camera
*intent* (one target + curve) pulls all three throttle stages out of the motion path —
the renderer animates on its own vsync clock, so everything joins the smooth side of
the seam.

### 2.6 Why the morph is asymmetric (the specific bug, generalized)

- **Open** tweens toward a *fixed* target (the viewport-pinned bar). Pure local WAAPI.
  Smooth.
- **Close** tweens toward the *page-anchored* rect — which is **itself moving on the
  stepped broadcast clock**. The inner FLIP runs smooth on WAAPI, but the outer
  wrapper's `transform`/`width` are recomputed from the stepped broadcast rect every
  frame. Smooth-inner + stepped-outer = jitter, worst at landing.

Fixed target ⇒ smooth. Moving target ⇒ jitter. Same seam as §1.

---

## 3. The key realization

**The renderer already has everything it needs to place any entity itself, and does
not use it during motion.**

`coords.ts` is the analytic projection: `canvasToScreenX(layout, x) = x*zoom +
pan.x + originX`. World coords + camera are both in the broadcast. The *only* reason
the renderer waits for main to pre-project and re-broadcast is history: main owns the
animation clock and pre-computes screen rects.

Flip that and everything opens up:

> Give the renderer the camera *destination and curve* instead of 20 stepped
> *samples*. Let it own one animatable transform. Project entities from world
> coords through that transform. Motion becomes local, subpixel, and re-render-free —
> by construction, for the whole canvas, not just the toolbar.

There's already a proof-of-concept of exactly this decoupling in the tree: the pan
nudge. It's the idea in miniature — local motion that ignores the broadcast and
reconciles at the end. The work is generalizing it (to zoom, to animated moves, to
both DOM planes) and solving the one thing it doesn't cover: native pages.

---

## 4. Q1 — The toolbar morph, zero re-renders, both directions

Make close symmetric with open: **a single local WAAPI tween between two known rects,
on the spring curve, phase-synced — ignoring every intermediate broadcast.**

Open already tweens to a fixed rect. Close jitters only because its end-rect is read
live. But the close end-rect is *computable at the instant close starts*: you know the
entity's world coords, and you know the camera's final resting value (focus-exit
restores a stored return camera). Project once —
`canvasToScreen(finalCamera, worldRect)` — tween start→end locally, drop the
broadcast on the floor for the duration.

Zero re-renders in the popup subtree during the morph. No `getBoundingClientRect`, no
FLIP-from-measured-baseline, no chasing a live rect.

The one enabling input: the renderer must know the **final camera** at animation
start. Two ways — (a) broadcast it (one field: `cameraTarget` + `startedAt` +
`duration`), or (b) if the return camera is already derivable renderer-side, project
without any new field. (a) is the honest primitive and it generalizes — see Q2.

*(This is the handoff's "option 1," sharpened: the fix is projecting the end-rect
from the known final camera, not snapshotting a broadcast rect.)*

---

## 5. Q2 — Every canvas item, smoothly, without re-renders

Generalize §4's primitive into the camera itself.

**Broadcast camera *intent*, not camera *samples*.** At move-start, main sends once:
`{ startCamera, targetCamera, startedAt, durationMs, curve }`. It does *not* need to
tick 20 stepped frames over IPC for the DOM to move.

The renderer owns **one canvas-space transform per DOM plane** (bgView, aboveView),
animates it with WAAPI using the existing `CAMERA_SPRING_CSS_EASING` `linear()`
easing, from `translate(startPan) scale(startZoom)` → `translate(targetPan)
scale(targetZoom)`. Entities live in **world coordinates** inside that container
(`left: worldX, top: worldY`, no zoom baked in). They ride the transform for free —
subpixel, zero React re-render for the entire move. React re-renders only when the
*model* changes (entity added / moved / resized), never for camera motion.

For **gesture** pan/zoom (user-driven, not animated), the same container transform is
driven by the live gesture delta locally and reconciled on gesture end — you already
do this for pan via `useScenePanOffset`; extend it to zoom (the currently-excluded
case) and gesture zoom is smooth and re-render-free too.

This requires two changes and hits one wall:

1. Entities positioned in **world coords**, not pre-projected screen coords — i.e.
   move the `* zoom + pan` projection out of the `*-entity-state.ts` builders and
   into the one CSS transform.
2. `useAnchoredPosition` derives from world coords + camera via `coords.ts` instead of
   reading `entity.screenX` — killing the measure/FLIP path (handoff item 4).
3. **The wall: native `WebContentsView` pages are not in the DOM transform.** See §7.

---

## 6. Q3 — Options the handoff didn't list

The handoff had (1) fix the target, (2) low-pass the target, (3) shrink step size,
(4) sidestep (pin/fade). Additional live options:

**5. Broadcast camera intent, not samples** (the §5 primitive). Highest leverage,
smaller than a compositor rewrite. Removes stepped IPC from the motion hot path
entirely. Everything downstream of it gets smoother for free.

**6. World-space containers + one CSS/WAAPI camera transform** (the §5 layer). Pairs
with #5. This is the "single canvas-space transform layer" from the handoff, made
concrete: the transform is the camera; children are camera-independent.

**7. Two-element cross-fade handoff, not a cross-space tween.** The toolbar morphs
between canvas-space (tracks camera) and viewport-space (pinned bar) — two coordinate
systems. Instead of tweening one element *across* spaces (FLIP + measuring + cross-
space math), render **two** toolbars — one canvas-anchored, one viewport-pinned — and
cross-fade + slide between them on focus toggle. Each lives natively in its own space
and is never tweened out of it. Likely the *simplest correct* morph. Cost: briefly
rendering two toolbars and matching their content.

**8. Renderer-side rAF projection from shared clock** (poor-man's #5, no
rearchitecture). Keep main broadcasting, but the renderer runs its own rAF loop that,
given `{ startedAt, targetCamera, duration, curve }`, computes the current camera each
frame and positions overlays locally — ignoring broadcast samples during the move.
Mutate style via refs, not `setState`, so still zero React re-render. This is the
cheapest way to get §5's smoothness for *overlays only* without touching entity
rendering.

**9. `view-transition` API** — rejected. Built for discrete DOM state swaps; snapshots
a layer and can't continuously track a moving *native* page underneath.

**10. Full WebGL/2D-canvas scene** — rejected. The native pages remain the hard part
regardless; the WCV constraint makes this not worth it.

**11. Detach-to-viewport during motion** (principled "sidestep"). During any camera
animation, the toolbar holds a fixed viewport position (or fades), reattaching to the
page anchor on settle. Cheapest of all; sacrifices "glued to the page" *during*
motion. Fine if moves are short (they're 320 ms).

---

## 7. The wall: native pages, stated honestly

A smooth CSS-transformed overlay layer would slide *out of phase* with the native
page, because the page's `setBounds` is stepped integer geometry from main. Two ways
through, cheap-first:

**7a. Phase-lock the native steps to the same curve (cheap, spike first).** Renderer
owns the clock and tells main `{ target, startedAt, duration, curve }`; main drives
`setBounds` on that same timebase. The page still *steps* (WCV bounds are integer),
but it steps toward the same endpoints on the same curve as the smooth overlay above
it. Whether a smooth border over a 16 ms-stepped page body is perceptible *during a
320 ms move* is an empirical question — build the spike, watch it, decide. If it's
imperceptible, this is the whole fix and it's small.

**7b. Composite moving pages as textures into the transformed layer (the real fix,
expensive).** Take pages *out* of per-frame native-bounds during a move: render the
moving page as a bitmap/`useSharedTexture` surface inside the DOM layer so it rides
the same transform as everything else. This is already a roadmap track
(`docs/offscreen-rendering-research.md`, Electron gotcha #12). Reach for it only if
7a's step is visibly out of sync.

### 7c. Empirical result (spike, reverted)

A first spike built §5 for the DOM scene only — renderer-owned rAF driving the page
chrome + grid smoothly, native pages left stepping on main's `setBounds`. **It made
things worse, not better.** With the border gliding on vsync and the page body
stepping at 16 ms, the two *visibly separated* mid-move — the border looked janky
*because it detached from the page it borders*. Previously both stepped from the same
geometry, so they were locked; ugly-but-glued reads as clean, smooth-but-detached
reads as broken.

Takeaway that reorders the plan: **the border-to-page lock dominates absolute
smoothness.** You cannot smooth the DOM overlay layer without smoothing (or at least
sub-stepping) the native page in the *same* motion — Step 2 and Step 3 below are
**coupled, not independent**. Solve the native page (§7a phase-lock or §7b texture)
*first*, or in lockstep; smoothing the overlay alone is a regression. The toolbar
morph (§4) is exempt: it has no native page bordering it, which is exactly why that
piece worked in isolation.

A related tell that the coupling is real: `interpolateCamera` rounds pan to integers
(§2.1) largely because native bounds are integer anyway. Under a CSS-transform model
the *overlay* transform can be fractional/subpixel while native bounds round
independently — the rounding is another symptom of overlay motion being chained to
native-bounds constraints.

---

## 8. Q4 — Step back: what this could be

Today: **one clock (main's 16 ms interval) drives two motion systems that render
differently** — native WCV `setBounds` (stepped/integer) and the DOM (stepped, but
*capable* of smooth) — plus overlays as a third thing that sometimes animates locally
(smooth) and sometimes tracks the broadcast (stepped). The jitter is the seam.

The coherent end-state:

> **The renderer owns camera motion as a single animatable transform. Main owns only
> the destination.** The camera is not "20 broadcast samples" — it's "a target and a
> curve." Native pages are the one thing that can't live in a CSS transform, so they
> are the *one* deliberate integration boundary: phase-lock their stepped bounds to
> the same curve (cheap) or composite them as textures into the transformed layer
> (the roadmap fix).

Under that model, the answers fall out for free:

- **Toolbar morph** = animate one element's transform between two locally-projected
  rects. Re-render-free both directions, trivially.
- **Every canvas item** = re-render-free during any camera move, by construction.
- **`getBoundingClientRect` / FLIP** = gone. Geometry is analytic (`coords.ts`) from
  world coords + camera, never measured.

### Sequencing (lazy, incremental, each shippable on its own)

- **Step 0 — now.** Fixed-target close: project the close end-rect from the known
  final camera, single WAAPI tween. Symmetric with open, re-render-free morph.
  Contained to `CanvasItemPopup.tsx` (+ maybe one broadcast field). Ships the jitter
  fix without waiting for any of the below.
- **Step 1.** Broadcast camera *intent* (`target + startedAt + duration + curve`)
  alongside today's samples. Switch overlays/morphs to project locally from intent
  (option 8). Removes overlay dependence on stepped samples; touches no entity
  rendering.
- **Step 2 + 3 are coupled (see §7c) — do them together, native-page first.** The
  DOM half — world-space containers + WAAPI-driven camera transform for bgView +
  above-view, entities in world coords, retiring `entity.screenX` pre-projection for
  `coords.ts` at the edges — **must not ship before** the native-page half, or the
  border detaches from the page and it's a regression. Reconcile native pages first:
  spike 7a (phase-locked steps) and only if that step is still visibly out of sync,
  reach for 7b (texture compositor). Then smooth the DOM in the same move.

### North-star acceptance test (unchanged)

During any camera move or morph: **zero React renders in the canvas subtree; all
motion expressed as WAAPI/CSS with declared targets; no `getBoundingClientRect`.**

---

## 9. Files that matter

| Concern | File |
|---|---|
| Camera loop (stepped `setInterval`) | `src/main/runtime/viewport-control.ts:269` |
| Integer pan rounding + spring + `linear()` easing | `src/shared/camera-transition.ts` |
| Analytic screen-rect projection (main) | `src/main/runtime/runtime-geometry.ts`, `*-entity-state.ts` |
| Layout pass + native `setBounds` | `src/main/runtime/layout-engine.ts:427` |
| One rAF-coalesced `setState` per frame | `src/renderer/canvas-bg/useCanvasLayoutState.ts` |
| The pan-nudge precedent (local, re-render-free) | `src/renderer/shared/hooks/useScenePanOffset.ts` |
| Analytic projection available to the renderer | `src/shared/coords.ts` |
| Overlay geometry from broadcast (to become world-coord) | `src/renderer/above-view/useAnchoredPosition.ts` |
| The morph itself | `src/renderer/above-view/CanvasItemPopup.tsx` |
| Native-page texture path (the 7b roadmap) | `docs/offscreen-rendering-research.md` |
