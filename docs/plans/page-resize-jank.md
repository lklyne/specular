# Page resize jank: what to measure before fixing anything

Status: plan, 2026-10-09. Nothing here is built. Branch `claude/browser-tabs-layout`.

Two things look wrong when a page's size changes in the Kit shell (`specular-shell`).

- **Window drag.** A page in Fill reflows with the window. During the drag the page lags the window edge and the canvas jitters.
- **Tab or lens switch.** The page changes size once. The frame painted at the old size shows in the corner for a moment before the new one arrives.

Both are the same problem. Three things have to agree on a size, they run on three clocks, and today nothing ties them together.

## How a resize travels today

Read from the code on this branch, not measured.

```
AppKit live-resize step (main thread, tracking run-loop mode)
  GPUIView setFrameSize:      GPUI resizes its drawable, calls the resize callback
  GPUIView displayLayer:      GPUI lays out and paints NOW, presentsWithTransaction = true,
                              its display link stopped. Chrome lands in the window's own
                              CA transaction.            gpui-pre-macos window.rs:3253, 3291
    canvas slot prepaint      canvas.set_slot -> Event::ViewportResized -> update
                              -> Effect::SetPageViewport -> CefPageSource::set_viewport
                              -> host.was_resized()      view/slot.rs:260, cef/source.rs:282
  SpecularCanvasView          autoresizes with the window. Its CAMetalLayer keeps the OLD
                              drawable size. Nothing draws it here.

our CADisplayLink tick (main thread, common modes, whenever the run loop gets to it)
  CanvasSurface::sync         surface.configure at the new size         surface.rs:175
  Runtime::turn               drains the CEF event queue
  Runtime::draw               view -> scene -> render -> queue.present  (async present)

CEF (browser side is on the main thread too, pumped by a CFRunLoopTimer)
  WasResized -> GetViewRect -> renderer layout -> viz -> capture
  -> OnAcceleratedPaint (inside a pump) -> thread-local queue -> waits for our next tick
```

What follows from that:

1. **The chrome is in the resize transaction and the canvas is not.** GPUI presents with the transaction. Our layer presents on its own, at least one tick later. Until then Core Animation scales the old drawable to the new layer bounds, because nobody set `contentsGravity` and the default is resize. I expect this is the canvas jitter, and it would happen with no page on the canvas at all.
2. **The page frame is one more hop behind.** A frame at the new size needs a pump to deliver `WasResized`, Chromium's whole pipeline, a pump to deliver the paint, then our next tick. Commit `cb426e89` measured CEF repainting about one resize step in three.
3. **Everything shares the main thread.** GPUI's synchronous layout, our display link, the CEF pump and every CEF callback. The pump timer's fallback rate is 30 Hz (`PUMP_FALLBACK_SECONDS`). The one number we have, 38 canvas frames in a 1.2 s drag, is from a debug build.
4. **A tab switch is more than one resize.** `follow_presentation` sends the new viewport at once. The texture scale follows the LOD, which waits `TEXTURE_GROW_SETTLE` (120 ms) before growing, and the frame rate tier changes separately. A page shown from a 25% canvas into Fill on a Retina display goes from scale 1 to scale 2. So the host is resized twice, and the first new frame is at the wrong scale.
5. **A frame's CSS size is guessed.** `PageGeometry::frame_viewport` infers it from the texel size and the last scale that matched. With a viewport and a scale both in flight the guess can be wrong. CEF hands us real numbers we ignore: `AcceleratedPaintInfo.extra` has `source_size`, `content_rect`, `capture_counter` and `timestamp`.
6. **Every new size is an import miss.** `ingest_shared` drops the import cache when the size changes, so each resized frame makes a Metal texture and a bind group. Chromium rebuilds its capture pool at each size too. Cost unknown.

None of this is ranked yet. That is what the instrumentation is for.

## What we need to know

| # | Question | Decides |
|---|---|---|
| Q1 | Per resize step, how late is the canvas layer behind the window edge, in ms and in pixels? | Whether to draw the canvas inside the resize transaction |
| Q2 | Per resize step, how long from `was_resized` to a painted frame of that size, and where does it go: waiting for a pump, inside Chromium, waiting for our tick? | Pump scheduling, external begin frames, or coalescing |
| Q3 | How many `was_resized` calls never get a frame of their size? | Whether CEF drops or holds resizes |
| Q4 | What share of the main thread does each tenant take during a drag: GPUI, our frame, the pump? | Whether to cut GPUI work during resize, or move work off the thread |
| Q5 | What does the user see: how many displayed frames show a stale page, a stretched canvas, or a bare strip, and how wide? | The one score every fix is judged by |
| Q6 | On a tab switch, how many host resizes happen, how long until the first frame at the final size and scale, and what is on screen in between? | Hold-until-painted, and applying viewport and scale together |
| Q7 | What does a resize cost in imports, IOSurfaces held and memory? | Whether size changes need pooling or stepping |

