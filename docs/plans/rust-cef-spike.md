# Rust + CEF shell spike

**Status:** Built, awaiting the first macOS run — all five crates on branch `claude/rust-cef-spike`; code lives in [`native/`](../../native/README.md). See [Implementation status](#implementation-status) for what is verified and what is not, and [`native/README.md` → Morning run](../../native/README.md#morning-run-on-macos-apple-silicon) for the copy-paste sequence.
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
- **Memory:** lower total footprint — no canvas-bg renderer, no Electron main JS heap,
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
| Total memory (all processes: browser + GPU + every renderer + helpers), idle and peak during a run | physical footprint (`proc_pid_rusage`, includes IOSurface/GPU memory) summed per pid over the `ps` process tree; RSS alongside as a cross-check | same, over the Electron process tree |
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
  nodes; the Rust app takes the path as its argument). They live in
  [`native/fixtures/`](../../native/fixtures/): `static-9`, `static-20`,
  `static-40`, `animated-20`, `input`. The animated and input pages are
  `data:` URLs, so no server is needed.

## Comparison method

1. Electron: open the fixture canvas, run `POST /perf/pan-zoom/run` with
   `{"summarize": true}` (profiles from `src/shared/pan-zoom-perf-test.ts`,
   input stepped at the display refresh interval, camera anchored at the canvas
   centre). Three runs; keep the trace files.
2. Rust: `specular-app --bench all` drives the same six profiles through
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
  total footprint, or >= one refresh interval lower p50 forwarded-input latency.
- **Neutral:** frame times equal within noise and memory/latency gains below
  those thresholds — the rewrite cost is not justified by performance alone;
  capability gains (popups, IME) are then the only argument.
- **Fail:** any frame-time metric worse than Electron beyond noise at 9 or 20
  pages, or the zero-copy IOSurface path cannot be made to work (a spike that
  only runs the CPU path has not answered the question).

## Implementation status

Built in a Linux container with no GPU, no display, and no access to the CEF
download host. "Verified" below means exactly what it says; nothing has run
on a Mac or against real Chromium.

**Verified here (Linux x86_64):**

- Whole workspace: `cargo fmt --check`, `cargo clippy --workspace
  --all-targets -D warnings`, `cargo test --workspace`, `cargo doc
  --workspace --no-deps` with `-D warnings`, all clean.
- `specular-core`: `.canvas` load -> save gives back the same JSON value
  (fixtures copied from the integration snapshots and the starter space);
  move/resize/add/remove each undo to the original; camera round trips and
  anchored zoom.
- `specular-compositor`: GPU tests render and read back pixels on a
  software Vulkan adapter (lavapipe): grid, page colour, rounded corners,
  dirty-rect upload, popup show/hide, page removal, bad-frame rejection,
  per-render frame and dropped-frame counters. They skip cleanly where no
  adapter exists (CI).
- Pure logic added in the review pass, unit-tested: the per-surface import
  cache (LRU, idle and resize eviction), the Electron paint-LOD port
  (frame-rate and texture-scale tiers with hysteresis, settle waits,
  off-screen culling), per-button pointer capture, typed `BenchLine`
  round trips through `assemble`, footprint summing, and the Electron
  present-count check.
- `specular-app` end to end with the synthetic source under `xvfb-run`:
  `--bench` prints one line per profile, `specular-bench assemble` and
  `compare` consume them. Numbers are meaningless (software GPU, CPU frames).
- All five bench fixtures under `native/fixtures/` parse and load.

**Type-checked only (`cef-dox`, Linux and `aarch64-apple-darwin`, clippy
`-D warnings`), never linked or run:**

- Every CEF call in `specular-cef` (listed in its README), the `--features
  cef` path in `specular-app`, the macOS IOSurface -> `MTLTexture` -> wgpu
  import in `specular-compositor`, the `CefAppProtocol` patch of winit's
  `NSApp` (`app_protocol.rs`), and the macOS `proc_pid_rusage` footprint
  sampler in `specular-bench`.
- `crates/specular-cef/scripts/bundle-macos.sh` (syntax-checked only).

**Open questions the first macOS run must answer:**

1. Does CEF start under winit's `NSApplication` once `app_protocol` has
   added `CefAppProtocol` to it, and does a `<select>` open without an
   "unrecognized selector" crash (see `crates/specular-cef/README.md`)?
2. Does Metal accept the sRGB view of CEF's BGRA IOSurface, and is retaining
   the IOSurface (instead of copying in the callback) tear-free?
3. Does the per-surface import cache hit (misses near the pool size, logged
   at exit), or does wrapping a surface in a Metal texture stop Chromium
   recycling it?
4. Is a 120 fps `windowless_frame_rate` honoured with shared textures?
5. Do Electron traces hold about steps + 1 `Display::DrawAndSwap` presents
   per phase? If offscreen page windows' draws are counted too,
   `electron-trace` notes it and the Electron frame numbers are void.

**Measurement gaps (known, not yet built):**

- Rust `--bench` reports frame timing, `drawsWithoutTexture`,
  `maxPaintToSubmitMs` and per-phase texture counters (`framesReceived`,
  `popupFrames`, `framesDroppedForPoolPressure`, outstanding textures) per
  profile, but no input latency (the bench forwards no page input).
  Forwarded-input latency comes from an interactive session's
  `inputLatency` line; it counts from the forwarded event to the first
  presented repaint of that page, a lower bound rather than the plan's
  colour-flip check. Gesture (wheel -> present) latency is not measured in
  either shell.
- Electron presents are every `Display::DrawAndSwap` on the viz thread,
  which may include offscreen page windows' displays (open question 5).
- Electron phases are recovered from quiet gaps in the trace, so animated
  fixtures need one profile per request.
- The Electron and Rust start cameras are matched by hand.
- Whether the Electron app loads `data:` URLs from `link` nodes (the
  `animated-20` and `input` fixtures) is unchecked; if not, serve the two
  pages from a local static server and edit the URLs in both shells' copies.
- The Rust shell ports Electron's paint LOD (frame-rate tiers, texture-scale
  tiers after settle, off-screen culling) and records the policy in every
  report; the agent/drag/focus-session full-rate exceptions are not ported
  (the bench uses none of them). Texture-scale LOD lowers CEF's device scale
  factor, which a page can observe as `devicePixelRatio`, where Electron
  keeps it. The compositor has no mipmaps, so far-out zoom shimmers
  (quality, not timing).

## Results

Fill one row per fixture x shell (median of three runs). Frame columns are the
worst profile; attach the per-profile JSON next to this doc.

| Fixture | Shell | drawFps | mean ms | p95 ms | max ms | longFrames | no-texture frames | footprint idle (MB) | footprint peak (MB) | input p50 ms | gesture p50 ms |
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
