# ADR 0039 — Render and UI stack for the Rust canvas

**Status:** Proposed. Measured headless on one machine. The "Needs a human at a Mac" list below is the gate to Accepted.
**Date:** 2026-10-07
**Related:** [Rust native rebuild plan](../plans/rust-native-rebuild.md), task M1 and its "Rendering and text" section. [ADR 0038](./0038-offscreen-texture-canvas-for-live-pages.md), which made every page a texture.
**Code:** `native/bakeoff/`, a throwaway cargo workspace outside `native/Cargo.toml`. Golden images are in `native/bakeoff/golden/`.

## Decision

1. **Canvas items.** Draw them in the compositor's own wgpu pass. Rounded rects stay on the SDF shape layer, text goes through glyphon 0.12 on cosmic-text 0.19, and strokes and arrows are tessellated by lyon 1.0.
2. **Panels.** Use egui 0.36 with egui-wgpu, painted in a second pass over the compositor's target.
3. **GPUI.** Do not adopt it, for panels or for canvas items.
4. **Vello.** Do not adopt it now. Keep `Scene` free of renderer types so one item kind can move to vello later if it needs gradients, blurs or clips.

The request was a UI built on wgpu. Decisions 1 and 2 are that: one wgpu 30 device, one window, one pass the app owns. GPUI is the only candidate that is not wgpu on macOS.

## What builds against wgpu 30

Checked on crates.io on 2026-10-07. Everything resolves to a single wgpu 30.0.1 with winit 0.30.13 in one lockfile.

| Crate | Version | wgpu requirement | Result |
|---|---|---|---|
| vello | 0.11.0, 2026-10-02 | `^30.0.0` | builds and runs |
| parley | 0.11.1 | none | builds |
| glyphon | 0.12.0 | `^30.0.0` | builds and runs |
| cosmic-text | 0.19.0 | none | builds |
| lyon | 1.0.19 | none | builds |
| egui, egui-wgpu, egui-winit | 0.36.2 | `^30.0`, winit `^0.30.13` | builds and runs |
| vello_hybrid | 0.2.0 | `^29.0.3` | not usable |
| gpui | 0.2.2, 2025-10-22 | none, Metal on macOS | builds with `runtime_shaders` only |

Vello reached wgpu 30 five days before this test. Version 0.10 was on wgpu 29 until then, so the plan's worry about vello's release schedule was fair. It did not bite today.

## The test

Both candidates draw 500 sticky notes with wrapped 14 px Helvetica, 200 freehand strokes and 100 arrows into a 1600x1000 offscreen target. Apple M3 Max, Metal, optimised build. Each frame is timed from the start of scene building until the GPU finishes it. There is no window and no vsync. "Sweep" changes zoom from 0.02 to 3 and back over 600 frames while orbiting. "Pan" holds one zoom for 120 frames. Both candidates cull to the viewport and cache text layout.

Milliseconds per frame, mean and p95:

| Path | Notes on screen | vello | SDF + glyphon + lyon |
|---|---|---|---|
| Sweep 0.02 to 3 | varies | 9.8 / 18.5 | 7.2 / 13.2 |
| Pan at 0.05 | 500 | 15.0 / 17.3 | 5.4 / 7.0 |
| Pan at 0.25 | nearly all | 11.2 / 14.6 | 4.5 / 6.0 |
| Pan at 1 | about 30 | 2.9 / 4.5 | 1.4 / 2.7 |
| Pan at 3 | about 6 | 2.0 / 3.9 | 1.1 / 2.4 |
| Pan at 0.05, text under 2.5 px skipped | 500 | 2.2 / 4.0 | 2.2 / 3.5 |
| Sweep, text under 2.5 px skipped | varies | 4.1 / 12.4 | 3.8 / 10.1 |

A 120 Hz frame is 8.3 ms, and the pages need most of it. The SDF stack fits every pan. Vello misses whenever a few hundred notes of text are on screen. Neither fits the zoom sweep at p95.

