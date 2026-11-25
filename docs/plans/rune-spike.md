# Rune spike

**For:** [ADR 0051](../adr/0051-rune-shell-around-the-compositor.md), with the measurements [ADR 0053](../adr/0053-page-frames-follow-the-display.md) waits on.
**Started:** 2026-11-25. Results go in the log at the end of this page, one entry a step.

The spike answers whether Rune can host Specular. It is throwaway code, as ADR 0040's bake-off was: a pass means the real shell is then built as a second binary in `specular-shell`, not that the spike is merged into it.

## Where the code goes

`native/bakeoff/rune-shell/`, a cargo workspace of its own outside `native/Cargo.toml`. Rune needs eight `[patch.crates-io]` entries at the workspace root, and those would put the whole app on Rune's patched `wgpu-hal` and `naga` before we have decided to. The spike depends on `specular-core`, `-doc`, `-interact`, `-scene` and `-compositor` by path, unchanged.

Rune is pinned to one revision of `dev` for the whole spike. The pin and the reason for any bump are recorded in the log.

## Questions

| # | Question | Pass | Runs on |
|---|---|---|---|
| 1 | Do Rune and our crates build into one binary with a single `wgpu` 30.0.1? | It builds; the lockfile has one `wgpu`, `wgpu-hal` and `naga` | Linux |
| 2 | Does `specular-compositor` draw inside a Rune `Compositor`, on Rune's device and into Rune's target, with Rune's UI over it? | `--snap` of `fixtures/kitchen-sink.canvas` matches `specular --snapshot` of the same canvas where the canvas shows, with a toolbar drawn over it | Linux (lavapipe), then Mac |
| 3 | Do the models drive Rune widgets with no adapter layer? | A toolbar built from `toolbar(&App)`; `--tree` lists each control under its `ControlId`; a scripted tap on a tool control changes `app.tool` through `update` | Linux |
| 4 | Does a GPU-free `Runtime` run a scenario step? | A `cargo test` taps a control by key and asserts on `App`, with no adapter | Linux |
| 5 | Does the pass run on Rune's Metal backend? | With multisampling off, the frame draws on `GpuBackend::Metal`; otherwise record what `GpuBackend::Wgpu` costs | Mac |
| 6 | Do page IOSurfaces import onto Rune's device? | Two CEF pages draw zero-copy in one z-order with a sticky above both | Mac |
| 7 | Does CEF's external pump share the run loop with Rune's event loop? | Pages load and paint; shutdown is clean; no re-entrant event delivery | Mac |
| 8 | Do keys and composition reach a page? | Digit 1 and numpad 1 arrive with their own key codes from `on_physical_input`; a scripted composition lands in a CEF text field | Mac |
| 9 | Does 120 Hz hold? | 40 CEF pages under a pan with a Rune popover open: p99 interval under 1.5 frames, on a quiet machine, release build | Mac |
| 10 | Can a panel be a script? ([ADR 0055](../adr/0055-chrome-as-files-then-scripted-panels.md)) | The toolbar as a Luau script over a snapshot of `toolbar(&App)`: `--tree` lists its controls under the same keys, a scripted tap runs the `Action`, and a saved edit reloads it with no rebuild | Linux |

## Measurements for ADR 0053

Taken on the Mac in the same session, on the existing Kit shell first so there is a baseline:

