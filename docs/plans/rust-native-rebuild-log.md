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
- F3: the `.canvas` writer is canonical, not byte-preserving. It writes what the Electron writer would: `specular.entityOrder` whenever the stack is non-empty, `annotations` only when there are some, nodes and edges in stack order, every float rounded to a hundredth except under a `zoom` key, whole floats as integers.
- F3: a node, edge or annotation that cannot be typed (unknown node `type` or `shapeKind`, missing required field, duplicate id) is kept as raw JSON in `Document::extra` under `nodes`, `edges` or `annotations` and written back after the typed items. The app does not see it. A load fails only on invalid JSON, a non-object top level, or one of those three keys not being an array.
- F3: group `pageIds`/`entityIds` are dropped on load and not regenerated. The Electron reader and writer no longer use them; membership is each member's `parent`. Page `groupId` and group `groupColor` are still written beside `parentGroupId` and `color`.
- F3: `Command::SetAnchor` carries `Option<Box<PageAnchor>>`, like the other boxed payloads, so the enum stays small.

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

### F3 — see `git log -- native/crates/specular-doc/src/canvas.rs`

- `specular-doc` now reads and writes `.canvas`: `Document::from_canvas_str`, `from_canvas_value`, `to_canvas_value`, `to_canvas_string` (two-space indent, no trailing newline), and `CanvasError`. Code is `src/canvas.rs` plus `src/canvas/{read,write,fields}.rs`.
- All six kinds, edges, annotations and the stack order are typed. Unmodeled fields stay in the `extra` of the item they sat on, leftover `specular` keys included. An optional value that does not fit its field (`"syncId": null`, an unknown `edgeKind`, a `null` label) stays in `extra` and the typed field reads as absent; a typed value set later wins over the leftover on save.
- Tests: 54 in the crate. `tests/canvas_repo.rs` loads and saves every `.canvas` under `tests/integration`, `resources/starter-space`, `native/fixtures` and `native/crates` and compares JSON values. `tests/canvas_fixtures.rs` is the spike's fixture suite ported onto `Document` and `History`, with its two fixtures copied to `tests/fixtures/`.
- For F4: load with `from_canvas_str`, then keep `History` beside the document. `appState` (zoom, pan, selection, panel state) is untyped in `Document::extra()["appState"]`; read the camera from it and write it back through `extra_mut()`. Save is `to_canvas_string`; the shell does the file write.
- For F4: removing an entity with its edges is a `Command::Batch` the caller builds. `remove_with_edges` in `tests/canvas_fixtures.rs` shows it.
- Saved keys come out alphabetical, not in Electron's order, so the first native save of an Electron file is a large diff with the same JSON value. The writer already inserts fields in Electron's order. To get that order in the file, turn on `serde_json`'s `preserve_order` and change `remove` to `shift_remove` under `src/canvas`. It cannot go on yet: it reorders the yrs document's output and fails `saving_a_reloaded_document_is_byte_stable` in `specular-core`. Do it when F4 deletes that document. With it on, `rich-workspace.canvas` saved byte-identical to what Electron wrote.
- No repo fixture has annotations, so annotation reading is tested on hand-written JSON only. `replies` is still required; an annotation without it is kept raw.
- Gate: fmt, clippy and tests pass for every crate except `specular-interact`, which another agent had half-written at the time (module files missing).
