# ADR 0051: Rune as the shell, with the compositor drawing inside its frame

**Status:** Proposed, pending the spike in [`docs/plans/rune-spike.md`](../plans/rune-spike.md). Nothing here has been built or run. If the spike passes, this supersedes [ADR 0040](./0040-gpui-kit-hybrid-shell.md).
**Date:** 2026-11-25
**Related:** [ADR 0039](./0039-rust-canvas-render-stack.md), the canvas's own wgpu pass, which stays. [ADR 0040](./0040-gpui-kit-hybrid-shell.md), the GPUI Kit shell this would replace. [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), which made the shell replaceable. [ADR 0052](./0052-one-runner-one-state-document.md), the testing this unlocks. [ADR 0053](./0053-page-frames-follow-the-display.md), the page pipeline changes made alongside.

## Context

The GPUI Kit shell works, with costs ADR 0040 named and the run log added to:

- The canvas is a sibling `NSView` under a transparent GPUI window. That rests on three facts about GPUI's macOS backend no API promises, and needs two frames kept in step: about 940 lines of wake channels, a model gate and display-link catch-up in `kit/canvas*` and `kit/shell.rs`.
- GPUI drops the key code, so an `NSEvent` monitor feeds pages their keys.
- Every GPUI frame renders the whole root view on the canvas's thread. A shimmering label took the canvas from 120 to about 44 fps in a debug build.
- The Kit cannot run headless. Scenarios press controls through the models with nothing drawn, and a second driver (`kit/debug_input`, 1,910 lines) replays them in a window, where 57 of 58 pass and take 723 s against 19 s headless.
- Nothing is exposed to VoiceOver.

[Rune](https://github.com/anomalyco/rune) is a Rust UI framework from Anomaly with its own renderer, text, layout and animation. Its first commit is 2026-10-22. Read at `a1fbe7ff` (2026-11-24):

- It renders through the wgpu 30 API. A `Compositor` hands the app its `wgpu::Device`, `Queue`, `CommandEncoder` and target each frame (`crates/rune/src/compose.rs`), in a window and headless alike. The app draws, then calls `draw_ui()`.
- Every app runs headless: `--snap` to a PNG, `--tree` to JSON that includes the AccessKit tree, `--script` for input by element key. A GPU-free `Runtime` drives input and a virtual clock in `cargo test`.
- `El::on_physical_input` delivers a key code, modifiers and repeat before shortcuts and focus navigation.
- Animation runs on retained tracks. The view function does not run to animate.

## Decision

Replace the Kit window with a Rune app, if the spike's questions come back yes.

- **One device, one frame.** `specular-compositor` renders inside a Rune `Compositor` on Rune's device, into Rune's target, and Rune draws the chrome over it. The sibling view, the transparent window, the pins on GPUI's backend and the two-frame coordination go.
- **Pages stay in our pass.** We import page IOSurfaces onto Rune's device ourselves, as today. Rune's `NativeSurface` element is not used for pages: it treats BGRA as straight alpha, where CEF's is premultiplied, and its own bench stops at 9 surfaces.
- **The chrome is Rune views over the same models.** `toolbar(&App)`, `dock(&App)`, `sidebar(&App)` and the rest stay in `specular-interact` (ADR 0044). The view adapters are rewritten; about 5,700 lines today, with no behaviour in them.
- **Keys come from `on_physical_input`**, and composition from Rune's IME hook. The `NSEvent` monitor goes. `key_bindings.rs` (AppKit editing commands for an entered page) stays in some form.
- **Rune is pinned to a revision of `dev`**, moved on purpose, as the GPUI pin was. Its eight `[patch.crates-io]` entries go in `native/Cargo.toml`, so our compositor builds against Rune's patched `wgpu-hal` and `naga`.
- **The shell is a second binary until it reaches parity**, beside the Kit one, as ADR 0040's was beside winit. The Kit shell is deleted only after a by-hand pass.

What the port touches: `specular-shell/src/kit/` (13,314 lines). About 3,200 lines of GPUI arrangement glue are deleted outright, about 1,300 of platform glue are re-homed, and the adapters are rewritten. Nothing in `core`, `doc`, `interact`, `scene` or `api` depends on GPUI.

## Alternatives

**Stay on GPUI Kit.** It works and is pinned. It keeps the sibling view, the second driver and the missing accessibility tree. `gpui-shell`, the scripted-view runtime Herdr uses, is in the same repository and would be one dependency away (ADR 0055).

**Embed Rune in our own layer** (`rune::embed::LayerHost`). The host owns the window and feeds input by hand. It exists for iOS and has no macOS example. Kept as the fallback if CEF's pump and Rune's event loop cannot share a run loop.

**Our own panels in the canvas pass.** The built-in renderer was that, and it was retired once the Kit drew the models.

## What was read

Nothing was built. From Rune's source and docs:

- No separate render thread exists yet (`ARCHITECTURE.md`); the app/render split is a plan. Our encode, CEF's pump and Rune's layout share the main thread, as they do today.
- Rune's default macOS backend is its own Metal one, which implements a subset: multisampling, depth and render bundles panic. Our pass uses 4x multisampling for lyon meshes only (ADR 0053 drops it). `GpuBackend::Wgpu` is the opt-out.
- Rune takes an instance, not a device. Every GPU resource, page imports included, is made on its device, and a compositor must handle `device_key` changes and `release_gpu`.
- There is no child-view or webview embedding. Offscreen textures are the only way to show a page, which is what we do.
- There is no live control socket; `--window-script` is fixed at launch. Our HTTP API stays the live channel (ADR 0052).

## Risks

- **Age and churn.** Five weeks old, version 0.0.1, no releases, 3,236 commits, 1,312 changelog entries in November. The pin holds it still; a bump may break anything.
- **No pull requests.** Fixes go upstream as issues. If one is not taken, we carry a patch.
- **The patched forks.** Our `wgpu` must stay on `~30.0.1`, and the `wgpu-hal` patch touches Metal.
- **Unanswered until run:** whether CEF's external pump coexists with Rune's winit loop on macOS; whether 120 Hz holds with 40 pages; whether a composition reaches a page field through Rune's window-level IME hook.

## Consequences if adopted

- One renderer draws the chrome in a window and headless, so scenarios see the real toolbar and sidebar (ADR 0052).
- An accessibility tree exists.
- Chrome that follows a canvas item can be ordinary UI again, since it is drawn in the canvas's frame.
- About 420 packages leave the lockfile with GPUI, less what Rune brings.
- We depend on a young framework whose maintainers take no patches.
