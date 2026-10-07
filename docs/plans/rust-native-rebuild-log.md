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
- F4: `Event` and `Effect` name a page by its `EntityId`. The shell keeps the table from entity to backend `PageId`.
- F4: a drag writes rects into the document as it goes, through `Document::apply`. The release puts the start rects back and records one `History` step; Escape just puts them back.
- F4: a page's viewport is not stored. It is the rect's rounded size, held at the starting size while a handle is dragged. Undo, redo and opening a document diff the pages before and after and return create, close and viewport effects.
- F4: `Tool` has eight variants: Electron's ten without `hand` and `inspect`. `Gesture` has only the three that work (`Move`, `Resize`, `CommentRegion`); each slice adds its own.
- F4: time comes in as `Event::Tick { unix_ms }` once per loop turn, so there is no timer effect. S9 can debounce against it. New ids come from a seeded sequence in `Session`.
- F4: a comment region over a page is stored the way Electron stores it: a `docRect` in the page's CSS pixels plus a `pageAnchor`. Scroll is taken as zero until pages report it.
- F4: `Action` is the command enum for key bindings, menus, panels and API "act" routes. There is no reply effect; A1 adds what it needs.
- F4: canvas bindings (C, Cmd+Z) go to the page while a page has keyboard focus, as Electron's undo binding does. Escape always cancels.
- F4: `--chrome off` only stops the drawing. Gestures and keys still act.
- F6: tests on the testkit are integration tests under a crate's `tests/`. A `src/` unit test would see two copies of its own crate's types, because the testkit links the library build.
- F6: a document snapshot is the canonical save with one compact JSON line per node, edge and annotation. It needs no per-kind code, so a new kind or field shows up in snapshots without touching the testkit.
- F6: `hold(mods)` keeps modifiers down until `let_go()`. `key` and `chord` send the press and the release. `release()` is always at the pointer's last position.
- F6: the golden-image helper in the plan's F6 line is left for the task that makes the renderer draw a `Scene`.
- F5a: `Scene` names a page by its `EntityId`, as events and effects do. `render_scene` takes a closure from `EntityId` to the backend `PageId`, so `view` needs no handle table. Images are named by `ImageId(u64)`, uploaded with `Compositor::set_image`.
- F5a: every item is in canvas space or screen space. There is no "canvas rect with a pixel-wide stroke" item. Chrome that hugs an entity at a fixed pixel size is a screen-space item that `view` projects with the camera.
- F5a: scene colors are 8-bit sRGB with straight alpha. Clip and opacity are per item, with no push and pop. A clip is a rect in the item's own space.
- F5a: a text run has an origin plus an optional wrap width and box height. An axis with an extent aligns inside it, an axis without one aligns against the origin. The renderer shapes and measures, so `view` never needs text metrics.
- F5a: batches do not always break at a page. An item joins the earliest batch of its kind at or after the last batch it overlaps, and a page is a batch of its own. An item over a page still paints after it. Border and title chrome beside 40 pages is one shape batch and one text batch, not 40 of each.
- F5a: dashed borders and dashed edges are paths with a `Dash`. The SDF layer draws solid rects and ellipses only. Rect and ellipse strokes can sit inside, centred or outside.
- F5a: a stroke thinner than one device pixel is drawn one pixel wide and faded by the same ratio.
- F5a: the caller says when the camera is zooming (`FrameView::zooming`). While it is, canvas glyphs keep their raster size until the zoom has moved 0.75x to 1.25x from it, and the pass viewport stretches them. The shell must render one frame with `zooming: false` when the gesture ends.
- S1: hit-testing runs in screen space with Electron's sizes (12 px handle squares and side strips on the outline 1 px outside the bounds, first match wins). Pages and other items share one stack order, so a page in front of a note covers it; Electron always puts notes above pages. Groups are still hit after everything else.
- S1: `Hit` has no reorder dots or gap handles. They arrive with auto-layout (ADR 0015). A group title's width is estimated at 6.1 px a character until text is measured.
- S1: the per-kind rules are `min_size`, `aspect_mode` and `has_anchors` in `caps.rs`. The page minimum is now Electron's 320x200, up from the spike's 120x80.
- S2: the entered page of ADR 0022 is `Focus::Page`. It stays entered only while it is the whole selection, which `update` checks once after every event. Only the entered page gets pointer and key input, and a page that got a press keeps the pointer until the release.
- S2: Escape is staged. It first backs out of a drag, an armed tool or an entered page and keeps the selection. With none of those it deselects.
- S2: a marquee changes the selection on release. Until then `App::marquee()` and `App::marquee_items()` give the rect and what it would take.
- S2: `Session::hover` is the entity under the pointer, of any kind. The hovered entity shows anchors, as in Electron.

