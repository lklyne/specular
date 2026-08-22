# PRD: One frame clock for the canvas

Replace the stacked 16ms timers on the canvas motion path with a single
vsync-driven cadence. The renderer's `requestAnimationFrame` becomes the only
pacing clock; main applies camera and drag input synchronously, and the layout
pass coalesces on a zero-delay single-flight guard instead of a 16ms wait.

Background research: `~/Documents/specular/sixteen-ms.html` (the explainer on
the canvas), `docs/canvas-motion-research.md` §2.5 and §7c, ADR 0023
postmortem, `docs/perf-zoom-pan-log.md` Exp A.

## Problem Statement

Zooming and dragging on the canvas feel a beat behind the hand. Pan feels
better than zoom. On a 120Hz ProMotion display the canvas never gets past
60fps, and even at 60 there is a periodic stutter that shows up while every
computed position is correct.

The cause is cadence, not cost. An input event passes through up to three
unsynchronised ~16ms clocks in series before the renderer draws: a 16ms input
bucket in main, a 16ms `requestLayout` debounce in main, then the renderer's
own rAF. Main has no frame clock, so its timers drift against the display and
produce repeated and wasted frames. The bucket and debounce were built to tame
a per-tick device-emulation reflow storm that bucketed emulation and the zoom
snapshot freeze have since fixed. What remains is lag.

Two pieces of code quietly depend on the debounce being late: the phantom-blur
guard in the press gesture, and the undo observer's deferral "to avoid stepping
on Electron's event routing." Those are ordering bugs that got a timer instead
of a fix, and they have to be handled before the timer can go.

## Solution

One clock drives canvas motion. The renderer coalesces wheel and pointer input
per animation frame and sends one summed delta. Main applies it synchronously,
positions the native views, and nudges the renderer, the way pan already does.
The layout pass keeps its single-flight guard but fires on the next event-loop
turn, so bursts still collapse and nothing waits a frame for no reason. Focus
reconciliation and the undo deferral become explicit steps the gesture
controller owns, so nothing relies on "16ms later."

Zoom and drag get the pan treatment. Pan loses its remaining bucket. ProMotion
displays run at their native rate. The judder goes away because there is no
second clock to beat against.

Main stays authoritative for geometry. The native page and its chrome keep
stepping from the same numbers, which ADR 0023's postmortem and the §7c spike
both show is the lock that matters. Only the cadence moves.

## User Stories

