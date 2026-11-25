# ADR 0053: Page frames follow the display, and the canvas pass loses multisampling

**Status:** Proposed. Not built. Items 1 to 3 are decided on what the code shows; items 4 and 5 wait on numbers the spike takes ([`docs/plans/rune-spike.md`](../plans/rune-spike.md)).
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
3. **The canvas pass is single-sampled.** Meshes get analytic edges: feathered geometry from lyon, or strokes as SDF segments. The multiply-blend mesh pipeline is kept. This is a rendering rule a snapshot cannot see, so it ships with a GPU readback.
4. **App-driven begin frames are tried, not adopted.** `external_begin_frame_enabled` with `SendExternalBeginFrame` from our display tick would align a page's frames to ours and replace the rate tiers. It is adopted for the entered page if the spike shows lower input-to-pixel latency and no dropped frames on our CEF build. It does not replace `WasHidden`, which is also the page's lifecycle signal, or the texture tiers.
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

## Consequences

- The page a user is in runs at the display's rate.
- The canvas pass uses nothing Rune's Metal backend lacks.
- Idle wakes fall: the paint wake replaces two polls.
- Analytic mesh edges are new code in a place where 4x multisampling was hiding the question.
- The begin-frame and host-process choices are made on recorded numbers, and this ADR is amended with them.
