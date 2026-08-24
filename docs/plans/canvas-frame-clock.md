# PRD: One frame clock for the canvas

Delete the two 16ms timers on the canvas motion path. The renderer's
`requestAnimationFrame` is the only pacing clock: Chromium already delivers one
wheel event per frame, main applies it synchronously, and the layout pass
collapses bursts on a zero-delay single-flight guard instead of a 16ms wait.

Background: `~/Documents/specular/sixteen-ms.html`,
`docs/canvas-motion-research.md` §2.5 and §7c, ADR 0023 postmortem,
`docs/perf-zoom-pan-log.md` Exp A.

## Problem Statement

Zooming and dragging feel a beat behind the hand. On a 120Hz ProMotion display
the canvas never gets past 60fps, and at 60 there is a periodic stutter while
every computed position is correct.

The cause is cadence, not cost. Input passes through three unsynchronised
clocks in series:

1. `src/main/runtime/viewport-input.ts` — a 16ms input bucket
   (`enqueueViewportInputDelta`).
2. `src/main/runtime/layout-engine.ts` — the 16ms `requestLayout` debounce.
3. The renderer's rAF.

Main has no frame clock, so its two timers drift against the display and
produce doubled and dropped frames. They were built to tame a per-tick
device-emulation reflow storm that bucketed emulation and the zoom snapshot
freeze have since fixed. What remains is lag.

Two extra facts from measurement:

- The renderer side is already display-rate. A wheel counter injected into
  above-view over CDP during a pinch on a 120Hz display recorded 896 wheel
  events, exactly one per rAF frame, median gap 8.3ms. Chromium rAF-aligns
  `wheel` the same way it aligns `pointermove`. No renderer-side coalescer is
  needed; the 60fps cap is entirely main's timers.
- Pan already lays out synchronously inside `setViewportCamera`
  (`viewport-control.ts`), then `applyViewportInputDelta` schedules a second,
  redundant debounced pass 16ms later. Zoom skips the synchronous layout and
  relies only on the debounced one, so its chrome (handles, outlines sized
  against `layoutData.zoom`) lands a frame after the scene transform. That is
  why zoom feels worse than pan.

## Solution

Delete the bucket. Delete the 16ms in `requestLayout`. Zoom gets the pan
treatment: apply on arrival, lay out synchronously, nudge. `requestLayout`
keeps its single-flight guard but fires on `setImmediate`, so a burst of
mutations still collapses to one pass and nothing waits a frame for no
reason.

Main stays the geometry authority. Native pages and their chrome keep stepping
from the same numbers, which ADR 0023's postmortem and the §7c spike both show
is the lock that matters. Only the cadence moves.

## User Stories

1. As a designer zooming with a trackpad, I want the canvas to scale under my fingers without a visible lag, so that the gesture feels direct.
2. As a designer dragging a page, I want the page to stay under the cursor, so that I can place it precisely.
3. As a designer panning, I want no regression from today, so that the fix does not trade one gesture for another.
4. As a designer on a ProMotion MacBook, I want pan, zoom, and drag at 120fps.
5. As a designer on a 60Hz display, I want a slow zoom to read as one continuous movement with no periodic stutter.
6. As a designer, I want selection chrome and the page border to move in lockstep with the page body every frame.
7. As a designer, I want the zoom snapshot freeze and drag freeze to keep working exactly as they do.
8. As a designer, I want a double-click right after a drag to still register, and an inline edit started right after a gesture to keep its focus.
9. As a designer, I want undo during a drag to cancel the drag and restore cleanly.
10. As a designer with twenty pages open, I want zoom to stay responsive.
11. As a developer, I want `requestLayout` to keep collapsing a burst of mutations into one pass, so that a tab switch runs layout once, not thirteen times.
12. As a developer, I want the `local/no-direct-view-mutation` lint invariant to hold.
13. As a developer, I want each PR revertible on its own.
14. As a developer, I want a before/after trace of the same scripted gesture, so the improvement is a number.

## Implementation Decisions

Two PRs. The first is the smallest possible diff and the one most likely to
surface ordering assumptions, so it ships alone.

### PR A. `requestLayout` fires on the next turn

`requestLayout` keeps its guard (a pending request swallows later calls) and
schedules `layoutAllViews` on `setImmediate` instead of a 16ms timer. Nothing
in the pass changes.

Two comments claim to depend on the 16ms and are updated in the same PR:

- `space-observers.ts` (undo observer): the comment says the debounce
  "provides enough deferral to avoid stepping on Electron's event routing"
  and, in the same breath, that the controller is reentrancy-safe so no
  deferral is needed. `setImmediate` still runs the pass outside
  `afterTransaction`, so whatever the hazard was is preserved. Rewrite the
  comment to say the pass runs on the next turn; update
  `src/main/runtime/CLAUDE.md` lines 34 and 84 to match.