1. As a designer zooming with a trackpad, I want the canvas to scale under my fingers without a visible lag, so that the gesture feels direct.
2. As a designer dragging a page, I want the page to stay under the cursor, so that I can place it precisely without overshooting.
3. As a designer panning with two fingers, I want the same responsiveness pan has today with no regression, so that the fix does not trade one gesture for another.
4. As a designer on a ProMotion MacBook, I want pan, zoom, and drag to render at 120fps, so that the canvas feels as smooth as Figma on the same hardware.
5. As a designer on a 60Hz external display, I want motion without periodic stutter, so that a slow zoom reads as one continuous movement.
6. As a designer, I want the selection outline, handles, and page border to move in lockstep with the page body, so that chrome never visibly detaches mid-gesture.
7. As a designer, I want the zoom snapshot freeze to keep working exactly as it does, so that live pages still become bitmaps during a gesture and re-raster crisp on settle.
8. As a designer, I want drag freeze to keep working exactly as it does, so that dragging a page still shows its bitmap instead of a reflowing live view.
9. As a designer, I want a double-click that lands right after a drag to still register, so that the phantom-blur fix does not reintroduce dropped second clicks.
10. As a designer, I want an inline text edit started right after a gesture to keep its focus, so that the edit is not torn down by a late focus reconcile.
11. As a designer, I want undo during a drag to cancel the drag and restore cleanly, so that the undo observer's deferral change does not leave the canvas in a half state.
12. As a designer, I want undo and redo of a tab switch, page create, and page delete to still land the native views in the right order, so that no page appears above the toolbar.
13. As a designer, I want focus and zoom-to-fit animations to stay smooth or get smoother, so that the camera tween does not regress when the debounce under it changes.
14. As a designer using the comment tool, I want hover outlines in pages to keep updating at the same rate, so that the comment pointer rate limit is untouched.
15. As a designer with twenty pages open, I want zoom to stay responsive, so that applying input per frame does not scale badly with page count.
16. As a designer, I want a trackpad momentum pause inside a pinch to still count as one gesture, so that the 300ms settle behaviour is preserved.
17. As a designer, I want the grid, group backgrounds, and device shells to move with the camera every frame, so that the background sheet does not lag the page.
18. As a designer, I want agent presence cursors to re-baseline correctly after a pan, so that the settle re-layout still runs once.
19. As a developer, I want the automated pan/zoom perf test to drive input at display rate, so that a 120Hz improvement is measurable instead of invisible.
20. As a developer, I want the perf HUD to report input-to-present latency for a gesture, so that the improvement is a number and not a feeling.
21. As a developer, I want a before/after trace of the same scripted gesture in the logs folder, so that the PR carries evidence.
22. As a developer, I want `requestLayout` to keep collapsing a burst of mutations into one pass, so that a tab switch still runs the layout pass once, not thirteen times.
23. As a developer, I want the `local/no-direct-view-mutation` lint invariant to hold, so that nothing outside the layout pass touches native view geometry.
24. As a developer, I want each step of this work to ship on its own and be revertible on its own, so that a regression in one step does not take the others with it.
25. As a developer, I want the interaction-layer doc and CONTEXT.md to describe the new cadence, so that the next person does not re-add a timer to fix an ordering bug.
26. As a developer, I want the two hidden dependencies documented where they are fixed, so that the phantom-blur and undo-deferral reasons are written as "why," not as "used to."
27. As a developer, I want integration coverage of the undo observer's new deferral, so that the reentrancy hazard it protects against is pinned by a test instead of a comment.
28. As a developer, I want the renderer-side input coalescer to be a pure, testable function, so that the summing and mouse-anchor rules have unit coverage.

## Implementation Decisions

Sequenced so each step is independently shippable. Steps 1 and 2 are
prerequisites; 3 through 5 are the payoff; 6 is the measurement that makes
the payoff visible.

### Step 1. Explicit focus settle

Focus reconciliation currently runs at the end of every layout pass, and the
press gesture's phantom-blur guard exists because that pass lands 16ms after a
gesture ends. Make focus settle an explicit step: the interaction controller
requests it on gesture end and on interaction-state change, and it runs from
the layout pass only when requested. The press gesture's blur guard is
re-derived from that contract. If the guard is still needed it gets a comment
explaining the remaining window; if not, it is deleted.

### Step 2. Undo observer owns its deferral

The undo observer's comment says the 16ms debounce provides "enough deferral"
to avoid stepping on Electron's event routing. Decide whether the hazard is
real with a test: cancel an active drag via undo from inside an IPC handler
and assert the runtime settles. If it is real, the observer schedules its own
zero-delay deferral and says why. If not, it calls the layout request directly
and the comment goes. Either way the observer stops borrowing the debounce.

### Step 3. Layout request becomes single-flight on the next turn

`requestLayout` keeps its guard (a pending request swallows later calls) and
fires on `setImmediate` instead of a 16ms timer. Bursts still collapse to one
pass. No behaviour in the pass itself changes. This is the smallest diff in
the PRD and the one most likely to surface remaining ordering assumptions, so
it ships alone with the full manual checklist from the layout-pass plan.

### Step 4. Renderer coalesces viewport input per frame

The canvas-bg and above-view gesture hooks stop sending one IPC per wheel or
pointer event. A pure coalescer accumulates pan and zoom deltas and the latest
mouse anchor inside a frame; on `requestAnimationFrame` it sends one delta.
Main's input bucket is deleted; `applyViewportInputDelta` runs on arrival and
lays out synchronously for both pan and zoom, the way pan does today. The
viewport nudge still goes out immediately after. The settle timers, bucketed
emulation, and zoom snapshot freeze are untouched and continue to gate the
expensive work.

The coalescer lives in shared code with no DOM dependency so its rules are
unit-testable: deltas sum, the most recent mouse anchor wins, a zoom and a pan
in the same frame ship together, an empty frame sends nothing.

### Step 5. Drag applies synchronously

