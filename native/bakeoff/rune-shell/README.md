# rune-shell: the Rune spike

Throwaway code for [`docs/plans/rune-spike.md`](../../../docs/plans/rune-spike.md)
and [ADR 0051](../../../docs/adr/0051-rune-shell-around-the-compositor.md).
Steps S1 to S5 and S7 are here. The findings are in the plan's log, not in this file.

One binary, `rune-shell`: a [Rune](https://github.com/anomalyco/rune) app
whose frame is drawn by `specular-compositor` on Rune's device, with a
toolbar Rune draws from `toolbar(&App)` over it.

Rune is pinned to `a1fbe7ff5a33663a87edbdffbbcc37260c549106` (`dev`,
2026-11-24), with its eight `[patch.crates-io]` entries at the same revision.

## What is where

| File | What |
|---|---|
| `src/shell.rs` | The Rune app's state: an `App`, its pages, a cut-down effect runner. No GPU |
| `src/pages.rs` | The page source (synthetic or CEF) behind an `Rc<RefCell<..>>`, and its queue |
| `src/cef.rs` | Feature `cef`: starts CEF, wakes Rune when a page paints, waits headless runs out |
| `fixtures/` | `cef-pages.canvas`: two local HTML pages (a key logger, a scrolling page) and a sticky over both |
| `src/files.rs` | Image files to pixels, as the real shell decodes them |
| `src/chrome.rs` | The Rune view: one keyed element per control of the toolbar model |
| `src/canvas.rs` | The Rune `Compositor` that hosts ours and draws Rune's UI over it |
| `src/input.rs`, `src/keys.rs` | Canvas input: a window-sized element under the toolbar, keys and the input method, as the app's `Event`s |
| `tests/canvas_input.rs` | Drag, click and delete, wheel, pinch and Cmd+Z over the canvas, on the GPU-free `Runtime` |
| `tests/toolbar.rs` | The GPU-free `Runtime` test (question 4) |
| `panels/toolbar/init.luau` | The same toolbar as a Luau script (question 10) |
| `src/panel.rs` | The host's side of a scripted panel: the snapshot, `run`, `ui.control` |
| `tests/luau_toolbar.rs` | Question 10 on the GPU-free `Runtime`: tree, reload, cost, sandbox |
| `reference/` | `specular --snapshot` for Linux, where the `specular` binary does not build |
| `tools/`, `scripts/` | The pixel diff, the tree listing, and the Rune input scripts |

## Build

Rust 1.99.1 (`rust-toolchain.toml`). On a machine with no GPU, software
Vulkan: `export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`.

```sh
cd native/bakeoff/rune-shell
cargo build                                    # the spike, into ./target
(cd reference && CARGO_TARGET_DIR=../../../target cargo build)   # native/target/debug/specular-reference
```

## The checks, as they were run

```sh
OUT=/some/folder

# Question 1: one wgpu, wgpu-hal and naga
grep -A2 '^name = "\(wgpu\|wgpu-hal\|naga\)"$' Cargo.lock
grep -c '^\[\[package\]\]' Cargo.lock

# Question 2: the canvas under Rune's UI against the reference, three cameras
tools/compare.sh $OUT a 20,60,0.25        # the overview: every kind, both pages
tools/compare.sh $OUT b -2150,-1750,1     # a page, comments and a dashed edge at 100%
tools/compare.sh $OUT c 25,-1650,0.5      # images, an svg, a gif and Documents

# Question 3: the controls under their ids, and a scripted tap
RUN="target/debug/rune-shell ../../fixtures/kitchen-sink.canvas --camera 20,60,0.25 --still --size 1200x800 --scale 1"
$RUN --tree $OUT/tree.json && python3 tools/tree.py $OUT/tree.json
$RUN --snap $OUT/before.png --at 0.05                              # prints tool=Select
$RUN --script scripts/tap-shape.json --snap $OUT/after.png --at 0.5   # prints tool=AddShape
$RUN --script scripts/tap-shape.json --tree $OUT/tree-shape.json --at 0.5
$RUN --script scripts/open-zoom.json --tree $OUT/tree-zoom.json --at 0.2   # the zoom list, open

# Question 4: no GPU at all
VK_ICD_FILENAMES=/nonexistent cargo test
```

`compare.sh` draws the same canvas, size, scale and camera with both
binaries and prints how many pixels differ below the toolbar (the top 48
rows). After a headless run the spike prints three `specular:` lines: the
device, target format and frame count, what the last frame drew, and the
app's tool and selection.

## S7: the toolbar as a script

`--chrome luau` draws the toolbar from `panels/toolbar/init.luau` instead
of `src/chrome.rs`. Building it needs a C++ compiler (`mlua` compiles Luau).

```sh
LUAU="$RUN --chrome luau"

# a. the same controls under the same keys, roles, names and states
$RUN  --tree $OUT/tree-rust.json && python3 tools/tree.py $OUT/tree-rust.json
$LUAU --tree $OUT/tree-luau.json && python3 tools/tree.py $OUT/tree-luau.json

# b. a scripted tap runs the control's Action; the zoom list opens and picks
$LUAU --script scripts/tap-shape.json --snap $OUT/luau-shape.png --at 0.5   # prints tool=AddShape
$LUAU --script scripts/open-zoom.json --tree $OUT/luau-zoom.json --at 0.2
$LUAU --script scripts/open-zoom.json --snap $OUT/luau-zoom50.png --at 0.6

# c. both chromes, pixel for pixel
$RUN  --snap $OUT/rust.png --at 0.05
$LUAU --snap $OUT/luau.png --at 0.05
python3 tools/diff.py $OUT/rust.png $OUT/luau.png

# d, e, f and the cost table: no GPU, and it prints what it measured
VK_ICD_FILENAMES=/nonexistent cargo test --test luau_toolbar -- --nocapture --test-threads=1
```

Hot reload by hand: run `rune-shell FILE.canvas --chrome luau` in a window
and save `panels/toolbar/init.luau`. That has not been run (no display);
the test does the same through the file watcher on a copy of the script.

What crosses the boundary:

- Down: `require("@specular/app").toolbar`, a signal the host fires with
  the toolbar as plain data when `toolbar(&App)` differs from what it last
  sent. The script keeps the table and renders from it.
- Up: `app.run(id)`, where `id` is a control of that data. The host looks
  the id up in the model as it is then and sends that control's `Action`
  through `update`.
- `ui.control` is a node the host adds for the roles, `checked` and
  `hasPopup`, which `@rune/ui` has no prop for.

## The spike's own flags

Everything else on the command line is Rune's (`--snap`, `--tree`,
`--script`, `--size`, `--scale`, `--at`).