- `pointer-session.ts` (phantom-blur guard): the guard swallows a blur
  while a press is armed, regardless of how late the reconcile lands. With
  the pass on the next turn, focus reconciliation fires right after the
  pointerup IPC, before a second pointerdown can arrive, so the window the
  guard covers gets smaller. The guard stays; its comment drops the
  "16ms later" framing.

Ship with the manual checklist below. Anything that was silently relying on
a frame of delay shows up here, and the right response is an explicit
deferral with a reason, not restoring the 16.

### PR B. Delete the input bucket; zoom lays out synchronously

- Delete `enqueueViewportInputDelta`, `pendingViewportDelta`, and
  `VIEWPORT_EVENT_FRAME_MS` from `viewport-input.ts`. The `canvasZoom` and
  `canvasPan` IPC handlers call `applyViewportInputDelta` directly.
- In `setViewportCamera`, remove the `if (!zoomChanged)` guard so zoom and
  pan both run `layoutAllViews()` before the nudge. During a zoom gesture the
  pages are frozen bitmaps, so the per-frame pass is mostly the scene payload
  rebuild.
- Delete the trailing `requestLayout()` in `applyViewportInputDelta`. The
  synchronous pass already ran; the settle callbacks in `markPanMotion` /
  `markZoomMotion` own the re-layout on settle.
- `pan-zoom-perf-test.ts` drives input at the display's refresh interval
  instead of a fixed 16ms and reports the interval it used.

Drag needs no change. Drag moves are entity mutations that request layout,
and after PR A that lands on the next turn. Chromium already delivers
`pointermove` once per frame.

### Not changed

- Main remains the geometry authority. No camera state moves to the renderer.
- `LAYER_STACK`, `applyStack`, and the ADR 0014 banding are untouched.
- The programmatic camera tween (focus, zoom-to-fit) keeps its main-side
  interval. It benefits from PR A automatically; moving it is a separate PRD
  because of the border-to-page lock risk in §7c.
- Settle timing (300ms), bucketed emulation, zoom snapshot freeze, drag
  freeze, and the comment-tool pointer rate limit are unchanged.
- The focus reconciler still runs unconditionally at the end of every layout
  pass (Phase 5d-v2 D4).

## Testing Decisions

A good test here asserts what a user or a downstream subsystem can observe.
It does not assert on timer handles or internal flags.

- Layout request single-flight (integration). Call `requestLayout` N times
  in one turn, await a turn, assert one pass ran and the views hold the final
  geometry. Prior art: the layout-pass coverage from the complete-layout-pass
  plan.
- Undo mid-drag (integration). Begin a drag, undo, await a turn, assert
  interaction state is idle, geometry matches the pre-drag doc, and nothing
  threw from a reentrant layout. Prior art: `tests/integration` undo
  round-trip tests.
- Viewport input (integration). Send a zoom delta, assert the page view's
  bounds and the camera updated before the handler returned. Prior art:
  harness-level assertions in `tests/integration/harness.ts`.

Measurement: record a Chromium trace (`Cmd+Alt+Shift+P` or
`POST localhost:29979/perf/trace/start`) of the scripted pan/zoom gesture on
main, after PR A, and after PR B, on a 120Hz display. Attach all three to the
integration PR. The perf HUD's existing fps readout should show 120 during
zoom after PR B.

Manual smoke once per branch before merge: pan, zoom, drag, double-click
after drag, inline edit after drag, undo during drag, tab switch, page create
and delete, comment tool hover, focus animation, on both a 60Hz and a 120Hz
display.

## Out of Scope

- Renderer-owned camera (ADR 0023 Phases 1 through 4). Rejected once; not
  being retried.
- Moving the programmatic camera tween to the renderer. Separate PRD.
- Making focus reconciliation an explicit, requested step instead of running
  on every pass. Reopens D4; not needed for cadence.
- A renderer-side input coalescer. Measured unnecessary (see Problem
  Statement).
- A HUD input-to-present latency readout. The trace answers the question.
- The wheel listener in canvas-bg's `useCanvasViewportGestures.ts` received
  zero events during measurement; all wheel input lands on above-view. Worth
  checking whether it is dead, in a separate change.

## Further Notes

- Expect PR A to be where surprises surface. The 140 `requestLayout()` call
  sites were written against a 16ms delay and any of them may have quietly
  relied on it.
- PR B runs the synchronous layout path on every frame during zoom, which is
  twice today's pan rate on a 120Hz display. If the scene payload rebuild
  shows up in traces at high page counts, the follow-up is the "cheap
  payload for camera-only change" idea from #265, not a timer.
- Each PR into `perf/canvas-frame-clock`, then one integration PR into
  `main`, per the AFK convention.
