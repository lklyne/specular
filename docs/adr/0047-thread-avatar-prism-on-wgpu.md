# ADR 0047: A thread's avatar is the Prism, drawn on wgpu and read back into GPUI

**Status:** Accepted, 2026-10-25, by Lyle with the [big run](../plans/rust-native-big-run.md)'s decisions. Built this way in `native/`.
**Date:** 2026-10-14
**Related:** [ADR 0040](./0040-gpui-kit-hybrid-shell.md), whose GPUI shell shows the avatar. [ADR 0039](./0039-rust-canvas-render-stack.md), which found the `surface()` limit this works around.

## Context

Each agent thread in the Kit's right panel shows the Prism as its avatar. The Prism is a gradient card inside a liquid shell and a glass shell, ported from a three.js build (lyle-klyne `builds/2026-10-11-prism`). It is 20 pt, 40 px on a 2x display, in each thread row, and 40 pt at the head of the open thread's run bar (the row that says what the agent is doing). It animates while the thread's agent works.

The web build is three.js TSL on WebGPU, and the app draws with wgpu. GPUI draws with its own Metal device and takes no wgpu texture.

## Decision

**The WGSL is three's own, dumped and checked.** We did not translate the TSL by hand. A page hook on `GPUDevice` and `GPUQueue` recorded every shader module, pipeline, uniform write, texture and pass of the running web build, on a fixed 60 Hz virtual clock so that frame N is the same on every run. `specular-prism` keeps the generated WGSL close to verbatim under `shaders/`. It reads each uniform block's layout from the WGSL source, so a regenerated shader can't shift offsets silently, and `uniforms.rs` is the one place that maps three's `nodeUniformN` names to `PrismParams` fields. The `parity` example renders frames headless and diffs every pass against the capture. At 1024 px the mean difference is under 0.001/255 on every pass, and the uniform bytes match word for word. Of the web build's nine post passes, seven have zero weight in the Prism preset and are not ported. The scripts that took the capture are in `native/crates/specular-prism/capture/`, with a README. The capture itself (about 550 MB) is not in the repository. `SPECULAR_PRISM_REFERENCE` or the example's first argument names a copy on disk.

**The avatar is read back through the CPU.** Each drawn frame is copied into a buffer, mapped, and handed to GPUI as a `RenderImage`, which GPUI uploads to its atlas. GPUI's `surface()` element takes only a biplanar YCbCr pixel buffer and asserts on anything else. That format has no alpha and half-resolution colour, so the glass's edge and colour fringes would be lost against the panel. The copy is 6.4 KB a frame. Reads are double-buffered, so the picture shown is one frame behind the draw, and each replaced image is dropped from the atlas with `window.drop_image` so it doesn't leak.

**It draws on the compositor's device, and builds off the main thread.** The avatar takes the canvas's wgpu device and queue (`CanvasSurface::gpu`), so there is no second device and its submits come from the thread the compositor submits from. Building the renderer compiles the shell shaders for about 0.45 s, so it runs on its own thread with a clone of the device and queue. The avatar shows a grey square until the renderer arrives.

**Each thread has its own clock, which runs only while its agent works.** Scene time is `4 s + seed(thread id) × 30 s + time spent working`. At 4 s the key light has eased in, and it is the web build's canonical frame. The seed gives each thread its own colours. Because idle time never counts, an avatar that starts working moves on from the picture it held, and one that stops keeps the picture it stopped at. An idle avatar draws nothing, reads nothing and schedules nothing.

**Each avatar is its own GPUI view and wakes itself on a timer.** A live thread avatar sets one on a fixed 33 ms cadence (30 fps), and the run bar's tumbling one on a 16.7 ms cadence (60 fps, below), rather than asking for every display frame. In GPUI 0.3.8 a view that changes re-renders every view above it, and the Kit's root renders the shell's view on every frame. So a live avatar still re-renders the whole shell, but 30 times a second rather than 120.

**The run bar's avatar tumbles, at 60 fps.** `specular_prism::Tumble` turns the shells (the card is a billboard and stays facing the camera) with a damped spring toward a target. Every 0.9 to 2 s of work a burst moves the target by one of the cube's own turns, mostly about a corner's or an edge's diagonal axis, and kicks it about a random axis. Because every target maps the cube onto itself, it always comes to rest looking like the still avatar, and stopping needs no jump. It steps at a fixed 240 Hz, so it moves the same at any frame rate, and the bursts are seeded by the thread id. A tumble at 30 fps looked steppy, so this one avatar draws at 60.

## Consequences

- A live avatar costs a GPU→CPU→atlas copy per frame. If a later GPUI takes BGRA surfaces with alpha, a shared IOSurface would remove that copy.
- Everything the renderer allocates lives on the canvas's device. A device loss takes the avatars with it.
- If the build thread fails, the grey square stays and the shell logs a warning. Nothing retries the build.
- Only two things would stop the shell re-rendering: moving the avatar out of the shell's view tree, or a GPUI that keeps a view cached when a descendant changes. We did neither.
- The avatar renders at 24 shell segments and 3x supersampling, not the web build's 80 and 1x. A 40 px contact sheet showed no difference from 80 segments, and 3x looked the same as 4x.
- Over the light theme's near-white panel, the glass's white rim mostly disappears. The web build does the same over white.

## Measured

Debug build, Apple M3 Max, ProMotion display, two avatars live (`SPECULAR_AVATAR_LIVE=1`):

- The shell's view rendered 120 times a second when the avatars asked for every frame, and about 33 times a second with the 30 fps timer. With no avatar live it rendered only on input.
- Recording and submitting one avatar frame took 1.25 ms of the main thread, and collecting its read 0.15 ms.
- `PrismRenderer` cost about 1 ms a frame at 40 px, whatever the size, supersampling or segment count. The cost is fixed per pass, not fill.
- The tumbling run bar avatar (80 px, the only live avatar while a thread is open): the shell's view rendered 60 times a second. One draw took 1.25 to 1.75 ms of the main thread in a debug build, and 0.25 to 0.5 ms in release.

Not measured: a release build, true GPU time (the adapter has no timestamp queries inside encoders), and more than two avatars live.