## The instrumentation

Four pieces. The first two answer most of it and are cheap.

### 1. The resize ledger

One event log for the three clocks, written to a JSONL file when `SPECULAR_RESIZE_TRACE=path` is set. Timestamps from one monotonic clock. A fixed-size ring written out at the end of the run, so logging does not add I/O to the thread being measured.

Every viewport request gets a sequence number. Everything downstream carries it.

| Event | Where | Fields |
|---|---|---|
| `live.begin` / `live.end` | `viewWillStartLiveResize` / `viewDidEndLiveResize` on `SpecularCanvasView` | |
| `win.size` | `setFrameSize:` override on `SpecularCanvasView` (our class, calls super) | points |
| `slot` | `Canvas::set_slot` | origin, size |
| `ask` | `CefPageSource::set_viewport` and `set_texture_scale` | page, seq, CSS size, scale, texels |
| `cef.view_rect` | `PageRenderHandler::view_rect` | page, size returned. First call after an `ask` is CEF picking it up |
| `cef.paint` | `paint::on_accelerated_paint` | page, coded size, `source_size`, `content_rect`, `capture_counter`, `timestamp`, matched seq or stale, and whether `content_rect` is smaller than the coded size (a letterboxed frame) |
| `cef.drop` | texture cap reached | page |
| `pump` | `pump_timer::pump` | start, duration, and the delay CEF asked for in `schedule` |
| `tick` | `on_display_link` | the link's `targetTimestamp`, lateness |
| `frame` | `Canvas::run_frame` | start, end, surface size, whether `sync` reconfigured, drawable wait, stage times, and per page: wanted CSS size, drawn CSS size, seq drawn, frame age |
| `import` | `ingest_shared` | page, hit or miss, ms |
| `gpui` | canvas slot prepaint and paint closures | start of each. Bounds GPUI's layout and paint from the inside |
| `loop` | a `CFRunLoopObserver` in common modes | busy span of each main run-loop turn |

A small `specular-bench resize-report` reads the file and prints one line per run:

- steps asked, frames painted at the asked size, asks never answered (Q3)
- p50, p95, max of ask to pickup, pickup to paint, paint to drawn (Q2)
- canvas frames presented, share that drew a stale page, widest bare strip in CSS px
- `win.size` to the first `frame` at that surface size, p50 and max (Q1, in time)
- main thread busy share, split into GPUI, frame, pump, other (Q4)
- imports, misses, ms in import, most surfaces held (Q7)

The stale and bare numbers here are what we drew. Whether they match what reached the screen is piece 3's job.

### 2. A probe page and a scripted drag

**The page.** `fixtures/probe/resize.html`, served from a `file://` URL. It paints its own size where a pixel scan can read it:

- a 4 px magenta bar pinned to the right edge and a 4 px cyan bar pinned to the bottom, so the page's idea of its width and height is a pixel position, and a stretched frame shows as a bar that is not 4 px
- a requestAnimationFrame counter drawn as a row of black and white cells, so two captured frames can be told apart and ordered
- a checkerboard body, so scaling shows as moiré and a bare strip shows as a flat colour
- `window.__probe` with the `innerWidth` and time of each `resize` event, readable over CDP, which gives the renderer's clock for Q2

Two heavier variants for realism: the same page with a 2,000-node flex layout so reflow costs something, and one real site.

**The drag.** A new script step in `debug_input`, next to `pan`:

```
resize-drag DX DY MS      # press the bottom-right corner, move by DX,DY over MS, one event a refresh, release
```

The memory note already says a `press` on the corner followed by `drag-to` starts AppKit's real live-resize loop, and that `resize W H` does not. This makes it continuous. Scenarios, each a script under `fixtures/scenarios/resize/`:

| Scenario | What it exercises |
|---|---|
| `drag-slow` | 200 px/s, width only |
| `drag-fast` | 1,500 px/s, both axes |
| `drag-wiggle` | back and forth across 100 px, to see queued resizes |
| `tab-switch` | canvas at 25% to a page tab in Fill and back, ten times |
| `lens-switch` | Fill to Device to Fill |
| `sidebar` | Cmd+B with a page in Fill |
| `canvas-only` | `drag-fast` on a canvas with no pages. Isolates Q1 |

