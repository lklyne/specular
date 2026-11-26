# ADR 0053: Page frames follow the display, and the canvas pass loses multisampling

**Status:** Proposed. Items 1 and 2 and the smaller fixes are built (follow-ups plan, Phase 2 and Phase 3 logs). Item 1 asks for the display's rate, but CEF 154 still paints a windowless page at 60: its begin frames come 16.67 ms apart whatever `SetWindowlessFrameRate` says, so the page a user is in reaches 120 only through item 4 or a later CEF. Item 4 was tried and not adopted (numbers below). Item 3 is not needed while the shell stays on wgpu (see item 3); item 5 waits on its threshold, which the spike did not reach.
**Date:** 2026-11-25
**Related:** [ADR 0038](./0038-offscreen-texture-canvas-for-live-pages.md), every page a texture. [ADR 0039](./0039-rust-canvas-render-stack.md), the canvas pass. [ADR 0045](./0045-item-view-as-session-state.md), the Fill lens. [ADR 0035](./0035-idle-cpu-throttling-for-pages.md), the tiers this keeps for pages that are not entered. [ADR 0051](./0051-rune-shell-around-the-compositor.md).

## Context

The canvas is cheap: a pan frame over 20 pages and 300 items costs 0.42 ms of CPU. The page path is where time goes, and most of it was ported from Electron to hold the bench level.

- **No page is asked for more than 60 fps.** `FULL_FRAME_RATE` is 60 and the only caller of `set_frame_rate` is the LOD, so the entered page and a page in Fill paint at half a ProMotion display's rate. CEF's clamp already allows 120.
- **A paint does not wake the canvas.** `OnAcceleratedPaint` pushes onto a thread-local queue that is drained on the next display tick: up to 8.3 ms, or up to 50 ms once the link has paused.
- **Latency was never measured.** The `inputLatency` probe stops at the return of the present call and no value is recorded. Read from the code, a key in an entered page is 4 to 7 display frames from the screen.
- **4x multisampling serves lyon meshes only.** Page quads and SDF shapes anti-alias in the shader and glyphon does not need it. Rune's Metal backend has no multisampling (ADR 0051).
- **The whole browser side runs on the main thread.** macOS CEF has no multi-threaded message loop, so every page's paint callback, devtools answer and CDP message shares the thread that draws. The pump's duration has never been recorded.
- **We hold CEF's IOSurface after the callback.** CEF documents the surface as valid only during `OnAcceleratedPaint`. `iosurface.rs` retains it and raises its use count, relying on Chromium's pool checking `IOSurfaceIsInUse`. That is Chromium's behaviour, not CEF's contract; the module says so.

## Decision

1. **The entered page and a page in Fill paint at the display's rate.** One more input to `PageLod::update`. Other pages keep the 60/30/15 tiers and `WasHidden` off screen.
2. **A paint wakes the canvas.** The page source signals the loop when a frame is queued. The 30 Hz pump fallback and the 50 ms idle turn stop being what delivers a frame.
3. **The canvas pass is single-sampled.** Meshes get analytic edges: feathered geometry from lyon, or strokes as SDF segments. The multiply-blend mesh pipeline is kept. This is a rendering rule a snapshot cannot see, so it ships with a GPU readback. *Not adopted, 2026-11-25:* built on the unmerged branch `claude/analytic-edges` (feathered edges, a readback test), it matched 4x but raised pan frame CPU from about 1.25 to 1.7 ms. 4x multisampling only touches mesh edges (pages, text and SDF shapes draw the same at 1x), its attachment is transient and so takes no app memory on Apple GPUs, and S6 found no GPU time difference. Its reason was Rune's Metal backend, which is deferred with ADR 0051. Revisit with that backend, or on a platform where the 4x attachment is not free.
4. **App-driven begin frames are tried, not adopted.** `external_begin_frame_enabled` with `SendExternalBeginFrame` from our display tick would align a page's frames to ours and replace the rate tiers. It is adopted for the entered page if the spike shows lower input-to-pixel latency and no dropped frames on our CEF build. It does not replace `WasHidden`, which is also the page's lifecycle signal, or the texture tiers. *Tried 2026-11-25 (follow-ups plan, Phase 3):* every page created with `external_begin_frame_enabled` and sent `SendExternalBeginFrame` at most every 8 ms from the pump. The entered animated page painted 115 times a second (60 without), with 30 of 482 intervals over 12 ms, but key to present on the flip page rose from p50 7.0 / p95 8.9 ms to 21.8 / 25.8 (three runs each): a key's paint waits for begin frames. CEF 154.0.34 was built 2026-10-04, before the 2026-10-23 begin-frame fixes. Not adopted. Retrying needs a CEF with those fixes, a sender on the display tick rather than the pump, the begin-frame flag on every page (it is set when a browser is made), the stuck-ack watchdog and a floor rate for pages with a CDP client.
5. **A page-host process is decided by the pump's share of the main thread.** If `CefDoMessageLoopWork` takes more than about 1 ms of a frame at p95 with 40 pages, CEF moves behind `PageSource` into a helper process that sends IOSurfaces as Mach ports. `PageSource` is already fire-and-forget calls and events with request ids. Under that, nothing changes.
6. **The surface-lifetime bet stays, written down.** If a run shows tearing, the fallback is the documented one: copy inside the callback into a texture we own. CEF PR 4358 adds a release callback; when it ships we move to it.