- F5b: `view(&App, viewport)` culls entities outside the viewport (plus 64 px for chrome), so a frame costs what is on screen. `view_without_chrome` is what `--chrome off` draws: entities and edges, with no page border or title and no session layer.
- F5b: `specular-scene` depends on `specular-interact` (and so does the compositor, through it). Edges are drawn from `App::edge_curve`, the curve hit-testing uses, in screen space.
- F5b: colours are the light theme only. The vivid inks are the CSS `oklch(from hue 0.5 c h)` values clipped to sRGB and written as constants in `view/palette.rs`. Blue is stored as `"7"`, which `specular-doc` reads as `Color::Custom("7")`; the palette maps it.
- F5b: drawings are outlined in canvas space (Electron outlines them in screen space), so a stroke has the same shape at every zoom. The highlighter is a flat 30% alpha with no gradient or grain: the scene has neither.
- F5b: a page keeps the 8-unit corner radius and gets a title above it (label, or URL without the scheme) as the title-bar stand-in. Electron draws neither on the canvas.
- F5b: text is never measured in `view`. An edge label has no gap cut in the line under it, a comment badge has a fixed width per digit, and a file card stacks its glyph and one line of name around the centre.
- F5b: a comment on a canvas point draws a 12 px dot and a comment on an element draws its badge in the page's top-right corner. Electron shows nothing for the first and needs the element's live position for the second.
- S9: `History::revision()` counts applied, undone and redone steps. `update` compares it before and after an event and returns `Effect::Save` when it moved, so no command site has to remember to. `clear` does not move it: a document just read is not saved back.
- S9: the file watch is a `stat` every 500 ms on the loop, not an OS watcher. A moved stamp (mtime or length) means read the file; the text decides. Our own write and a `touch` compare equal to what we hold and are ignored.
- S9: nothing is saved or reloaded while a gesture is in flight. The document holds the drag's unfinished rects, which Escape takes back.
- S9: the camera is written only when a document change saves. Panning alone does not write the file. A pending save is flushed on exit.
- S9: a run with `--bench` or `--annotations N` never writes the file or reads its camera. A file with no `appState` camera opens at the old fixed start camera.
- S3: a plain drag on a body moves the selection. Option held during the drag makes it a copy, as in Electron. The spike's Alt+drag move is gone.
- S3: a move snaps to the 20-unit grid, as Electron's does. The pressed entity's top-left lands on a grid line and every other operand moves by the same delta, so a selection keeps its layout. Electron snaps each entity on its own. A pressed drawing does not snap.
- S3: entering a page (ADR 0022) happens when the click is released, not on the press, because a press on the selected page may turn into a drag.
- S4: resize is computed from the start rect and the pointer each frame, not from accumulated deltas as in `resize-accumulator.ts`. The results match except past a limit, where Electron's version drifts from the pointer.
- S4: Option does nothing in a resize, as in Electron. Shift follows the kind's `AspectMode`: shapes and non-media files lock with Shift, text and image or video files unlock with it.
- S4: text height is content-sized and nothing measures text headless. A scaling drag that keeps the ratio writes the scaled height as a stand-in; reflow and Shift drags leave the height alone.
- S4: resizing a page writes no `pageSizeMode` or device metadata and leaves `preset_index`. The viewport is still the rect's size (F4).
- S5: copies keep their group unless the group is copied too, lose a page anchor unless that page is copied, and take an edge only when both its ends are copied. They go in front of the stack.
- S5: duplicate places the copy 80 units to the right, else below, else at the first free spot of a grid scan. Every entity counts as occupied.
- S5: the cursor is recomputed after every event but a tick and returned as `Effect::SetCursor` only when it changes. `Session::cursor` holds the last one.
- K6: images are a table in `Session` keyed by the `file` string, each with an `ImageKey` that `update` allocates. `update` returns `Effect::LoadImage` when a document is opened and after any history step; the shell answers with `Event::Image`. The scene's `ImageId` is the key's number.
- K6: an image nothing shows any more is kept until another document is opened, so undoing a delete does not reload it. `Effect::DropImage` is only returned on `DocumentOpened`.
- K6: which files are images is Electron's `IMAGE_EXTENSIONS`, checked in `update`. What can be decoded is the shell's business: svg, bmp and ico are asked for, fail, and stay cards. An `http(s)` path is not fetched and fails too.
- K6: the decode thread also premultiplies and builds the mip levels (`ImageMips::build`, in the compositor crate but pure CPU). The main thread only uploads, one image per loop turn.
- K6: an image larger than the device's texture limit is scaled down on the decode thread. EXIF orientation is applied, as a browser does for an `<img>`.
- K6: `contain` draws only the image, with nothing in the letterbox bars. `cover` crops with `ImageDraw::source`. No corner radius.

