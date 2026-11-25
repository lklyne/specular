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

### S7, 2026-11-25, Linux (lavapipe; debug build). Question 10: pass, with three things the ADR has to change

- **Exists:** `panels/toolbar/init.luau` (181 lines; `chrome.rs` is 273), `src/panel.rs` (the host's side, 415 lines, about half of it model to plain data and id to `Action`), `--chrome luau|rust` (Rust unless said) and `--panel DIR`, `tests/luau_toolbar.rs` (5 tests, no GPU). A capture is `docs/adr/assets/0055/luau-toolbar-shape-armed.png`.
- **Run:** everything below unless it says read. Not run: a window, a release build, macOS, `--!native`.
- **What `rune-luau` adds:** 15 packages (515 from 500; `mlua` from git, `luau0-src`, `notify` for `watch`), none changed, still one `wgpu` 30.0.1 and the pinned lyon. Luau's C++ takes 39 s of a clean build on 4 cores. The debug binary grew 47 MB (798 to 845, unstripped). `tools/compare.sh` still gives 0 differing pixels at the three cameras.
- **Down: a signal.** The host fires `toolbar` on `@specular/app` with `toolbar(&App)` as `Data` when the model differs (`PartialEq`) from what it last sent, and after each new VM. `Script::fire` enters the VM and that moves the script's version, which is what `Script::view` memoizes on. So one call hands over the data and invalidates the render. The key is the whole toolbar model: one key a panel.
- **Up: `app.run(id)`.** It queues; `ScriptOptions::around` drains the queue after the handler with `&mut Shell`, looks the id up in the model as it is then, and sends that control's `Action` through `Event::Action`. A disabled or unknown id does nothing, so the host, not the script, enforces `enabled`. A general table is already there for controls: `Event::Control(ControlId, Modifiers)`. Actions with arguments (`ZoomTo(u16)`) need `Action` to decode from a name and `Data`, which nothing does today.
- **a. Tree:** the subtree under `toolbar` is equal as JSON to the Rust toolbar's (keys, roles, names, states, actions) at rest, with a tool armed, with the zoom list open and after picking 50%. The one difference is above it: Rune keys a surface's root `luau:toolbar:render` whatever key the script gives, so the script's root is the window column and the ribbon its child.
- **b. Actions:** `tap-shape.json` prints `tool=AddShape`; `open-zoom.json` opens `zoom.10` .. `zoom.200` and picks 50%.
- **c. Pixels:** 0 of 960,000 differ at rest, with the list open and at 50%; 16 differ by 1 level in the snap after the tap (not looked into). I read the PNGs. That took the Rust colours handed over as a palette (`fill = "surface"`).
- **d. Runtime test:** passes with `VK_ICD_FILENAMES=/nonexistent`.
- **e. Hot reload:** a save of the script, seen by the watcher, changed the ribbon's name and order within 20 ms with no rebuild (compile 2 to 6 ms, swap about 1 ms). `app.tool` and the open list (the script's own state, through `saveHotState`) survived. A save with a syntax error, and one that throws as it starts, are logged and the last good VM keeps running and taking presses. A script that starts and throws in `render` is swapped in and shows Rune's error card with a Reload button; the app carries on. So "keeps the last good" holds for compile and start, not for render.
- **f. Sandbox:** `io`, `dofile`, `loadfile` and `package` are nil; `os` is `clock date difftime time`; `debug` is `info traceback`; `loadstring` and `getfenv` exist. `require` of `../outside`, `./sibling`, `/etc/passwd` and an unregistered `@specular/app` is refused. Globals and `string` are read-only. A script can overwrite `@rune/ui`'s functions in its own VM. `while true do end` stops at 250 ms, runaway recursion as a stack overflow, a hog at 64 MB with "not enough memory"; the VM answers afterwards each time, and the 64 MB goes back at the next full collection.
- **Cost (debug, GPU-free `Runtime`; not a benchmark):** idle frame, view rebuilt with nothing changed, pan, selecting a sticky: 0 VM entries, 0 renders. Tool change by a press: 3 entries (handler, snapshot, render), 0.6 ms in the VM. By an action: 2 entries, 0.55 ms. Start: 4 entries, about 1 ms. Heap 0.8 MB. A frame's wall time was the same for both chromes within noise (1.2 to 1.5 ms) except on a change (2.5 against 1.5 ms).
- **Finding: zoom enters the script.** The zoom readout is in the toolbar model, so 100 frames of pinch rendered the panel 68 times. The ADR's "a pan or zoom cannot enter a script" holds for pan only. The readout has to be a host-drawn node or its own surface, or the cost accepted.
- **Finding: `@rune/ui` cannot say this toolbar.** No `checked`, no `hasPopup`, and no `toolbar`, `menu` or `menuitemradio` role: each is an error ("unknown prop `checked`"), and without them the ribbon is a `group` of unchecked buttons. One host node, `ui.control` (about 30 lines), closed it. So the script API is Rune's vocabulary plus ours from the first panel.
- **Not expressible, not tried:** icons (words here, as in `chrome.rs`), tooltips with chords, swatches, steppers, fields, focus handoff, dismissing the list on a press outside, and anything that needs the pointer's position.
- **Against the README:** `reload()` called outside a frame draws nothing until something else invalidates; a syntax error reads `init.luau140:` with no separator; `selected` fills with a palette colour the default palette does not have (read); `lib.rs` says Linux runs native code, which needs the `native-code` feature (read).
- **The sidebar (read):** `ui.virtualList` calls `renderRow(i)` only for rows entering the window and `ui.each` memoizes per item on a key and a version. A whole-model snapshot would convert hundreds of rows to `Data` on every change, so the sidebar wants rows pulled by index through `around_read`, with a version a row.