- `FILE.canvas`: the canvas to open.
- `--camera x,y,zoom`: as `--snapshot-camera`. Without it the document is
  fitted.
- `--chrome rust|luau`: what draws the toolbar. `rust` unless said.
- `--panel DIR`: `--chrome luau` with the toolbar script from `DIR`.
- `--still`: hold the app's clock and each synthetic page's first frame, as
  the reference snapshot does. Without it the clock is Rune's, which is
  virtual in a headless run.
- `--backend wgpu|metal`: the GPU backend on macOS. `wgpu` unless said;
  `metal` is Rune's own, which has no multisampling, so it needs
  `--samples 1`.
- `--samples 4|1`: samples per pixel of our scene pass
  (`Compositor::with_samples`). 4 unless said.
- `RUNE_SHELL_PACE=1`: print each burst of frames (frames under 100 ms
  apart) when a gap ends it: frame intervals, our render's CPU time and
  Rune's `update_ms`. A headless run prints its one burst in the summary.

- `--source synthetic|cef`: what hosts the pages. `synthetic` unless said;
  `cef` needs `--features cef` and the bundle (S5 below).
- `--pump timer|caller`: with `--source cef`, who turns CEF's loop.
  `timer` (the default) is `specular-cef`'s run-loop timer, as in a window;
  a headless run spins the main run loop for 4 ms each poll so the timer
  and the wake observer fire. `caller` pumps from each poll, as the real
  shell's headless run does.
