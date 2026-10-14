# Capturing the web build

These scripts produced the ground truth `specular-prism` is checked against (ADR 0047): every
WGSL module, pipeline, uniform write, texture and per-pass readback of the Prism scene in
lyle-klyne's site (`src/content/builds/2026-10-11-prism/`), on a fixed 60 Hz virtual clock.

The scripts write next to themselves: `../reference/_raw/` for the raw capture and `../reference/`
for the processed files. Copy this folder into a scratch directory (for example
`<scratch>/scripts/`) and run it from there, so the ~550 MB capture lands outside the repo.

## Run

1. Start lyle-klyne's dev server (`pnpm dev`, port 4321).
2. In lyle-klyne's `webgpu-canvas.tsx`, add `;(window as any).__prismRenderer = renderer` after
   `await renderer.init()`, temporarily. `uniform-labels.js` needs it to name uniforms. Revert it afterwards.
3. Start a separate Chrome: `--remote-debugging-port=9333 --user-data-dir=<scratch>`.
4. `python3 upload-server.py` (it listens on :8765 and writes POSTed files under `../reference/_raw/`).
5. `./capture-all.sh` (about 20 s).
6. `python process.py` (needs numpy and pillow) turns `_raw/` into `reference/`: `passes.md`,
   `wgsl/`, `uniforms/`, `textures/`, `geometry/`, `pass-outputs/`, `shots/`.
7. `python3 dump-uniform-bytes.py` writes `reference/uniform-bytes/` for the parity example's uniform check.

Then, from `native/`:

```sh
cargo run -p specular-prism --example parity -- <scratch>/reference <out>
SPECULAR_PRISM_REFERENCE=<scratch>/reference cargo test -p specular-prism
```

## What the hook does

`gpu-hook.js` is injected before the page's scripts. It patches `GPUDevice`, `GPUQueue` and the
encoders to record every object. Descriptors are deep-copied at call time because three reuses them.
It shadows every `writeBuffer`, adds `COPY_SRC` to textures and the canvas so each pass can be read
back, and snapshots each render pass's colour attachments right after `end()`. It also replaces
`performance.now` and rAF timestamps with a virtual clock, so `__gpu.advance(n)` steps exactly n
frames and frame N is the same on every run. `cdp.mjs` is the small CDP driver that loads the page
and evaluates the `dump-*.js` snippets.
