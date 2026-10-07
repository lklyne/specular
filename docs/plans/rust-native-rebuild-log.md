# Rust native rebuild: run log

Appended by each agent as it finishes a task from
[`rust-native-rebuild.md`](./rust-native-rebuild.md). Newest entry last.

## Decisions

Choices made during the run that the plan did not settle. One line each,
with the task that made it.

- Orchestrator: yrs is dropped. The document is typed structs with inverse-command undo.
- Orchestrator: task F1 (rename and split crates up front) is folded into F4 and F5. New crates are created fresh beside the spike's crates; the old ones are renamed or absorbed when their replacement lands.
- Orchestrator: no PRs or pushes during the run. One commit per task on the current branch.
- F2: ids are the `.canvas` id strings (`EntityId`, `EdgeId`, `AnnotationId`), not slotmap keys. An undo has to restore the same identity, and edges, `parent` and anchors refer to ids by string on disk.
- F2: `specular-doc` has its own `Rect` and `Point` in `f64`. `.canvas` numbers are JSON doubles and `specular-core`'s `f32` `CanvasRect` would change them on a load and save.
- F2: the stack order is `Vec<ItemId>` where `ItemId` is `Entity(id)` or `Edge(id)`, because edges interleave with entities in `specular.entityOrder` (ADR 0014). Entity and edge ids share one namespace.
- F2: commands are primitive and never cascade. References may dangle (files from other tools can contain them). Delete builds a `Command::Batch` from `Document::children` and `Document::edges_touching`.
- F2: `History` is a separate struct from `Document`. `Document::apply` alone is not an undo step, which is how loading and non-undoable changes are done.
- F2: `label` sits on `Entity`, not in each kind. Five of six kinds have one, and the file kind has it under `specular.label`.
- M1: canvas items are SDF shapes + glyphon 0.12 (cosmic-text 0.19) + lyon in the compositor's pass; panels are egui 0.36; vello and GPUI are turned down. ADR 0039, Proposed.
- M1: text editing uses cosmic-text's `Editor`, not parley's `PlainEditor`, because glyphon renders cosmic-text buffers.
- M1: `native/bakeoff/` sets `opt-level = 3` on its dev profile so timings mean something without `--release`.

## Needs a human at a Mac

Things an agent could not verify headless.

- M1: the four checks at the end of ADR 0039 (sharpness on a real display, glyph shimmer while zooming, egui's look, IME into an egui field).

## Entries

### F2 — `359df918`

- New crate `native/crates/specular-doc` (deps: serde, serde_json, thiserror). `Document`, `Entity`, `Kind` (all six variants with their fields), `Edge`, `Annotation`, `PageAnchor`, `Color`, `Command` (15 variants incl. `Batch`), `CommandError`, `History`. 28 tests; full workspace gate passes.
- `Document::apply(Command) -> Result<Command, CommandError>` returns the inverse. A refused command, or a `Batch` with one refused member, leaves the document unchanged.
- The yrs document in `specular-core` is untouched and still used by `specular-app`. Delete it (and the `yrs` workspace dep) when F4 moves the shell onto `specular-doc`.
- For F3: `Edge`, `Annotation`, `PageAnchor`, `Stroke` and every value enum already derive serde in wire shape with a flattened `extra`. `Entity`, `Kind` and `Document` do not; write wire node structs and convert. Build a `Document` by applying `InsertEntity`/`InsertEdge`/`InsertAnnotation` and dropping the inverses; top-level leftovers go in `Document::extra_mut()`.
- For F3: a wire value that does not fit its typed field (`"syncId": null`, an unknown `shapeKind` or `edgeKind`) should stay in `extra` rather than fail the load. `Color::Neutral` on a node is `color: "1"` plus `specular.colorRole`. Group `pageIds`/`entityIds` and `groupColor`, and page `groupId`, are derived on disk: regenerate them on save, do not keep them in `extra`.
- For F3: `f64` fields serialize whole numbers as `100.0`, which is not equal to `100` as a `serde_json::Value`. Normalize the value tree once on save (the Electron writer does the same pass to round to 2 decimals).
- For F3: `Annotation.replies` is required and anchor variants have no `extra`. Loosen if a real file disagrees.
- For S3: drawing stroke points are in canvas space, so moving a drawing is `Batch[SetRect, SetKind]`.
- Not done from the plan's F2 line: porting the spike's fixture round-trip tests. They need the reader, so they belong to F3.

### M1. Render and UI bake-off

- Commit: see `git log -- native/bakeoff`.
- Exists now: `native/bakeoff/`, a standalone workspace with a vello candidate, an SDF + glyphon + lyon candidate, an egui panel crate and a GPUI proof in its own workspace. `docs/adr/0039-rust-canvas-render-stack.md` has the numbers and the verdict.
- vello 0.11.0, glyphon 0.12.0 and egui-wgpu 0.36.2 all build on wgpu 30.0.1 with winit 0.30.13. Add them to `[workspace.dependencies]` at those versions when K1 needs them.
- For K1 to K6: `Scene` carries shapes, text runs and paths, with no renderer types. `specular-render` owns the glyphon atlas and the lyon tessellator, in a 4x MSAA pass.
- For whoever builds `specular-render`: pages and items share one z-order, so batches break at each page. Glyphon draws all prepared text in one call, so use one `TextRenderer` per run of items, or depth. The bake-off did not build this.
- Skip text under about 2.5 px on screen. It removes the worst frame times.
- Zooming re-rasterises glyphs at each scale and misses 8.3 ms at p95. Hold the raster size during a zoom gesture and refresh on settle.
- `gpui-proof` needs the `runtime_shaders` feature here because the Xcode Metal toolchain is not installed. Its `target/` is 2.6 GB and can be deleted.
