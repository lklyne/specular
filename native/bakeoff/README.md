# Render and UI bake-off (task M1)

Throwaway code behind [ADR 0039](../../docs/adr/0039-rust-canvas-render-stack.md).
A standalone cargo workspace. It is not a member of `native/Cargo.toml` and the
main workspace gate does not apply to it.

Each candidate draws the same scene (500 sticky notes with wrapped text, 200
freehand strokes, 100 arrows) into an offscreen 1600x1000 target, writes PNGs
at zoom 0.05, 0.25, 1 and 3, paints egui panels over a zoom-1 frame, then
prints frame times for a zoom sweep and four pans. No window opens.

```sh
cargo run -p bakeoff-vello            # candidate A: vello + parley
cargo run -p bakeoff-sdf              # candidate B: SDF rects + glyphon + lyon
cargo run -p bakeoff-sdf -- --lod     # skip text under 2.5 px
cargo run -p bakeoff-sdf -- --retained --out /tmp/frames
```

The dev profile is optimised (`opt-level = 3`), so there is no need for
`--release`.

- `crates/scene` builds the scene, the camera paths, the PNG readback and the timing loop.
- `crates/ui` is the egui toolbar and sidebar, drawn in a second pass over the canvas target.
- `crates/vello`, `crates/sdf` are the two canvas candidates.
- `gpui-proof/` is a separate workspace for the GPUI candidate.
- `golden/` holds the centre crops the ADR judges, and 5x side-by-side magnifications (vello left, SDF right).