Vello's cost is CPU time. It resolves every glyph outline on every frame, about 8 to 10 ms for the 60,000 glyphs of the full scene, and the cost follows glyph count. Encoding the world once and replaying it under the camera transform made things worse, 10 to 13 ms at every zoom, because it gives up culling and still resolves every glyph.

Glyphon's cost during a zoom is that each new scale is a new glyph size, so it rasterises and lays out quads again. The usual fix is to hold the rasterised size while the gesture runs and re-rasterise when it settles. I did not build that.

Text sharpness, judged from `golden/compare-5x-zoom-*.png` with vello on the left:

- **Zoom 1 and 3.** No difference I can see at 5x magnification. Both are sharp.
- **Zoom 0.25.** Text is 3.5 px and unreadable in both. Vello's is even. Glyphon snaps glyphs to whole pixels, so its lines look uneven.
- **Zoom 0.05.** Text is a tint in both. The strokes differ because I held them at a 1 px minimum in the SDF candidate and not in vello. That is a policy either renderer can apply.

Glue code is 248 lines for vello and 649 for the SDF stack, of which 70 are shader. The SDF total includes two pipelines and the buffer handling that the compositor already has.

## Why not vello

It is the smaller and prettier integration, and it loses on two counts that matter here.

Pages cannot go through it without a copy. `Renderer::register_texture` copies the texture into vello's image atlas at the start of each frame, and it wants `Rgba8Unorm`. So vello would draw canvas items into its own full-window texture, and the compositor would blend that with the page quads. The document has one z-order across pages and items. Every run of items between two pages would then need its own full-window vello render. The SDF stack draws in the same pass as the page quads and can break batches at each page instead.

The second count is the frame time above.

What we give up: painter's order with no batching work, even text at tiny sizes, exact antialiasing of hairlines, and gradients, blurs and clip layers for free.

## egui for panels

egui-wgpu 0.36.2 takes a wgpu 30 device and renders into a pass we begin ourselves with `LoadOp::Load`. That is the "toolbar over the compositor's pass" the plan asked for. A toolbar and a sidebar with a 40-row scroll list cost 0.25 ms of CPU and about 1.1 ms to GPU completion, over either candidate. See `golden/sdf-egui.png`. The panel code is 112 lines and fits the architecture as planned: a function of state that returns events. egui-winit 0.36.2 accepts winit 0.30.13, so the shell keeps its event loop and feeds egui the events it has already routed.

The fallback of HTML panels in CEF is not needed.

## GPUI

Read from gpui 0.2.2 on crates.io and Zed main at `72d073d6`. A proof is in `native/bakeoff/gpui-proof/`.

**Does it have a wgpu renderer on macOS?** No. macOS uses gpui's own Metal renderer, in 0.2.2 and on main, where `gpui_macos` imports `gpui_apple::metal_renderer`. Main has a `gpui_wgpu` crate, pinned to wgpu 29, and only the Linux and web platforms select it. The crates.io release is a year old. Zed has not published the split crates. A third party republishes main as `gpui-pre`.

**Can it show CEF page IOSurfaces zero-copy at 120 fps with 40 pages?** Not as shipped. The `surface` element takes a `CVPixelBuffer`, and `MetalRenderer::draw_surfaces` asserts the format is biplanar YCbCr 4:2:0, the video format. The proof confirmed it both ways. An IOSurface-backed 420f buffer painted correctly. A BGRA one, which is what CEF produces, aborted the process at `metal_renderer.rs:1085`. There is no hook for a custom texture or render pass. The scene's primitives are quads, shadows, paths, underlines, sprites and that surface. Two ways around it exist. One is a fork with a BGRA branch. The other is our own Metal layer under gpui's, which puts every page below every canvas item and loses z-order. The 40-page case was not measured. A new page frame marks the window dirty and gpui redraws the element tree, so page frames and UI layout would share one cost.