`fixtures/scenarios/resize/run.sh LABEL` runs them against a release bundle into `runs/resize/LABEL/` and prints the table. Every fix is a before and after of that table.

### 3. What reached the screen

The ledger says what we drew. Core Animation decides what the user saw, and it scales our layer without telling us. So capture the window from outside.

- A debug overlay behind `SPECULAR_DEBUG_EDGES=1`: the compositor draws a 2 px line at the surface's right and bottom edges in a colour nothing else uses. A stretched or stale canvas layer then shows as a line that is thick, blurred, or not at the window's edge.
- Record the window during a scenario with ScreenCaptureKit, which gives one image per composited frame with its display time. A short Swift tool, `scripts/capture-window.swift WINDOW_NUMBER SECONDS out/`. The window number is already logged at startup.
- `resize/scan.py` reads each image: the window's width is the image's, the canvas's width is where the overlay line is, the page's width is where the magenta bar is, plus bar thickness and the counter.

Per displayed frame that gives window, canvas and page widths. From them: frames where the canvas trails the window and by how many pixels, frames where the page trails the canvas, frames with stretch, and the time from the last resize step to all three agreeing. This is Q5, the score.

The capture costs GPU time, so run it as a separate pass from the ledger pass and compare only like with like.

### 4. Inside Chromium and inside the main thread

Only when 1 to 3 point there.

- **Chromium trace.** If pickup to paint (Q2) is the big number, record a trace over a scenario with `CefBeginTracing` and `CefEndTracing` and read it in Perfetto. The categories that matter are `gpu.capture`, `viz`, `cef`, `ui` and `disabled-by-default-viz.surface_id_flow`. Counting `FpsRateLimited` and `PipelineLimited` against `Capture` per resize says whether the capturer is what throws frames away.
- **Signposts.** Emit an `os_signpost` interval for each `frame`, `pump` and GPUI slot paint. Then `xctrace record --template 'Time Profiler'` over a scenario shows the main thread's samples under those intervals, in a release build with symbols. This answers Q4 below the tenant level.

## What each answer would lead to

So the measurements are aimed at decisions. None of these is chosen.

| If the numbers say | Then try |
|---|---|
| Canvas trails the window with no pages (Q1) | Draw the canvas frame inside the resize step with `presentsWithTransaction`. wgpu-hal already reads the layer's flag at acquire and then waits and presents by hand (`wgpu-hal metal/mod.rs:814`), so it is a property on our layer plus a frame from the slot's prepaint. Set `contentsGravity` to top-left so a late frame is cropped, not scaled |
| Most of Q2 is waiting for a pump | Pump at once after `was_resized` and after each resize step. Drop the 30 Hz fallback during a gesture |
| Most of Q2 is inside Chromium, asks go unanswered (Q3) | CEF already runs one resize at a time and skips the sizes in between, so coalescing on our side buys nothing. What is left: raise the page's paint rate to the display's during a resize (Fill paints at 60 on a 120 Hz display, and a frame the capturer rejects is retried two periods later), call `Invalidate` from the paint handler as cefclient does for CEF issue 3929, and make sure a pump follows every paint so the hold is released at once. External begin frames only time the display compositor. They do not give a frame per resize |
| Paint to drawn is large | Wake a frame from `OnAcceleratedPaint` and stop waiting for the tick |
| GPUI owns the main thread (Q4) | Freeze the Kit's models during a live resize, as `ModelGate` already does for pans |
| Stale page frames dominate Q5 whatever we do | Change what a stale frame looks like. Fill the bare strip with the page's own background colour, as a browser's gutter does. Or hold the page's rect at the painted size for a bounded time |
| Tab switch is two resizes (Q6) | Send viewport and scale in one call. Size the page before showing its tab, and hold the switch until the frame at the final size lands, with a deadline |
| Imports are the cost (Q7) | Ask CEF for sizes in steps during a drag and crop, so the pool and the cache survive |

## Order of work

1. Ledger events and `resize-report`. No behaviour change.
2. Probe page, `resize-drag` step, the seven scenarios, `run.sh`.
3. Baseline on a release bundle. Read the table and rank the six suspects above.
4. Edge overlay, capture tool and `scan.py`. Baseline Q5.
5. First fix, chosen by the baseline. Rerun. Keep or revert on the numbers.
6. Chromium trace and signposts only if step 3 leaves Q2 or Q4 unexplained.

## Conditions for a number worth keeping

From earlier bench work on this machine:

