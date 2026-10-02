# Rust + CEF shell spike

**Status:** In progress — scaffold landed on branch `claude/rust-cef-spike`; code lives in [`native/`](../../native/README.md).
**Related:** [ADR 0038 — Offscreen GPU-texture compositing for live pages](../adr/0038-offscreen-texture-canvas-for-live-pages.md) (the Electron architecture this is measured against), [ADR 0023](../adr/0023-renderer-owned-camera-gpu-panzoom.md) (rejected; why "move the camera" alone was not the win), [`perf-tracing.md`](../perf-tracing.md), [`perf-zoom-pan-log.md`](../perf-zoom-pan-log.md).

## Question

Would rebuilding Specular's shell in Rust, with Chromium embedded via CEF
offscreen rendering, beat the current Electron app on **pan/zoom frame times**,
**memory**, and **input latency**?

This is a measurement vehicle. It builds only what the measurement needs: a
window, a camera, a compositor that draws page textures over a dot grid, a
CEF page host per page, and a bench. No chrome, tools, persistence UI,
agents, or entity kinds beyond pages.

## Hypothesis

ADR 0038 already removed the native-compositor backpressure that scaled with
page count: every page is an offscreen `BrowserWindow` painting shared
textures, composited in canvas-bg. What remains between a page's paint and the
screen in Electron is:

1. main process receives the paint, imports the texture, and posts it to
   canvas-bg over IPC (one hop per frame per page, `MAX_OUTSTANDING_TEXTURES = 6`);
2. canvas-bg (a renderer with its own JS heap, Blink layout, and a GPU
   process round trip) draws every texture into a `<canvas>`;
3. the canvas-bg surface is itself composited by the browser's viz.

A Rust shell collapses that to: CEF delivers an IOSurface on the main thread,
wgpu samples it directly (IOSurface -> `MTLTexture` -> wgpu-hal Metal texture,
no copy), one render pass, present. We expect:

- **Frame times:** no worse at 9/20 pages (Electron is already pinned at 120 fps
  on static pages) and measurably better at 40 pages and under animated
  content, where Electron's per-frame IPC + canvas-bg draw is on the critical path.
- **Memory:** lower total RSS — no canvas-bg renderer, no Electron main JS heap,
  no separate toolbar/sidebar renderers. CEF's per-page renderer processes
  cost the same in both, so the delta is a fixed overhead, not per page.
- **Input latency:** lower for input forwarded into a page (one in-process
  call to `SendMouseClickEvent`/`SendKeyEvent` vs. renderer -> IPC -> main ->
  `sendInputEvent`) and for gesture-to-present (camera update and draw in the
  same thread and frame as the wheel event).
- **Capability gaps closed:** `<select>` popups render (CEF `OnPopupShow` /
  `OnPopupSize` / `PET_POPUP`), and IME shows live composition
  (`ImeSetComposition`) — both missing from Electron OSR (ADR 0038).

## What is measured

All runs: release builds, the same Apple Silicon Mac, the built-in 120 Hz
display, the same window size, power adapter connected, no other apps.

| Metric | Rust spike | Electron (main) |
|---|---|---|
| Presented-frame interval per profile: `drawFps`, mean, p50, p95, p99, max, `longFrames` (> 1.5 x refresh budget) | `specular-bench` timing each `present` during the six profiles | `/perf/pan-zoom/run` trace, intervals between `Display::DrawAndSwap` on the browser's viz thread (same summary computed by a script over the trace) |
| Frames without a texture (`framesWithoutTexture`) and peak outstanding textures per page | compositor `RenderStats` | canvas-bg texture-surface counters |
| Total RSS (all processes: browser + GPU + every renderer + helpers), idle and at the end of a run | `ps`/`footprint` summed over the process tree | same, over the Electron process tree |
| Forwarded-input latency: pointer down / key down -> fixture page repaints -> frame presented | timestamp at `send_input`, fixture page flips colour on the event, timestamp at the first `present` carrying the new colour | same fixture via the Electron page-input path, frame timestamps from the trace |
| Gesture latency: wheel event -> presented frame with the new camera | event timestamp to `present` | `InputLatency::*` / `EventLatency` slices in the trace |
| Capabilities: `<select>` popup visible and positioned, IME marked text visible | manual check on fixture pages | known gaps (ADR 0038) |

## Fixtures

- **Static:** the same real sites the ADR 0038 lab used (Wikipedia, GitHub,
  MDN, HN, docs sites), at 9, 20, and 40 pages, 1280x800 CSS each.
- **Animated:** 20 copies of a `requestAnimationFrame` counter + CSS-animated
  element page (the ADR 0038 animated-content fixture), uncapped.
- **Input:** one page that flips its background on `pointerdown` and `keydown`.
- Both shells load the same `.canvas` file per fixture (JSON Canvas `link`
  nodes; the Rust app takes the path as its first argument).

## Comparison method

1. Electron: open the fixture canvas, run `POST /perf/pan-zoom/run` with
   `{"summarize": true}` (profiles from `src/shared/pan-zoom-perf-test.ts`,
   input stepped at the display refresh interval, camera anchored at the canvas
   centre). Three runs; keep the trace files.
2. Rust: `specular-bench` drives the same six profiles through
   `Camera::apply_input_delta` — the same `zoom -= deltaY * 0.002` math, the
   same step expansion (`build_steps` is a port of `buildPanZoomPerfSteps`),
   the same 250 ms phase gap. Three runs.
3. Report the median of three runs per cell. A difference smaller than the
   run-to-run spread is "no difference".
4. Results from the synthetic source or any CPU (`OnPaint`) frame path are
   invalid for comparison and are not entered below.

## Success / fail criteria

Decided per metric against Electron at the same fixture:

- **Win:** at 20 animated pages, p95 frame interval within 1.1 x the refresh
  budget with `longFrames` <= Electron's, **and** at least one of: >= 25% lower
  total RSS, or >= one refresh interval lower p50 forwarded-input latency.
- **Neutral:** frame times equal within noise and memory/latency gains below
  those thresholds — the rewrite cost is not justified by performance alone;
  capability gains (popups, IME) are then the only argument.
- **Fail:** any frame-time metric worse than Electron beyond noise at 9 or 20
  pages, or the zero-copy IOSurface path cannot be made to work (a spike that
  only runs the CPU path has not answered the question).

## Results

Fill one row per fixture x shell (median of three runs). Frame columns are the
worst profile; attach the per-profile JSON next to this doc.

| Fixture | Shell | drawFps | mean ms | p95 ms | max ms | longFrames | no-texture frames | RSS idle (MB) | RSS run (MB) | input p50 ms | gesture p50 ms |
|---|---|---|---|---|---|---|---|---|---|---|---|
| static x9 | Electron | | | | | | | | | | |
| static x9 | Rust/CEF | | | | | | | | | | |
| static x20 | Electron | | | | | | | | | | |
| static x20 | Rust/CEF | | | | | | | | | | |
| static x40 | Electron | | | | | | | | | | |
| static x40 | Rust/CEF | | | | | | | | | | |
| animated x20 | Electron | | | | | | | | | | |
| animated x20 | Rust/CEF | | | | | | | | | | |

Capabilities:

| Check | Electron | Rust/CEF |
|---|---|---|
| `<select>` popup renders and is positioned | no (ADR 0038) | |
| IME marked text during composition | commit-only (ADR 0038) | |

## Verdict

_Pending results._