**Who owns the event loop, and can CEF live in it?** gpui does. It registers its own `NSApplication` subclass and calls `run`. That class has no `CefAppProtocol` and gpui has no API to swap it. Adding the protocol methods at runtime before `run` should work, and gpui can schedule delayed work on the main thread for CEF's external message pump. Neither was tested. IME is the better half: gpui's `InputHandler` exposes marked text, ranges and candidate bounds, which map onto CEF's composition calls. Key events are the worse half. gpui's `Keystroke` carries a key name and modifiers, with no native key code, and CEF wants the key code. We would also give up winit, since two libraries cannot own `NSApplication`.

**How would text editing and zoomable canvas text work?** gpui ships text primitives and a 746-line example input, with no editor widget. Zed's editor is GPL and separate. The `gpui-component` input is Apache-2.0 but depends on the `gpui-pre` republish. For zoom, gpui has no transform for elements or glyphs. `paint_glyph` passes a unit matrix. Each zoom step is a new font size, which means new shaping, new wrapping and new glyph rasters. It works and looks sharp. The proof held 120 fps with about 250 notes visible and spent 5.8 ms of the 8.3 ms frame doing it, with no pages at all.

**Verdict.** GPUI is a good application framework that assumes it owns the window, the event loop and the renderer. Specular's compositor needs to own all three to put 40 live IOSurfaces in z-order with canvas items. Adopting GPUI means a fork for BGRA surfaces, runtime patching of its application class for CEF, reverse-mapping key codes, and throwing away the working wgpu compositor and winit shell. It still would not be wgpu on macOS. egui on our own pass costs about 1 ms and none of that.

| | Canvas items | Panels | Pages zero-copy | wgpu on macOS | Event loop |
|---|---|---|---|---|---|
| SDF + glyphon + lyon, with egui | fastest measured | egui, about 1 ms | yes, same pass | yes | ours, winit |
| vello, with egui | 2 to 3 times slower when dense | egui, about 1 ms | no, copies into an atlas | yes | ours, winit |
| GPUI | 5.8 ms for 250 notes | built in | no, YCbCr only | no, Metal | GPUI's |

## Consequences

- K1 to K6 draw through three item types in `Scene`: shapes, text runs and paths. `specular-render` owns the glyphon atlas and the lyon tessellator.
- Text editing uses cosmic-text's `Editor`, since glyphon renders cosmic-text buffers. The plan's parley `PlainEditor` option goes away with vello.
- The renderer has to break batches where a page sits between items in z-order. Glyphon draws all prepared text in one call, so each run of items needs its own `TextRenderer`, or text needs depth. This is the largest piece of work the choice creates, and the bake-off did not build it.
- Skip text under about 2.5 px. It costs nothing visible and removes the worst case in both renderers.
- The canvas pass needs 4x MSAA for lyon's edges. Glyphon and the SDF shapes work in a multisampled pass.

## Not measured

- Presentation to a window, and a HiDPI target. A 2x display has four times the pixels of this test.
- Canvas items and 40 pages in the same frame.
- Varied text. The scene uses 24 English words, so the glyph atlas holds a few dozen glyphs. CJK or many fonts would cost glyphon more per zoom step.
- The numbers were taken while other builds ran on the machine. Two full runs agreed within about 10 percent.

## Needs a human at a Mac

1. Open `native/bakeoff/golden/compare-5x-zoom-*.png` and the crops on a real display and confirm the sharpness call.
2. Zoom slowly with glyphon text on screen and judge whether glyphs shimmer as they snap to pixels. A still image cannot show this. If it is bad, the held-raster approach above also fixes it during gestures.
3. Look at `golden/sdf-egui.png` and decide whether egui's look is acceptable once themed. It was tested at 1 point per pixel only.
4. Type into an egui text field with an IME through egui-winit.
