# rune-shell: the Rune spike

Throwaway code for [`docs/plans/rune-spike.md`](../../../docs/plans/rune-spike.md)
and [ADR 0051](../../../docs/adr/0051-rune-shell-around-the-compositor.md).
Steps S1 to S3 are here. The findings are in the plan's log, not in this file.

One binary, `rune-shell`: a [Rune](https://github.com/anomalyco/rune) app
whose frame is drawn by `specular-compositor` on Rune's device, with a
toolbar Rune draws from `toolbar(&App)` over it.

Rune is pinned to `a1fbe7ff5a33663a87edbdffbbcc37260c549106` (`dev`,
2026-11-24), with its eight `[patch.crates-io]` entries at the same revision.

## What is where

| File | What |
|---|---|
| `src/shell.rs` | The Rune app's state: an `App`, the synthetic pages, a cut-down effect runner. No GPU |
| `src/files.rs` | Image files to pixels, as the real shell decodes them |
| `src/chrome.rs` | The Rune view: one keyed element per control of the toolbar model |
| `src/canvas.rs` | The Rune `Compositor` that hosts ours and draws Rune's UI over it |
| `tests/toolbar.rs` | The GPU-free `Runtime` test (question 4) |
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

## The spike's own flags

Everything else on the command line is Rune's (`--snap`, `--tree`,
`--script`, `--size`, `--scale`, `--at`).

- `FILE.canvas`: the canvas to open.
- `--camera x,y,zoom`: as `--snapshot-camera`. Without it the document is
  fitted.
- `--still`: hold the app's clock and each synthetic page's first frame, as
  the reference snapshot does. Without it the clock is Rune's, which is
  virtual in a headless run.

With none of Rune's headless flags it opens a window. That has not been
run: there is no display here.