## Needs a human at a Mac

Things an agent could not verify headless.

- M1: the four checks at the end of ADR 0039 (sharpness on a real display, glyph shimmer while zooming, egui's look, IME into an egui field).
- F5a: zoom with canvas text on screen once `view` lands. glyphon samples its atlas with a nearest filter, so held glyphs stretched up to 1.25x may look blocky or shimmer mid-gesture. If so, narrow `MIN_STRETCH` and `MAX_STRETCH` in `scene_pass/raster_hold.rs`.
- F5b: run `specular-app` on `resources/starter-space/Welcome.canvas` and on `fixtures/input.canvas`. An offscreen render of the first looked right, but the shell switch itself was not run. Check the page titles, the 8 px handles on the outline, a marquee, and that text sharpens one frame after a zoom stops. Small canvas text came out grey rather than near-black in the offscreen render; compare on a real display.
- F5b: one `--bench` run with `--chrome on` against the last build. Each page now has a title (one text run) and each seeded annotation is a dashed path, tessellated per frame, where it was two SDF shapes. `max_shapes_drawn` will read lower.
- F4: nothing was run. Agents may not start `specular-app`, so check `specular-app fixtures/input.canvas` by hand (click, type, Alt+drag, corner resize, C then drag, Escape, Cmd+Z and Cmd+Shift+Z with no page focused) and one `--bench` run against an older build for output shape and frame times.
- S1 and S2: nothing was run. With `specular-app fixtures/input.canvas`, check that one click selects a page without the page reacting, a second click or a double-click lets you type into it, Escape leaves it, and a drag from empty canvas does not scroll or select text in a page.
- S9: nothing was run. Open a copy of a canvas, move something, and check the file changes about a third of a second later with the camera in `appState`. Edit the file in an editor while the app is idle and check the canvas follows, keeping the camera. Quit within 350 ms of a change and check it was written.
- S3 to S5: nothing was run. Check drag feel against the grid, Shift mid-drag, Option-drag (the copy preview is not drawn yet), each handle on each kind, a two-item resize, Backspace, Cmd+D, arrows, and the corner cursors.
- K6: nothing was run. Open a canvas with png, jpeg, webp and gif files beside it (relative `assets/...` paths), one missing file and one svg. Check each image appears a moment after the card, keeps its aspect, stays smooth when zoomed far out, and that the missing file and the svg stay cards.

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

### F4 — see `git log -- native/crates/specular-interact`

- New crate `specular-interact` (deps: `specular-doc`, `specular-core`, glam, tracing): `App`, `Session`, `Selection`, `Focus`, `Event`, `Action`, `Effect`, `Cursor`, `Tool`, `Gesture`, `Hit`, `hit_test`, `PagePlacement` and `update(&mut App, Event) -> Vec<Effect>`. 69 tests, all scripted events through `update`.
- Ported onto it: page move (Alt+drag), corner resize, the comment-region drag, click to select and focus, pointer, wheel, key and IME forwarding with per-button capture, pan, zoom, pinch, Escape. New: Cmd+Z and Cmd+Shift+Z through `History`, covering moves, resizes and comment regions.
- `specular-app` is now `Shell`: `translate.rs` and `app/input.rs` turn winit into `Event`s, `app/effects.rs` runs `Effect`s. `chrome_state`, `annotation`, `placement`, `handles` and `input_map` are gone from it.
- F3 landed mid-task, so the shell loads with `Document::from_canvas_str`, and the yrs document, `json_canvas`, its tests and the `yrs` dependency are deleted from `specular-core`. Next agent in `specular-doc`: turn on `serde_json`'s `preserve_order` now, as F3's entry describes.
- The camera still starts at the shell's fixed `START_CAMERA`, not the file's `appState`, so bench runs stay comparable. Nothing saves yet: `Effect::Save` and `Effect::WriteClipboard` exist for S9 and S6 and the runner only logs them.
- For F5: `chrome.rs` in the shell is the stand-in for `view`. It reads `App::pages`, `App::handle_target`, `Session::comment_preview` and `region_on_canvas`; move it into `specular-scene` and delete it. Non-page kinds load but are not drawn or hit.
- For S1 and S2: `hit_test` knows handles and pages only. A click on a page still selects, focuses and forwards at once; select-first (ADR 0022) is not in. Only pages get handles (`min_size` in `handles.rs` is the per-kind `match`).
- For S8: `keys.rs` is three hard-coded bindings behind a `Route`; replace it with the table.
- For F6: `src/tests/mod.rs` has the press, drag, release and key helpers to lift into the testkit.
- A page whose URL changes is closed and created again, and loses keyboard focus on the way. P3 needs a navigate effect.

### F6 and the `preserve_order` cleanup — see `git log -- native/crates/specular-testkit`

- `specular-doc` turns on `serde_json`'s `preserve_order` and uses `shift_remove` under `src/canvas`. Saved keys are in Electron's order. `canvas_repo.rs` checks that both `rich-workspace.canvas` copies load and save to the same bytes.
- Not byte-identical yet: `Welcome.canvas` (and `tests/fixtures/pages.canvas`, which also lacks `entityOrder`). A `"syncId": null` goes to `extra` and is written after the typed fields, so it moves down its node. The fix is for the writer's `put` to write an `extra` value in the typed field's slot when the typed field is absent. The `native/fixtures` files differ because they have no `specular.entityOrder`.
- New dev-only crate `specular-testkit` (deps: doc, core, interact, glam, insta, serde_json): `TestApp` with `from_canvas`, `with_pages`, `with_entities`, `from_document`, `empty` + `open`; chainable input in `src/input.rs`; `take_effects`; `undo`/`redo`; `doc_snapshot`, `assert_doc_snapshot!` and `assert_undo_returns_to_start`. `native/CLAUDE.md` "Adding a feature" lists the calls.
- `specular-interact/src/tests/gestures.rs` is now `specular-interact/tests/gestures.rs` on the testkit (26 tests). `src/tests/routing.rs` still uses the helpers in `src/tests/mod.rs`; move it over and delete them when someone is next in there.
- For F5: the scene hook is the comment at the end of `specular-testkit/src/snapshot.rs`. Add the `specular-scene` dependency, `scene_snapshot`, `TestApp::scene_snapshot` and `assert_scene_snapshot!` there.
- `cargo-insta` is not installed on this machine. Inline snapshots were written by hand from the failure output; `cargo install cargo-insta` makes that one command.
- Gate: fmt and `cargo test --workspace` pass. Clippy passes for `specular-doc`, `specular-interact` and `specular-testkit`; it fails on dead code in `specular-compositor`, which the F5 agent was editing (retried once).
- The commit includes all of `Cargo.lock` as it stood, which has the F5 agent's `specular-scene`, glyphon and lyon entries. Only the testkit and insta lines of `Cargo.toml` are staged.

### F5a — see `git log -- native/crates/specular-scene`

- New crate `specular-scene` (dep: `specular-doc`, for `EntityId`). Types only: `Scene`, `Item` (`Space`, clip, opacity), `Draw` with `PageDraw`, `RectDraw`, `EllipseDraw`, `PolygonDraw`, `PathDraw`, `TextRun`, `ImageDraw`. `Draw::bounds()` gives the extent of everything but text. There is no `view` yet.
- `Compositor::render_scene(target, &FrameView, &Scene, page_of)` is the new entry point and returns `SceneStats`. `render`, `SceneView`, `ShapeDraw` and `RenderStats` are unchanged and `specular-app` still uses them. The code is in `specular-compositor/src/scene_pass/`.
- One 4x MSAA pass that resolves into the target: grid, then batches in order. Rects and ellipses go to the SDF shader, polygons and paths through lyon 1.0.19, text through glyphon 0.12.0, pages and images through the quad shader. Text under 2.5 logical px is skipped and counted.
- Placement, batching, shape instances, meshes, dashes, text placement and the raster hold are pure, with 69 new unit tests. 22 GPU readback tests in `tests/scene_gpu.rs` and `tests/scene_text_gpu.rs` skip with no adapter. They cover page z-order, clips, opacity, HiDPI, an sRGB target and the held-glyph stretch.
- The text tests need a system font and only check where the ink is. A machine with an adapter and no fonts would fail them.
- For F5b (`view`): the shell's page table goes in as `page_of`. A page with no host or no frame is counted in `render.pages_without_texture` and skipped. Give sticky text a wrap width and a clip so off-screen notes are culled without being shaped. Pass the old `PAGE_CORNER_RADIUS` as `PageDraw::corner_radius`.
- For F5b: when the shell switches to `render_scene`, delete `SceneView`, `ShapeDraw`, `shape_list.rs`, `build_draw_list` and the single-sample pipelines. They are kept only for `specular-app`.
- Shared internals changed, with the old output kept: `QuadInstance` has a uv rect and an opacity, `ShapeInstance` has a stroke offset and a kind, and `fs_shape` composites the stroke over the fill. All 15 old smoke tests pass.
- Cost to know about: batching tests an item against the items of earlier batches, which is quadratic when shapes and text alternate. About 1,000 visible notes with readable text is roughly a million rect tests a frame. A grid would fix it if a bench shows it.
- Not measured: frame times. Nothing was run but the tests, and no `--release` build was made. Tessellation runs every frame for visible paths, as in the bake-off's default mode.
- The FontSystem loads on the first frame that has text, which takes a moment. The shell may want to warm it at startup.
- Gate: fmt, clippy and `cargo test --workspace` all pass.

### S1, S2 and the `.canvas` writer gap. See `git log -- native/crates/specular-interact/src/select.rs`

- `hit_test` returns `Hit::{GroupLabel, Handle, Anchor, PageContent, EntityBody, GroupBorder, Edge, Empty}` for every kind and for edges, in stack order. `Handle` is a corner or a side, and its `HandleOwner` is one entity or the whole selection. `tests/hit_test.rs` ports the cases from `tests/unit/hit-test.test.ts`, apart from reorder dots and gap handles.
- The select tool does click, Shift-click toggle, click on empty canvas to clear, click on an edge, and marquee with group promotion (Command or Control takes only what the rect encloses, and can start on a body). Pages are select-first. A double-click enters a page, as in Electron, and the entering click is not forwarded.
- ADR 0034 is in. `App::selection_scope()` returns `members`, `operands` (groups expanded, page-hooked items attached) and `bounds`. `holds(id)` is the rule that a press on any operand keeps the selection.
- `.canvas`: a leftover such as `"syncId": null` is written in its typed field's slot. `Welcome.canvas` now saves byte-identical and is in the byte test.
- `src/tests/` is gone. The routing tests are `tests/routing.rs` on the testkit, which gained `text`, `shape`, `file`, `drawing`, `group`, `inside`, `connected`, `selected_ids` and `press_button`.
- For S3: a press on a body selects and nothing else. The only move is still Alt+drag on one page. Build the move from `selection_scope().operands` and `marquee::DRAG_THRESHOLD`. A click (no drag) on one of several selected items should select it alone, and does not yet.
- For S4: only a page corner starts a resize. Every other handle takes the press and does nothing. `Gesture::Resize` still carries a `Corner`.
- For the edge task: an anchor press falls through to the body under it (`select::press`). `App::edge_curve(id)` is the bezier on screen, ported from `edge-geometry.ts`, and the edge view should draw from it so the line and its hit band agree.
- For the scene: draw `App::handles()` (eight handles, `OUTLINE_PADDING` outside the rect), `App::marquee()` and outlines for `marquee_items()`. The shell's `chrome.rs` still draws four corners from `handle_target()`, which now answers for any kind.
- Not done: double-click to edit text or a shape, to enter a group or to rename its title. Pressing or releasing Command mid-marquee changes the mode only at the next pointer move. No cursor feedback over handles.
- Gate: fmt, clippy and `cargo test --workspace` pass (624 tests). `Cargo.lock` and `specular-scene` had another agent's uncommitted changes, which are not in this commit.

### F5b — see `git log -- native/crates/specular-scene/src/view.rs`

- `specular_scene::view(&App, viewport) -> Scene` and `view_without_chrome`. `src/view.rs` holds the exhaustive `match` on `Kind`; `src/view/{page,text,shape,drawing,group,edge,file}.rs` draw the kinds and `session.rs` and `annotations.rs` the layer over them. `palette.rs` is `canvas-colors.ts`, `shape_path.rs` is `shapes.ts`, `freehand.rs` is perfect-freehand 1.2.3's `getStroke` for the drawing layer's options, tested against the library's own output.
- The shell calls `view` and `Compositor::render_scene`. `chrome.rs`, `SceneView`, `ShapeDraw`, `ShapeExtent`, `shape_list.rs`, `build_draw_list`, `Compositor::render` and the single-sample pipelines are deleted. `Compositor::warm_text` loads the fonts at startup. `FrameView::zooming` is true on any frame whose zoom differs from the last one drawn.
- `gpu_smoke.rs` is the frame-ingestion tests moved onto `render_scene`; its five shape tests went, `scene_gpu.rs` covers them. `tests/ink/` holds the helpers only the shape and text tests use.
- Testkit: `scene_snapshot`, `TestApp::scene_snapshot` and `assert_scene_snapshot!` in `src/scene_snapshot.rs`. 18 tests in `specular-scene/tests/view.rs`, one snapshot per kind and per session state, as `.snap` files (the lines are long).
- Session layer: 1 px outline per selected entity, four corner handles from `App::handles`, marquee rect plus outlines of `App::marquee_items`, an outline on `Session::hover`, the comment preview, and a selected edge in the selection colour. Nothing was added to `specular-interact`.
- For K1 to K6: the constants in each kind's module are Electron's light-theme values. Stickies have no shadow and nothing has a dark theme. `text_vertical_align` on a shape is honoured, which Electron does not do.
- For K6 and whoever draws images: a file is a card with a glyph and its name whatever its type. `ImageDraw` is unused by `view`.
- For E-tasks: an edge whose entity is missing draws nothing. Edge anchors on the selected entity are not drawn.
- Gate: fmt, clippy and `cargo test --workspace` pass, GPU tests included on this machine.

### S9 — see `git log -- native/crates/specular-app/src/persist`

- `update` returns `Effect::Save` after every history step, undo and redo. `specular-interact/tests/save.rs` checks it, and that a reload keeps the camera and drops dead selection ids. Three assertions in `tests/gestures.rs` gained the `Save`.
- `specular-app/src/persist/`: `file_sync.rs` is the pure part (350 ms trailing debounce, retry after a failed write, the disk-check timer, and the reload decision), `app_state.rs` reads and writes the camera in `appState`, `disk.rs` is `stat` and the temp-file-then-rename write, `mod.rs` is `Persistence`, which the shell calls once per loop turn.
- The shell opens at the file's camera, runs `Effect::Save`, and sends `Event::DocumentOpened` when the file changed and nothing of ours is unsaved. With unsaved changes it logs a warning and our save overwrites theirs. A file that no longer parses is logged and ignored.
- For whoever adds panels: `appState.selectedEntityIds` and the sidebar keys are kept as read but not updated.
- For S6 and later effects: `Effect::WriteClipboard` still only logs.
- Not done: a max wait on the debounce. Someone who changes the document at least every 350 ms for a long time is not saved until they pause.
- Gate: fmt, clippy and tests pass for `specular-doc` and `specular-app`, and for `specular-interact` on HEAD plus this change (checked in an exported copy). The working tree had another agent's move and resize work in `specular-interact`, whose `tests/moves.rs` fails clippy; none of it is in this commit.

### S3, S4 and S5. See `git log -- native/crates/specular-interact/src/move_drag.rs`

- Move: `Gesture::Move(MoveDrag)` in `move_drag.rs`. Any selection, built from `selection_scope().operands`, with the grid, Shift's axis lock and drawing points. A click on one of several selected items selects it alone.
- Resize: `Gesture::Resize(ResizeDrag)` in `resize_drag.rs`, math in `resize.rs`. All eight handles for one entity and for a selection. Text reflows from the sides and scales its size from the rest. Drawings scale their points. Pages get one `SetPageViewport` on release.
- Verbs in `verbs.rs`: `Action::Delete` (Backspace, Delete), `Action::Duplicate` (Cmd+D), `Action::Nudge` (arrows 5, Shift+arrows 20). Option-drag copy shares `clone.rs` with duplicate.
- `live.rs` is the shared drag plumbing: `Start` captures an entity, `write` is one frame with no undo step, `commit` makes the one step. New gestures should use it.
- `update::document_step` runs a command as one step and reconciles page hosts and the selection. Use it for any command that can add or remove pages.
- A modifier pressed or released mid-drag takes effect at once, marquee included. That closes the S2 note about Command mid-marquee.
- Not done: side handles show no resize cursor. `Cursor` needs `ResizeNs` and `ResizeEw`, and `specular-app/src/translate.rs` matches on `Cursor` with no wildcard, so adding them breaks the shell until it gets two arms. Then change `cursor::of_handle`.
- For the scene: `App::copy_preview()` gives the rects an Option-drag would leave copies at. Nothing draws them.
- Not done: dropping into or out of a group on release, re-resolving a page anchor after a move, alignment guides.
- `tests/gestures.rs`, the testkit's `driver.rs` and doc example, and the example in `native/CLAUDE.md` no longer use Alt+drag as a move.
- I ran `cargo fmt --all` once, which may have reformatted another agent's uncommitted compositor files.
- Gate: fmt, clippy and tests pass for `specular-doc`, `specular-interact` and `specular-testkit` on this commit alone, checked in an exported copy (302 tests). The workspace gate passed its tests once (759) and then failed in `specular-compositor`, `specular-scene` and `specular-app`, which other agents had mid-edit. Their image hunks in `specular-interact` are not in this commit.

### K6 — see `git log -- native/crates/specular-app/src/images`

- Interact: `images.rs` (`ImageKey`, `Image`, `ImageState`, `ImageNotice`, `is_image_file`), `Effect::LoadImage` and `DropImage`, `Event::Image`, `App::image(file)`. `tests/images.rs` covers the requests, the answers and the drop on reopen.
- Scene: `view/file.rs` emits an `ImageDraw` for a ready image and the card otherwise. `view/image.rs` is the `object-fit` math (contain by default, cover, fill). Two snapshots in `tests/images.rs`.
- Compositor: `ImageMips` and `ImageSpec` in `scene_pass/mips.rs`, `Compositor::image_spec` and `set_image_mips`; `set_image` now goes through them. The shared sampler filters between mip levels. A readback test in `scene_gpu.rs` draws a striped image at an eighth of its size and fails if the levels are missing.
- Shell: `images/resolve.rs` (space folder, absolute, `local-file://`), `images/decode.rs` (the `image` crate with png, jpeg, webp, gif only), `images/mod.rs` (`ImageLoader`, one thread), `app/image_run.rs` (the effects and the upload). The space folder is the `.canvas` file's directory; a demo grid has none, so only absolute paths load there.
- Not done from the plan's K6 line: dropping a file on the canvas and copying it into `assets/`. Nothing creates a file entity yet.
- Not done: reloading an image when its file changes on disk, animated GIFs, svg. A file entity that changes its `file` path through a command loads the new one and keeps the old texture until the next open.
- A load that can never be answered (no GPU window yet, or the thread failed to start) leaves the image `Loading`, which draws the card.
- `specular-app/src/app/mod.rs` is about 480 lines. The bench and LOD methods are the part to move out.
- Gate: fmt, clippy and `cargo test --workspace` pass, GPU tests included on this machine.