- `CefDoMessageLoopWork` duration, mean and p95, at 9, 20 and 40 pages during a pan (the ledger's `Pump` entries).
- Input to present for an entered page at 60 and at 120 windowless fps, with a colour-flip page and a screen recording.
- Paint to present, with and without a wake on paint.
- GPU time a frame with and without multisampling at 2x.

## Steps

1. **S1, Linux.** Workspace, pin, build. Answers 1.
2. **S2, Linux.** The compositor inside a Rune `Compositor` with synthetic pages; the toolbar from the model; headless `--snap`, `--tree` and `--script`. Answers 2 (on Vulkan) and 3.
3. **S3, Linux.** One `Runtime` test. Answers 4. Records what stands in for text measure with no GPU.
4. **S4, Mac.** The same binary in a window on Metal. Answers 2 on Metal, and 5.
5. **S5, Mac.** CEF behind `--features cef`: bundle, pump, import, input. Answers 6, 7 and 8.
6. **S6, Mac.** Pacing and the ADR 0053 numbers. Answers 9.
7. **S7, Linux.** The toolbar as a `rune-luau` script beside the Rust one. Answers 10, and records what a render costs and what the script could not say.

S1 to S3 and S7 need no Mac and run first. S4 to S6 need macOS on Apple Silicon with a display, the CEF download and a bundle.

## What a fail means

- 1 or 2 fails: stop. Rune cannot host the pass as it is.
- 5 fails with multisampling off: stay on `GpuBackend::Wgpu` and record the cost.
- 7 fails: try `rune::embed::LayerHost` with our own window before stopping. If that is what works, ADR 0051 is rewritten around it.
- 8 fails on composition only: keep an AppKit input client for entered pages, as `key_bindings.rs` is kept.
- 9 fails: compare with the Kit shell's numbers from the same session before blaming Rune.

## Rules

- Nothing in `native/crates/` changes for the spike. If a crate has to change to be hosted (the compositor taking a device it did not make, for one), the change is written down in the log as a finding and made in the spike's own code where it can be.
- Debug builds unless a step says release. Numbers only from release.
- A step's log entry says what was run and what was only read.

## Log


### S1, 2026-11-25, Linux (lavapipe container). Question 1: pass

- **Exists:** `native/bakeoff/rune-shell/`, a workspace of its own with its lockfile and `rust-toolchain.toml` (1.99.1). Rune is pinned to `a1fbe7ff5a33663a87edbdffbbcc37260c549106` (`dev`, 2026-11-24) with its eight patches at the same revision, copied from its README's "Add it". Our five crates are path dependencies, unchanged.
- **Run:** `cargo build` (debug). Rune and `specular-compositor` link into one binary, `rune-shell`. No source change and no feature flag was needed on either side.
- **Lockfile:** 500 packages (`native/Cargo.lock` has 951). Exactly one each of `wgpu` 30.0.1 (crates.io), `wgpu-core` 30.0.1 (crates.io), `wgpu-hal` 30.0.1 (Rune's git fork) and `naga` 30.0.1 (Rune's git fork). `cargo tree -i wgpu` shows glyphon, Rune and the spike on the one copy.
- **Conflict 1, resolved by pinning.** A fresh resolve takes `wgpu`, `wgpu-core` and `wgpu-types` 30.0.2 from crates.io while the patched `wgpu-hal` and `naga` stay at 30.0.1. I pinned the family to 30.0.1 with `cargo update --precise`, which is what Rune's own lockfile and ours hold. Whether 30.0.2 over the 30.0.1 forks builds was not tried. The real workspace should write `wgpu = "~30.0.1"` as Rune does, or the next `cargo update` splits them.
- **Conflict 2, found in S2.** The fresh resolve also took `lyon_geom` 1.0.20, `lyon_algorithms` 1.0.22 and `lyon_tessellation` 1.0.23, one patch release past `native/Cargo.lock`. They draw a dashed curve with its dashes a fraction of a pixel along (54 pixels of 902,400 at 100%). Pinned back to 1.0.19, 1.0.21 and 1.0.22. So the lockfile is part of what a snapshot looks like, and moving to Rune's workspace must carry ours over, not resolve afresh.
- **Not a conflict:** two `resvg` (0.45 Rune's, 0.46 ours) and two `harfrust`, `skrifa`, `png` and `kurbo` coexist. Rune's default features bring zbus and hunspell on Linux; they were left on.
- **For the Mac:** nothing in the manifest is per-platform. The patched `wgpu-hal` touches Metal, and this step only built its Vulkan side.

### S2, 2026-11-25, Linux (lavapipe). Question 2 on Vulkan: pass. Question 3: pass

- **Exists:** `src/canvas.rs` (the Rune `Compositor`), `src/chrome.rs` (the toolbar view), `src/shell.rs` and `src/files.rs` (state and effects, no GPU), `reference/` (below), `tools/` and `scripts/`. Captures are in `docs/adr/assets/0051/`.
- **How it is hosted, with nothing in `native/crates/` changed.** `Compositor::new(device, queue, format)` already takes a device it did not make, and `render_scene(target, ..)` a view it did not make. On the first frame, and whenever `Frame::device_key()` or the format changes, the spike makes a `Compositor` from `f.device.clone()`, `f.queue.clone()` and `f.format`. The 4x pass resolves straight into Rune's target.
- **Finding: `render_scene` makes and submits its own encoder.** It cannot record into `f.encoder`. That works because our submit lands before Rune submits its own, at the cost of a second submit a frame. A real shell wants `render_scene` to take an encoder.
- **Finding: `Frame::draw_ui()` cannot be used.** It clears the target (`LoadOp::Clear`), and the load variant, `Renderer::draw_over`, is `pub(crate)` (read, not run). The spike calls the public pieces: `ui.prepare`, `ui.encode_layers`, then `ui.draw_target_range_with(.., LoadOp::Load, ..)`. `prepare` is not quite `prepare_ui`, which is also private and picks byte precision for 8-bit sRGB-blended targets. No difference showed in the chrome; nothing compared it.
- **Format and colour: no mismatch.** Rune's headless target is `Rgba8Unorm` with `srgb_blend` on, which is our offscreen format and our encoded blending. What a window's surface format is was not run.
- **Reference.** `cargo run -p specular-shell -- --snapshot` does not build on Linux: the `specular` binary is the Kit window (7 errors, `gpui_kit`, `objc2`). `reference/` is ten lines calling the shell library's public `launch`, with a lockfile copied from `native/Cargo.lock`. Two reference runs are byte-identical.
- **Run, 1200x800 at scale 1, same camera, lavapipe both sides** (`tools/compare.sh`): below the toolbar's 48 rows, 0 of 902,400 pixels differ at each of three cameras: the overview (325 items, both pages), a page with comments at 100%, and the images, svg, gif and Documents at 50%. Also 0 at 600x400 scale 2. I read the PNGs: the canvases look the same and the ribbon sits over the "Drawings" heading at the top.
- **What differed on the way, all explained:** the pages (the synthetic page animates; the reference shows its first frame, so `--still` holds it), the lyon drift in S1, and at scale 2 the page textures, because pages are hosted before the first frame says what the scale is.
- **Question 3.** `chrome.rs` reads `toolbar(&App)` and calls Rune's builders; there is no type between. `--tree` lists `tool.select` .. `tool.inspect`, `zoom`, `theme` and `share.copy` under role `toolbar`, each a `button` with the model's label and, for tools, `checked`. A scripted `{"type":"tap","key":"tool.shape"}` took `app.session().tool` from `Select` to `AddShape` through `Event::Action`, and the tree then shows `tool.shape` checked. 2,788 pixels changed, all in the toolbar's rows. A second script opens the zoom list (`zoom.10` .. `zoom.200` as `menu-item-radio`) and picks 50%; the readout then says 50%. Which list is open is the spike's own state, as in the Kit.
- **Finding: the scale arrives too late.** A Rune app learns its device scale in its first frame. `Effect::CreatePage` runs when the document opens, before that. The spike passes `--scale` in and corrects in the frame.
- **Finding: the effect runner had to be written again** (about 300 lines for pages, images and Documents). `specular_shell::Runtime` owns its window's compositor through `ShellWindow` and asks it for the text measure and the image spec at any time. Under Rune the compositor exists only inside a frame. Decoded pixels wait in the state and are uploaded in `render`.
- **Device changes:** on a new `device_key` or `release_gpu` the compositor is dropped and made again, and images are uploaded again from the kept pixels. A page has no texture until it next paints. None of this ran: every run had one device.
- **For the Mac:** the spike has no platform code. Opening a window was not run. Pointer and key input to the canvas is not wired; only the toolbar takes presses. Nothing wakes Rune when a page paints: `Compositor::animating` is true only while text settles, so real pages need `AppSpec::setup`'s `Waker`. Rune's default Mac backend is its own Metal one, where multisampling panics, and our pass has only 4x pipelines, so S4 starts with `App::gpu_backend(GpuBackend::Wgpu)` and `rune::prewarm_gpu_on`. Question 5's "multisampling off" needs a single-sample pipeline in `specular-compositor`. The IOSurface import calls `device.as_hal::<Metal>()`, which can only answer on `GpuBackend::Wgpu`, and there through Rune's patched `wgpu-hal`.

### S3, 2026-11-25, Linux. Question 4: pass

- **Exists:** `tests/toolbar.rs`, one test. It builds the same `App` as `main` without the compositor, takes `headless_runtime(size, Env)`, opens `kitchen-sink.canvas`, taps `tool.sticky` by key, steps three frames and asserts `app.session().tool == Tool::AddSticky`, then that the accessibility tree shows the button checked.
- **Run:** `VK_ICD_FILENAMES=/nonexistent cargo test`: 1 passed in 0.22 s, with no Vulkan driver to find. No adapter is asked for anywhere on this path.
- **No adapter layer:** the test presses the element the window shows, found by the `ControlId`. The 165 entities load, both pages are hosted and the images are decoded, all without a device.
- **Text measure with no GPU:** `specular_compositor::GlyphMeasure::new()`, which loads the system and bundled fonts into its own font system and shapes as the scene pass does. The spike uses it in the window too, since the measure is wanted before the first frame. The cost is a second font system beside the compositor's, whose `Fonts` is private; a real shell wants to hand one to the other.
- **Does anything force a GPU?** No. `specular-compositor` is linked for `GlyphMeasure`, `ImageMips` and the types, and makes nothing until `Compositor::new`. Image mips need the compositor's `ImageSpec` (the target's colour space and the device's size limit), so the spike keeps straight pixels and builds mips at upload.
- **Surprises in Rune's API, against the notes this started from:** `draw_ui` clears (above). There is no flag to print app state; `Compositor::summary` prints after a headless run, and the spike uses it. Headless runs default to scale 2. A list opened by a tap fades in, so a `--snap` 0.1 s later catches it half drawn; that the fade blends over our canvas shows Rune's isolated layers work with `LoadOp::Load`. `Runtime::window_scale()` gives the scale, not `scale()`. Everything else was as described.
- **Not done in S1 to S3:** no window, no macOS, no CEF, no canvas input, no dark theme, no release build, no timing worth reading (lavapipe, debug: about 25 ms of CPU a frame).