### S4, 2026-11-25, macOS (M3 Max, built-in ProMotion display). Question 2 on Metal: pass. Question 5: pass, with aliased mesh edges and no page import

- **Run:** everything below unless it says read. Debug unless it says release. The window was on screen and unlocked; I captured it with computer-use after wrapping the binary in a throwaway `.app` (an unbundled process is not offered, and `screencapture` from the shell is refused). Not run: a system pointer click in a key window, CEF, the dark theme, a second display.
- **Build:** the first macOS build passed with no change (54 s), so Rune's patched `wgpu-hal` builds its Metal side with our crates. The lockfile did not move. The spike gained `--backend wgpu|metal`, `--samples 4|1`, `RUNE_SHELL_PACE=1` and a `tooling` feature (Rune's `--snap` in release; it adds no packages).
- **Headless on Metal (`GpuBackend::Wgpu`, 4x):** against the reference, cameras a and c differ in 0 of 902,400 pixels. Camera b differs in 289,206, all inside the two page rectangles and 0 outside them: the reference shows synthetic frame 2 on this Mac in four runs out of four, because its wall-clock pump paints twice, while `--still` holds frame 1. So the canvas matches; the page frame is a reference timing artifact.
- **Window:** the surface is `Bgra8Unorm` with sRGB blending, at scale 2 (2400x1600 for 1200x800 points). The canvas draws, the toolbar draws over it, and text is sharp at 2x. Neutrals equal the headless snap within 1 level. Saturated colours equal the headless sRGB values converted to Display P3 within 2 levels, so the window is colour-managed as sRGB and shows no shift. The pages look darker only because the window is not `--still`.
- **Input:** `--window-script` taps armed `tool.shape`, opened the zoom list over the canvas and picked 50%. Background input from computer-use never arrived. Neither an `AXPress` nor a raw click drew a frame. The toolbar is in the macOS accessibility tree (an `AXToolbar` of `AXToggle`s, the list as `AXMenuItem`s), but an `AXPress` on a toggle does not run the press, so VoiceOver could find the toolbar but not activate it.
- **Resize:** `--window-script` refuses `resize` in a window. Dragging the corner from 1200x800 to 802x562 redrew at the new size with the toolbar re-centred, and the capture after it showed nothing stretched or stale. At that width the toolbar overlaps the traffic lights.
- **Pacing:** with nothing changing the window draws no frames (gaps of 1 to 3 s between scripted steps). In debug, animating frames come every 41.67 ms. That is five vblanks at 120 Hz, missed because Rune's `update_ms` is 27 ms. In release, bursts run at p50 10 to 14 ms and p95 17 ms on both backends, while our render takes 1.6 to 2.3 ms and Rune's update about 1 ms. So it is neither a steady 120 Hz nor CPU-bound on our side. The window was not key and the machine was not quiet; S6 measures this properly.
- **The compositor option (a change in `native/crates/`, reported as the rules ask):** `Compositor::with_samples(device, queue, format, samples)`; `Compositor::new` calls it with 4. At 1 the scene pass draws straight into the target with `StoreOp::Store` and no resolve, the pipelines and glyphon renderers are built at that count, and no multisampled texture is made. 7 files under `specular-compositor/src/`, 106 lines added and 49 removed. The 4x output is unchanged (0 differing pixels against the reference at cameras a and c after it). Workspace clippy passes.
- **`GpuBackend::Metal`:** at 4x it panics with "doesn't support multisampled textures" (`rune-gpu/src/metal/resource.rs:297`). At 1x the frame draws headless and in a window, and is pixel-identical to wgpu at 1x at all three cameras. No other `unsupported!` was hit: no depth, render bundles or viewport past the target came up on this canvas. The first zoom on Metal had one 63 ms render, not looked into.
- **1x against 4x:** differences are on lyon meshes only: 1.55% of pixels at the overview, 0.28% at camera b and 0.04% at camera c. Freehand strokes, highlighters, edges, arrowheads, dashed outlines and the outlines of diamonds, triangles, hexagons, parallelograms, chevrons and cylinders stair-step. SDF rectangles, ellipses and pills, text, pages and images are unchanged. That is the list ADR 0053's analytic edges must cover. [Close-up](../adr/assets/0051/drawings-4x-vs-1x.png).
- **Frame CPU, release, headless, three runs each** (Rune's `frame cpu` p50 and p95 over 180 forced frames at rest, overview of the kitchen sink, 325 items, 2400x1600): wgpu 4x 0.25 / 0.31 to 0.40 ms, wgpu 1x 0.24 to 0.28 / 0.29 to 0.40 ms, Metal 1x 0.17 to 0.20 / 0.21 to 0.37 ms. That covers Rune's frame and our render's CPU side with cached text. It excludes GPU time, the present and page uploads.
- **For S5:** the loop is winit's `run_app`, which is the main thread's `NSApplication` run loop (read). `AppSpec::poll` runs at the start of each frame and whenever `AppHandle::wake` or the setup `Waker` wakes the loop, also while hidden; returning true asks for a frame (read; the idle gaps confirm nothing polls on its own). So the CEF pump belongs in `poll`, woken by CEF's `OnScheduleMessagePumpWork` through the `Waker`, with `Tx::wake_at` as the 30 Hz fallback; a `CFRunLoopTimer` on the main run loop is the other hook (read, not tried). `device.as_hal::<Metal>()` answers on `GpuBackend::Wgpu` and returns none on Rune's Metal backend, which is a `wgpu::custom` device. So today's IOSurface import works only on Wgpu, and S5 runs there; on Rune's backend the import would need its own path into that device.