- Release bundle. Check `ps` for `target/debug` first.
- Quiet machine. The load average was 31 when this plan was written, from other agents building. A run under that is noise.
- Display awake, window uncovered and floated (`SPECULAR_FLOAT_WINDOW=1`), power connected.
- Three runs a scenario. Report the median and the worst.
- Do not use the app in the herdr pane for this. Launch a second bundle on a scratch space.

## Open

- Whether `capture_counter` and `source_size` are filled in on macOS in CEF 154. Step 1 will show.
- Whether our `CADisplayLink` fires during the live-resize loop at the display's rate, or only when GPUI lets the run loop turn. The `tick` lateness field will show.
- Whether a scripted drag on the resize corner enters the live-resize loop on every run. The `live.begin` event will show.
- wgpu gives no way to read when a drawable reached the screen (wgpu issue 9856). The capture pass stands in for it.

## External findings

Read from CEF, Chromium, Electron, Zed, Flutter and Ghostty source on 2026-10-09 by a research agent. I have not checked each line myself. Line numbers move.

### CEF holds one resize at a time, on purpose

`CefRenderWidgetHostViewOSR` in `libcef/browser/osr/render_widget_host_view_osr.cc`:

- `WasResized` reads `GetViewRect`. If the size changed it sets `hold_resize_`.
- While the hold is set, every further `WasResized` only sets `pending_resize_`. Any number of calls collapse into one flag.
- The hold is released after an `OnAcceleratedPaint` whose pixel size equals the view size times the scale. Then CEF posts one more `WasResized` as a UI-thread task, which reads `GetViewRect` again at that moment.

So the "one step in three" from `cb426e89` is the design, not a fault. A resize is one full trip through the renderer, and the sizes asked for during the trip are skipped. Two things follow for us.

- The release and the follow-up are UI-thread tasks, so they wait for our pump. A late pump stretches every trip.
- Our texture cap drops a paint without telling CEF, which is fine for the hold. The callback still returns.

Chromium's `RenderWidgetHostImpl::SynchronizeVisualProperties` has a second one-at-a-time gate (`visual_properties_ack_pending_`), which the hold already covers.

### The capturer has its own rate limit

- `windowless_frame_rate` sets the capturer's minimum capture period. CEF's default is 30. We set it per page from the LOD tier: 60, 30 or 15.
- A frame the capturer rejects for rate is retried after twice the period.
- Between the renderer activating at the new size and the capturer taking the new size, frames can arrive letterboxed: new buffer, old content, bands. CEF issue 3929 describes it. `content_rect` against `coded_size` identifies them. These do not release the hold.

### External begin frames do not help

`SendExternalBeginFrame` drives the display compositor's tick. The renderer round trip, the hold and the capturer's limit are unchanged.

### The IOSurface is only promised during the callback

`cef_render_handler.h` says the handle cannot be used outside the callback and its contents should be copied. We retain it and mark it in use, which `specular-cef/src/iosurface.rs` already calls a bet on Chromium internals. The capture pool is 11 frames. A resize rebuilds it. If the pixel scan ever shows torn or half-painted page frames, look here first.

### What a browser does about the same lag

Chromium does not wait for the page on macOS. `BrowserCompositorMac` uses a resize deadline of zero, which its own comment says "immediately shows stale content". `DelegatedFrameHost::EmbedSurface` draws the stale frame unstretched and fills the uncovered area with a gutter colour taken from the page's background.

We already draw the stale frame unstretched from the top-left. We do not fill the strip. That is the cheapest visible improvement on the list, and it is what every Chromium window does.

Electron's offscreen view has the same flags but only holds around its software composite, so it accepts the next resize without waiting for a frame of the last one.

### The canvas layer

- GPUI's view uses `NSViewLayerContentsRedrawDuringViewResize` and draws in `displayLayer:` with `presentsWithTransaction` on. Confirmed in the pinned `gpui-pre-macos` 0.3.8 as well.
- wgpu has no API for this, but its Metal backend reads the layer's `presentsWithTransaction` when it acquires a drawable and then commits, waits until scheduled and presents by hand. Setting the property on our own layer works with `queue.present` as it is. wgpu issue 9828 describes a race when turning it off at the end of a live resize.
- Flutter's macOS embedder blocks the resize on the raster thread with a timeout (`FlutterResizeSynchronizer`).
- Ghostty sets its layer's gravity to top-left "so that the contents aren't stretched during resize operations before a new frame has been drawn".

Suspect 1 at the top of this plan is what GPUI, Flutter and Ghostty each wrote code to avoid.

### Not confirmed

- Whether `NSView.displayLink` fires during the live-resize loop, and at what rate.
- Which CEF version added the extra paint-info fields, and whether macOS fills them.
- The Instruments and `xctrace` details in piece 4.