Smaller fixes that go with these: at the six-frame cap the newest paint is dropped where the oldest should be; the retire callback is registered only if something is already retired at submit, which can stall leases under an occluded window; pages keep painting while the window is occluded; a pointer move in an entered page owes a frame and a 250 ms tail of forced ones; the LOD runs twice a turn.

## Alternatives

**Leave the page path as ported.** It holds 120 fps for the canvas. It leaves the page a user is typing into at 60 with a queue wait.

**A native view for the entered page.** The lowest latency, and a second path for layering, input and the Fill lens. ADR 0038 exists because one path was worth more.

**Another engine.** WKWebView gives no texture; Servo's web compatibility is short of arbitrary sites; a CDP screencast is encoded images. CEF offscreen is still the only route to Chromium fidelity as GPU textures.

## What was read, and what is not known

From `specular-cef`, `specular-compositor` and `specular-shell` at `d375c2ee`, and CEF master's source and issue tracker. Nothing was run.

- CEF's begin-frame timing was fixed on master on 2026-10-23 (`ff683da`, `098737a`). Before it, one user measured up to 54% dropped frames on macOS arm64. Whether cef 154 has the fixes is unchecked. A stuck-ack fix (PR 4312) is unmerged, so a watchdog is needed.
- With begin frames the capturer's rate still comes from `windowless_frame_rate`, which must be set to the display's rate as well.
- A page that gets no begin frames runs no `requestAnimationFrame` and no CSS animation. A page with a CDP client attached needs a floor rate.
- Not measured anywhere: the pump's duration, GPU time in a window, input to scan-out, the cost of an import miss, 40 CEF pages under either shell on a quiet machine, a 5K target.

## Measured

Step S6 of [the Rune spike](../plans/rune-spike.md), 2026-11-25: release builds, an M3 Max with its ProMotion display, real CEF 154 pages (live sites), a 6 s pan in a 1600x1000 window at scale 2, three runs each on the Kit shell and the Rune spike. The machine was not quiet. These are the numbers items 4 and 5 wait on; the decisions above are unchanged.

- **`CefDoMessageLoopWork` during a pan** (the ledger's `Pump` entries; pumps under 50 µs are only counted, and taken as 25 µs). Kit, then Rune: mean 0.09 / 0.07 ms at 9 pages, 0.06 / 0.07 at 20, 0.05 / 0.05 at 40; p95 0.25 / 0.18, 0.19 / 0.18, 0.14 / 0.12 ms; 11 to 31 ms of pumping a second; single pumps up to 7 to 10 ms a few times a run. The pump time between two frames at p95 is 0.38 / 0.29, 0.29 / 0.34 and 0.39 / 0.30 ms. Item 5's 1 ms at p95 with 40 pages is not reached.
- **Input to present, entered page, 60 fps windowless** (the `inputLatency` probe: forwarded input, the page's next paint, the frame that drew it; no screen recording): Kit p50 8.4 ms, p95 25.7. Rune, which wakes on paint, p50 12.1, p95 14.5; at 120 fps p50 12.7, p95 14.3, so 120 did not lower it there. The Kit's endpoint is after its present returns and Rune's before Rune presents. The Kit at 120 needs `FULL_FRAME_RATE` changed; begin frames (item 4) were not tried.
- **Paint to present with a wake on paint** (Rune): p50 5.6 ms, p95 8.2 at 60 fps. Without one was not measured: in Rune nothing else wakes an idle window.
- **GPU time a frame, 4x against 1x** (Rune's device, waiting on the GPU from our submit to done, 20 pages): 1.91 ms [1.81 to 2.32] against 1.99 [1.88 to 2.01]. The method does not resolve a difference; Metal timestamp queries need a device feature the host did not ask for.

## Consequences

- The page a user is in runs at the display's rate.
- The canvas pass uses nothing Rune's Metal backend lacks.
- Idle wakes fall: the paint wake replaces two polls.
- Analytic mesh edges are new code in a place where 4x multisampling was hiding the question.
- The begin-frame and host-process choices are made on recorded numbers, and this ADR is amended with them.
