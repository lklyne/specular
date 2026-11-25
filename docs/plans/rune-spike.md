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

S1 to S3 need no Mac and run first. S4 to S6 need macOS on Apple Silicon with a display, the CEF download and a bundle.

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