- `--probe-keys SECONDS`: at that time, digit 1 then keypad 1, built from
  their physical keys as `on_key` builds them. Rune's script key names
  have no keypad keys.

With none of Rune's headless flags it opens a window.

## S4: macOS

```sh
cargo build --release --features tooling   # --snap and friends in release, for timing

# the same snap on Rune's Metal backend, single-sampled, and against wgpu
$RUN --backend metal --samples 1 --snap $OUT/metal1.png --at 0.2
$RUN --backend wgpu  --samples 1 --snap $OUT/wgpu1.png  --at 0.2
python3 tools/diff.py $OUT/wgpu1.png $OUT/metal1.png --skip-top 48

# a window; RUNE_SHELL_PACE prints frame bursts to stderr
RUNE_SHELL_PACE=1 target/debug/rune-shell ../../fixtures/kitchen-sink.canvas \
    --camera 20,60,0.25 --window-script scripts/open-zoom.json
```

Canvas input: `src/input.rs` puts one window-sized element under the
toolbar. Its `on_drag` takes a press, drag and release (primary, secondary,
middle) with the run of clicks, `on_pointer_move_local` the hover,
`on_wheel` the scroll (Cmd zooms), and `on_magnify` a pinch or Control and
the wheel. Keys go to the window's `on_key` (presses nothing focused took,
modifier keys alone with `App::modifier_keys`) and the root's `on_key_up`;
composed text to `App::on_ime` while the app turns the input method on.
Everything goes through `Shell::dispatch`.

```sh
# drag sticky-color-3, wheel up, Cmd+Z: prints the sticky selected; the
# snaps show it dragged at 0.5 and back in place, panned, at 1.2
$RUN --script scripts/drag-sticky.json --snap $OUT/dragged.png --at 0.5
$RUN --script scripts/drag-sticky.json --snap $OUT/undone.png --at 1.2
```

Rune's `--window-script` replays taps and keys in a real window, timed from
its first frame; it refuses `resize`. To capture or click the window with
macOS tools, wrap the binary in a throwaway `.app` with a bundle id: an
unbundled process is not offered to them.

## S5: CEF pages

`--features cef` links `specular-cef` (off by default; the default build is
unchanged). CEF needs the `.app` layout: the bundle script copies the
binary into the five helpers, and `main` sends Chromium's children to
`run_subprocess_if_needed` first. Run the bundle's inner binary.

```sh
export CEF_PATH="$HOME/.local/share/cef"      # the CEF 154 download, reused
cargo build --features cef
CARGO_TARGET_DIR=$PWD/target ../../crates/specular-cef/scripts/bundle-macos.sh debug rune-shell
CEF=target/debug/rune-shell.app/Contents/MacOS/rune-shell
PAGES="fixtures/cef-pages.canvas --camera 100,140,1 --source cef"

# two pages and a sticky over both; waits for both to paint first
$CEF $PAGES --snap $OUT/cef-rest.png --at 0.5

# select then enter page A, click its field, digit 1 (script) then digit 1
# and keypad 1 (probe), type "ab", compose にほん and commit 日本, then
# enter page B and wheel it; a close request at 4.7. --realtime gives CEF
# wall-clock time. Each page title change prints (the key logger puts the
# last event in it). --at 5.5 runs past the close.
$CEF $PAGES --realtime --script scripts/cef-keys.json --probe-keys 1.5 \
    --snap $OUT/cef-keys.png --at 4.5

# a close request: the window waits for CEF to stop, then closes
$CEF $PAGES --realtime --script scripts/cef-close.json --snap $OUT/close.png --at 2.0
$CEF $PAGES --pump caller --realtime --script scripts/cef-close.json --snap $OUT/close.png --at 2.0

# a window (not run by the spike's agents): pages paint with no input
$CEF fixtures/cef-pages.canvas --source cef
```

After a run the summary says how page frames came (`shared (IOSurface)`
or `cpu`), the mean and longest delay from CEF's paint to the poll that
took it, and how often the observer woke Rune.