Drag move IPC already arrives per pointer event with no bucket. After Step 3
the only wait left is the layout request; drag moves call the synchronous
layout path directly (delta apply, native `setBounds`, nudge) so the dragged
page lands in the same frame as the cursor. The renderer coalesces pointer
moves per rAF using the same coalescer as Step 4 so main is not asked to lay
out more than once per display frame. Drag freeze bitmaps ride the same
payload they do today.

### Step 6. Measure at display rate

The automated pan/zoom perf test drives input at the display's refresh
interval instead of a fixed 16ms, and reports the interval it used. The perf
HUD gains an input-to-present latency readout: the renderer stamps each
coalesced send, main echoes the stamp on the nudge, and the renderer measures
against the next presented frame. A scripted gesture is traced before Step 3
and after Step 5 and both traces are attached to the integration PR.

### Not changed

- Main remains the geometry authority. No camera state moves to the renderer.
- `LAYER_STACK`, `applyStack`, and the ADR 0014 banding are untouched.
- The programmatic camera tween (focus, zoom-to-fit) keeps its main-side
  interval for now. It benefits from Step 3 automatically; moving it to the
  renderer is a separate PRD because of the border-to-page lock risk in §7c.
- The comment-tool pointer rate limit stays.
- Settle timing (300ms), bucketed emulation, zoom snapshot freeze, and drag
  freeze are unchanged.

## Testing Decisions

A good test here asserts what a user or a downstream subsystem can observe:
how many layout passes ran for a burst, where a native view ended up, whether
focus landed on the expected surface, whether the runtime settled after an
undo mid-gesture. It does not assert on timer handles or internal flags.

Modules to test:

- Viewport input coalescer (unit). Pure function, table-driven: summing,
  anchor precedence, mixed pan+zoom frames, empty frames. Prior art:
  `tests/unit/zoom-motion.test.ts`, `tests/unit` gesture-utils coverage.
- Layout request single-flight (integration). Call the request N times in one
  turn, assert one pass ran and the views hold the final geometry. Prior art:
  the layout-pass coverage added with the complete-layout-pass plan.
- Focus settle (integration). Gesture end followed immediately by a second
  press; assert focus stays on above-view and the press commits. Prior art:
  press-gesture unit tests and the focus reconciler coverage.
- Undo mid-drag (integration). Begin a drag, undo, assert interaction state is
  idle, geometry matches the pre-drag doc, and no error was thrown from a
  reentrant layout. Prior art: `tests/integration` undo round-trip tests.
- Drag sync path (integration). Send a drag move and assert the page view's
  bounds updated before the handler returned. Prior art: harness-level
  assertions on runtime arrays in `tests/integration/harness.ts`.

Manual smoke once per branch before merge, per the project convention: pan,
zoom, drag, double-click after drag, inline edit after drag, undo during drag,
tab switch, page create and delete, comment tool hover, focus animation, on
both a 60Hz and a 120Hz display.

## Out of Scope

- Renderer-owned camera (ADR 0023 Phases 1 through 4). Rejected once; not
  being retried here.
- Moving the programmatic camera tween to the renderer. Separate PRD.
- Any change to zoom snapshot freeze, drag freeze, bucketed emulation, or the
  300ms settle.
- Smoothing the DOM overlay independently of the native page (§7c showed this
  regresses).
- The comment-tool pointer rate limit and the perf-test clock beyond making
  the latter display-rate.

## Further Notes

- The order matters. Steps 1 and 2 look like cleanup but they are the reason
  the debounce has not already been shortened. Shipping Step 3 first would
  reintroduce the dropped-second-click bug the phantom-blur guard was written
  for.
- Expect Step 3 to be where surprises surface. Anything that was silently
  relying on a frame of delay will show up there, and the right response is an
  explicit deferral with a reason, not restoring the 16.
- Step 4 runs the synchronous layout path on every frame during zoom. Pages
  are frozen bitmaps during a zoom gesture, so the per-frame pass is mostly
  the scene payload rebuild. If that rebuild shows up in traces at high page
  counts, the follow-up is the existing "cheap payload for camera-only change"
  idea from #265, not a timer.
- Each step is one PR into `perf/canvas-frame-clock`, then one integration PR
  into `main`, per the AFK convention.
