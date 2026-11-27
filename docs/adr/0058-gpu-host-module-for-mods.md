# ADR 0058: A `gpu` host module, so a mod can bring its own shaders

**Status:** Proposed, split out of [ADR 0055](./0055-chrome-as-files-then-scripted-panels.md) on 2026-11-26. Not built, and deferred until a mod needs it that web content cannot serve.
**Date:** 2026-11-26
**Related:** [ADR 0055](./0055-chrome-as-files-then-scripted-panels.md), mods and the runtime. [ADR 0057](./0057-mod-items-and-tools.md), mod items and their surfaces. [ADR 0047](./0047-thread-avatar-prism-on-wgpu.md), a native shader on the canvas's device.

## Context

A mod that brings its own shaders (a watercolor brush, a particle field, a shader toy, a 3D viewer) has two ways to reach a GPU.

- **As web content.** A page from the mod's folder runs in CEF, which has Chromium's WebGPU, and the canvas draws its frames like any page's. This works with ADR 0057 and nothing more. It costs a browser per item and a frame copy.
- **Natively.** The mod renders on the canvas's own wgpu device into a texture the canvas pass samples with no copy. This ADR is that path.

The test case is [`webgpu-brush`](https://github.com/lklyne/webgpu-brush) (npm 3.0.0, Lyle's WebGPU port of p5.brush). It can paint into a device it is handed (`initDevice({ device })`) and needs no canvas element to draw.

A review of the first write-up, against the published `gpui-shell` 0.11.0 and `webgpu-brush` 3.0.0, found the native path is larger than it read:

- **The host-module boundary is JSON.** A value crossing it is null, a bool, a number, a string, an array or an object. There are no bytes, no typed arrays, no objects with methods, and no way to install a global such as `navigator.gpu`. A WebGPU binding needs direct access to the QuickJS runtime, which `gpui-shell` does not export. That means an upstream change or a fork.
- **The brush uses a wide slice of WebGPU.** Compute passes, `drawIndirect`, `getBindGroupLayout`, `mapAsync` and `copyTextureToBuffer`. Its bundle also refers to `document`, `OffscreenCanvas` and `requestAnimationFrame`, which would need stand-ins.
- **Nothing ready-made fits.** Deno's `deno_webgpu` binds WebGPU to V8 through `deno_core`. Its Rust layer is a reference, not a dependency. (Not read in source; from general knowledge.)

## Decision

Not now. The brush ships as web content (ADR 0057, phase 2), and that is measured first.

If the native path is then wanted, there are two ways, in this order:

**1. A Rust host module for the one thing.** ADR 0055's core rule asks whether a mod can do it correctly. A brush engine in Rust and WGSL, exposed to mods as a small host module (`brush.stroke(itemId, points, style)`), needs only JSON across the boundary and no WebGPU binding. `specular-prism` is the precedent: its own WGSL on a `&wgpu::Device` it is handed. The cost is that each new GPU effect is a Rust change.

**2. The WebGPU API in script.** Only if several mods need their own shaders. Then:

- **The API** is the W3C WebGPU JavaScript API on the canvas's device, so code written for the browser runs as it is. A library that accepts a device works without change, and `webgpu-brush` doing so is the acceptance test. Left out to begin with: an HTML canvas context, SPIR-V and GLSL, and video as an external texture.
- **Surfaces.** `gpu.surface(itemId, { scale })` returns a double-buffered texture sized to the item. The mod renders into it and calls `present()`. It is a texture source in ADR 0057's terms. The canvas keeps drawing the last presented texture through a pan or zoom, and when a zoom settles the mod gets a `resolution` event.
- **One device.** A mod's command buffers are submitted by the host on the canvas's queue before the canvas pass of that frame. Device loss is reported to every mod, and each recreates its resources.
- **Validation is wgpu's own.** Shaders are WGSL through naga with bounds checks, and every call goes through wgpu's validation, as for any wgpu program. An error goes to the mod as a WebGPU error, never a panic. Nothing is added on top.
- **Headless.** `specular mod check` uses a headless adapter where one exists and can save a surface as a PNG.

Mods are trusted (ADR 0055, "Trust"), so there is no per-mod grant, memory cap or GPU time limit in this ADR. A GPU cannot be preempted, so a runaway dispatch stalls the frame; that is a bug in the mod, shown in `specular mod stats`, not something the host polices.

## What is not known

- How the brush performs as web content, which decides whether any of this is needed.
- Whether GPUI Kit would export enough of the QuickJS runtime for a binding, or we would carry a fork for it.
- How much of WebGPU the binding needs before `webgpu-brush` runs, and how much of the DOM has to be stood in for.

## Consequences

- No GPU work is committed by ADR 0055 or ADR 0057.
- The first native brush, if there is one, is Rust behind a small host module, and the scripted WebGPU API waits for a second client.
