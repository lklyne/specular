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

- Tools: a gesture that creates an entity puts it in the document while it runs, as a move writes rects. `App::creating()` names it. Release takes it out and inserts it as one `History` step; Escape takes it out.
- Tools: each stroke is its own drawing entity and its own undo step, which is what Electron's pointer-up does. `useDrawingSession` can hold several strokes but nothing adds a second one.
- Tools: tool defaults are `App::tool_defaults()`, beside the document and the session. `Effect::SaveToolDefaults` carries the whole value and `Event::ToolDefaultsLoaded` sets it. `ToolDefaults::to_json` and `from_json` are the preferences file's `toolDefaults` shape.
- Tools: a variant key (R, O, Shift+R, M, Shift+M) is `Action::SetToolVariant(patch)`, which arms the patch's tool and writes the default. A tool's key pressed again does nothing, so C no longer toggles the comment tool off.
- Tools: a chord's `cmd` is Command or Control, as Electron's `CmdOrCtrl`. Escape matches whatever modifiers are held, so it cancels a Shift or Option drag.
- Tools: `Session::editing` holds only while that entity is the whole selection. While it is set the plain-key bindings do not fire; undo, redo and Escape do. Only text and stickies set it. Electron also opens a new shape's label for editing.
- Tools: a new page is Electron's `P` then click: preset 0 (375x667), `about:blank`, with the device metadata Electron writes.
- Tools: an anchor written at placement has `pageId` and `pageUrl` and no scroll offset. `canonical_page_url` trims and strips the hash; it does not normalise the URL as Electron's `new URL()` does.
- T3: `view` cannot measure text, so it cannot stack a document's blocks. The scene has a new draw, `Draw::Column`: rows of `TextRun` cells that the renderer stacks, each row as tall as its tallest cell. A list item is a marker cell and a text cell, a table row is one cell per column.
- T3: `TextRun::spans` sets weight, italic, family, colour, underline and strike on byte ranges. There is no per-span size. A heading is its own row.
- T3: glyphon draws glyphs only. Underlines, strikes, quote bars, dividers and table lines go through its custom-glyph path as solid masks, so they keep the text's batch, clip and order. They are for thin lines. A fill that way would eat the glyph atlas.
- T3: the markdown parser is `specular-scene/src/markdown`, and `view` parses the text on every frame a Document is on screen. `App` holds only the text, as `NoteState`, keyed by the `file` string.
- T3: the note thread polls stamps every 500 ms, as S9 does, and reports a file only when its text reads differently. `Effect::LoadNote` means read it and keep watching until `Effect::DropNote`.
- T3: table columns are equal widths. A code block wraps and has no background, as in Electron. An image is the text `[image: alt]`.
- T3: Document scroll is `App::note_scroll(entity)`, in canvas units. `update` does not know the text's height, so it stops the offset at the top only. The renderer stops drawing at the end of the text.
- T1: `Session::editing` is a `TextEdit`, not an id. The working text lives only there until the edit ends; the document keeps the old text, and `App::editing_text(id)` is what to draw.
- T1: the rect is the exception. A text entity's fitted rect is written into the document while typing, as a drag writes rects, so the outline, handles and hit-testing follow. Ending the edit puts the old rect back and makes one step of text plus rect.
- T1: a text or sticky placed by its tool is in the document with no undo step until the edit ends. Ended with text in it, it is one step that undoes to nothing. Ended empty, it is taken out and no step or save happens.
- T1: "empty" is whitespace only. An emptied text entity is removed with its edges. An emptied shape label keeps its shape.
- T1: Escape, a press anywhere but the edited body, a tool change, a verb and a selection change all keep the edit. Only `DocumentOpened` throws it away.
- T1: `TextMeasure` is one method, `layout(text, spec) -> TextLayout`, which is plain data: lines with a byte range, a top, a height and a caret stop per grapheme boundary. Up and down, line ends, point to offset, the caret and selection rects and the fitted size are computed from it in `edit/layout.rs` and `edit/frame.rs`. `App` holds it as `Arc<dyn TextMeasure>`; the default estimates half an em a character with no wrapping.
- T1: no caret affinity. An offset where a wrapped line ends belongs to the next line, so End and a click past the last glyph stop one grapheme short on a wrapped line, usually before the space it broke at.
- T1: the editor's keys are the macOS set, with Command meaning Command or Control as in the binding table. CodeMirror's Emacs keys (Ctrl+A, E, K and friends) are not in.
- T1: the editor's undo joins consecutive typing into one step and consecutive deletes into another. A caret move, a paste, a cut, a line break and a composition each start a new one. `Action::Undo` and `Redo` go to it while an edit is open.
- T1: bullet lists (Enter continues, Enter on an empty item leaves, Backspace after a marker removes it, Tab and Shift+Tab nest) apply to text and stickies, not shape labels. Bold, italic and strike are T5's.
- T1: where an entity's text sits (`App::text_frame`) is in `specular-interact`, with the padding, sizes, line heights and the shape label box copied from `specular-scene`'s `view/text.rs` and `view/shape.rs`. The scene should read the frame from interact so the two cannot drift.
- T2: while the input method is composing, every key but Escape is ignored. A composition is one editor undo step from its first marked character, and a cancelled one leaves none. A click or ending the edit keeps the composed text as it stands.
- Shell batch: copied entities go on the clipboard as text, `specular:canvas:` followed by a compact `.canvas` of the selection's operands and the edges between them. It is not Electron's `web-canvas:entities:` payload, so the two apps do not paste each other's entities.
- Shell batch: `Effect::ReadClipboard` is answered with `Event::Clipboard(ClipboardContent)`, the text and a PNG of the image. With an edit open the text goes to `edit::paste`. Otherwise `Paste::of` decides in Electron's order: copied entities, an image, a one-line URL (a Desktop page), other text (a sticky). `Event::Paste` is gone.
- Shell batch: not ported from `clipboard-paste.ts`: copied file references, SVG, HTML and JSON text, and long text becoming a Document.
- Shell batch: `update` names a pasted or dropped file `assets/<entity id>.<ext>` and returns `Effect::WriteAsset` or `Effect::CopyAsset` ahead of the `LoadImage`. A dropped file already inside the space folder is shown by its relative path with no copy. Only images and `.md` are taken.
- Shell batch: one drop is one undo step for all its files. Electron makes a step per file.
- Shell batch: preferences are the native app's own file, `preferences.json` in `~/Library/Application Support/Specular Native` (the XDG config folder elsewhere, `SPECULAR_NATIVE_CONFIG_DIR` overrides). Not Electron's file: it rewrites its own from memory, so two writers would lose changes. A save keeps the file's other keys. A `--bench` run neither reads nor writes it.
- Shell batch: the Edit, Tools and View menus are data, `specular_interact::menus(&App)`. An item's shortcut is the first `BINDINGS` row with its action, and a tool's item runs what its key runs (Shape is R's rectangle variant). An item is enabled when that row's `Context` holds, no drag is in flight and it has something to act on.
- Shell batch: new rows in `BINDINGS`: Cmd+X, C, V, A, Cmd+= and Cmd+- (1.25x about the viewport's middle), Cmd+0 (100%) and Cmd+1 (zoom to fit, 64 px of room, never above 100%). Cmd+1 is `Context::Always`, as Electron's reset-viewport fires from inside a page. Backspace now comes before Delete so the menu shows it.
- Shell batch: select all takes every entity with no parent. Edges are not selected.
- Shell batch: on macOS the shell turns winit's default menu off (`with_default_menu(false)`) and installs muda's with `init_for_nsapp` from `resumed`. `app_protocol.rs` adds two methods and three protocols to winit's `NSApplication` class and never touches `mainMenu`, so the two do not meet. Quit and Close are the shell's own items, not AppKit's `terminate:`, so the pending save is written. A `--bench` run keeps winit's menu.
- Shell batch: File has Open, Save and Close. Open goes through an empty document so no image or note carries over from the old space folder. Save writes a pending autosave now. There is no Save As and no New.
- Shell batch: the window title is the file name without `.canvas`, plus ` — Edited` while a save is pending, with AppKit's edited dot. A canvas with no file is titled Specular.

- T1 scene: the compositor's `GlyphMeasure` and its `TextSystem` share one `FontSystem` behind a mutex. `TextRun::set(text, spec, ..)` in `specular-scene` is the only place a `TextSpec` becomes a run, and both the measure and `view` go through it and then through `text_shape::shape`.
- T1 scene: `TextMeasure::is_exact()` says the layouts are the renderer's. A loaded document's texts and stickies get their fitted rect, with no undo step and no save, only when it is true. The default estimate and the testkit's `FixedAdvance` return false, so a test's fixture rects stay as written.
- T1 scene: a text resize measures the height on every frame of the drag, and the edge the handle does not move stays put. A text has a width floor and no height floor, and a left or right handle keeps no ratio. This replaces S4's stand-in height.
- T1 scene: `App::handles()` is `None` while text is edited, so the handles are neither drawn nor hit. The outline stays. Electron's outline layer has no editing condition that I could find, so this follows the task and not the code.
- T1 scene: the caret and the composition underline are screen-space rects `max(1, round(zoom))` pixels thick, in the text's colour. The selection is canvas-space rects behind the glyphs. A selection hides the caret.
- T1 scene: the blink is 500 ms shown and 500 ms hidden, counted from `TextEdit::active_ms`. `update` stamps it after any event that changed the caret, the anchor, the text's length or the composition. No tick effect was needed: the shell polls and sends `Event::Tick` every loop turn.
- T1 scene: an empty plain text draws "Add text" at 40% alpha, edited or not.
- Visual check: the window's surface and the snapshot target are not sRGB formats (`Bgra8Unorm`, `Rgba8Unorm`), so colours blend encoded, as in a browser. On an sRGB target dark text came out thin and grey and a 30% highlight over text turned the text olive.
- Visual check: paths and polygons fill with the non-zero rule, as a 2D canvas does. A freehand outline crosses itself at its caps and corners, and even-odd left holes there.
- Visual check: `specular-app` depends on `specular-testkit` outside tests. A headless run is a `TestApp` plus the three effects that load things (pages, images, Documents); every other effect is dropped, so it never writes the canvas, the clipboard or preferences. `insta` comes along in the binary's dependency tree.
- Visual check: a headless run's clock starts at a fixed time and only `wait` moves it, so one script draws the same frame every run. `--snapshot-scale` is an addition to the task's flags.
- Visual check: the hover outline is not drawn while a gesture is in flight. `Session::hover` is only refreshed by a move with no button down, so it went stale during a drag.

- T4: a Document is edited as its source, one row a source line (`source_rows`), not in the read view's layout. Read, markers are gone and lists and tables are cells, so nothing maps back to a byte. Both views share `ColumnDraw`, the sizes and the colours. The text shifts a little when an edit opens.
- T4: the syntax styler is `edit/source.rs` in `specular-interact`, by hand and one line at a time (a code fence is the only state between lines). The editor has to measure styled text, and pulldown-cmark lives above it in `specular-scene`.
- T4: `TextMeasure::layout_styled(text, spec, spans)` measures one styled line. `edit/stack.rs` stacks the lines into one `TextLayout`, so motion, clicks, the caret and selection rects are the T1 code unchanged. `TextRun::source` is the one place a styled line becomes a run, for the measure and for `view`.
- T4: the renderer reports each owned column's height (`ColumnDraw::owner`, `Compositor::column_heights`) and the shell sends `Event::NoteHeights` when one changes. `update` stops a Document's scroll at its end with it. While edited, the end comes from the source's own layout.
- T4: saving is debounced in `update`, 350 ms after the last change on `Event::Tick`, as `Effect::WriteNote`. Ending the edit, opening another canvas and quitting write at once.
- T4: the shell refuses a `WriteNote` when the file holds a text it never read or wrote, and answers `NoteNotice::Refused`. That closes the half second between an outside edit and the watcher seeing it.
- T4: an outside change while editing, or a refused write, keeps both texts. Ours takes the file. Theirs is written to `<name> (conflict <id>).md` and gets a Document beside the first entity showing the file, as one undo step. A text equal to what is already known of the file is ignored.
- T4: undo, with ADR 0023. `Document` holds a transient `notes` map and `Command::SetNote`, never written to `.canvas`. A finished edit that changed the text is one history step (not one a commit, as Electron's is). The text the edit started from is seeded first with no step. An undo or redo that changes a held text writes the file. An outside change while not editing resyncs the held text with no step, so undo goes back from it and redo returns it.
- T4: `add-document` is two-phase because only the shell knows what names are taken: `Effect::CreateNote { rect }`, then `Event::NoteCreated { file, rect }` places the entity as one step and opens the edit. Names are Electron's `Untitled Note.md`, `Untitled Note 2.md`. An empty Document stays.
- T4: a double click on a Document puts the caret where it landed with nothing selected. A text or sticky still opens with everything selected.
- T5: formatting is `Action::Format`, bound in `BINDINGS` under a new `Context::Editing`. Cmd+B, I, E, Shift+X and Shift+8 are Electron's. It has none for these, so: Cmd+Shift+7 numbered, Cmd+Shift+9 task, Cmd+Option+1 to 6 heading, Cmd+Option+0 body. A text or sticky takes bold, italic, strike and bullets, as Electron's sticky does. A shape label takes none.
- T5: Enter keeps a list's own marker (`*` stays `*`), continues a number and adds an empty task box. Tab outside a list types two spaces in a Document and does nothing in a sticky.
- T1 leftovers: a resize floor is the kind's minimum or the size the entity started at, whichever is less. Page Up and Down move the caret by the Document's window, or the viewport for a text. A selection drag held past a Document's window scrolls it on each tick, and one held at the viewport's edge pans the canvas when the text runs off that side.

- QA: a save writes `App::document_to_save()`, not `App::document()`. A text measured when the file was opened goes back to the size it was read with unless the session changed its rect. Without this, the first change to an Electron file rewrote every text's width and height.
- QA: the formatting shortcut with the caret just before a run's closing marker steps past the marker. Cmd+B, a word, Cmd+B leaves one pair. Electron's `toggleWrap` nests a second pair there, so this differs on purpose.
- QA: changing the brush moves the stroke width to the nearest width that brush is offered in (pen 2 and 4, highlight 8 and 16). Electron does this in the popup only, so its Shift+M draws a 2 wide highlight.
- QA: after any event that moved the camera or the document, or ended a drag, `update` runs the pointer again where it stands (`pointer::settle`). A drag in flight follows a wheel or a pinch, and the hover is found again after an undo. `Session` keeps the last modifiers for it.
- QA: a headless run keeps the clipboard and the Documents it makes in memory (`headless/stand_ins.rs`), and reports Document heights after each snapshot as the shell does after each frame. New script steps are `triple-click`, `compose`, `commit`, `clipboard`, `wheel`, `pinch` and `save`.
- QA: an Option-drag draws each copy as a tinted outline where it will land. The originals stay put until the release.

- Groups/edges/S7: stack-order verbs have no Notes and Pages sections, because the native stack is one (S1). A selected group is moved as its whole run; Electron's math given only the group id leaves the children behind on a backward move.
- Groups/edges/S7: a new group goes just in front of its frontmost member's run and the run is gathered there in the same step. Electron appends it at the top and leaves it scattered until the next reorder.
- Groups/edges/S7: a freeform group's rect follows its members (union plus 24), in the same undo step as the change and frame by frame during a drag. Electron only refits auto-layout groups; the task asked for this. `group_fit::then_fit` runs from `gesture::apply_fitted`, which every step goes through. Not refitted: an empty group, a group moved or resized as a whole, a group no step touched. So a hand-sized group tightens the first time a member changes.
- Groups/edges/S7: the group drop target is tested against the group rects captured when the drag began, as CONTEXT.md says, since the live rects now follow the drag.
- Groups/edges/S7: Electron has no entered-group state; its double click selects the group's direct members. That is kept, with `Session::entered_group` on top: it holds while the selection stays inside the group, Escape steps out one level (selecting the group left), and only then deselects.
- Groups/edges/S7: delete takes a group with its descendants and their edges (ADR 0034's operands). Electron's `deleteGroups` removes member pages only. Ungroup acts on one selected group, and group needs two items, as in Electron.
- Groups/edges/S7: `hit::body_at` picks the innermost group under the point, then the frontmost. The outer group used to win, so a nested group's interior could not be reached. Group tints are drawn before every entity, as in Electron, with the border and title left in the group's stack slot.
- Groups/edges/S7: a group title and an edge label are edited with the text editor as `Target::Title` and `Target::EdgeLabel`, one line each, ended by Enter. The edge label's `TextEdit::entity` holds the edge's id; the two id types share a namespace (F2). Escape cancels a title rename, as Electron's inline label does, and commits an edge label, as its popup field does.
- Groups/edges/S7: Electron edits an edge label only in its popup. Here a double click on the edge opens it in place.
- Groups/edges/S7: an edge drag changes nothing in the document until the release. A drop with no anchor in reach but over another item's body connects to the side facing the fixed end; Electron snaps to anchors only. Drawings are not targets. A re-route that ends where it began records no step.
- Groups/edges/S7: an anchor dot shows only for the side the pointer is over, on the hovered or selected item, and all four dots show on every item during an edge drag, as in Electron. `anchors.rs` is the one set for hit-testing and drawing.
- Groups/edges/S7: re-anchoring on a move's release covers the selected entities only. One whose page moved with it keeps its anchor, and an unchanged page and URL writes nothing, so scroll and element fields survive. Nudge re-anchors too, as Electron's does. An Option-drag copy is placed like a paste.
- Polish: `History<S>` carries the caller's state from either side of each step, and the app's `S` is the selection. `apply_step` records the selection before; `update` settles the selection after, once the event that made the step is done. Undo restores the first and redo the second. Electron also keeps a selection on each stack item, but it is the one from after the action, restored without checking the ids exist, so undoing a duplicate there selects copies that are gone. A step made by ending a text edit carries the selection the edit set, so undoing a placed sticky leaves nothing selected.
- Polish: an unselected drawing is hit within 6 px of its ink (or half its drawn width, if more), not anywhere in its box as in Electron. Selected, the whole box is the drawing, so it drags from anywhere inside its outline.
- Polish: an edge and an entity it crosses are still hit in stack order, as Electron's DOM stacking does. Where the line is over an entity the press is shared: a drag moves the entity and a click selects the edge. Electron selects the edge on click and a drag from there does nothing.
- Polish: Cmd+D pans the camera by the least that shows the copies with 48 px around them. Electron leaves the camera, so on a crowded canvas a duplicate there looks like nothing happened.
- Polish: Escape while composing is left as T2 had it (it ends the edit and keeps the marked text). Electron's editors do the same, with no composition check. A real input method takes Escape before the app sees it, which only a person can check.
- Polish: `Draw::Shadow` is a rounded rect's blurred shadow. The compositor draws it as one more instance of the SDF shape shader, the edge under a Gaussian by the error function, so it joins its card's batch and nothing is blurred. Stickies, file cards and Documents get Electron's `0 2px 8px rgba(0,0,0,0.08)` in canvas units. The whole kitchen sink went from 22 batches to 24.
- Polish: `Item::blend` is `Normal` or `Multiply`, and only paths and polygons honour `Multiply` (a second mesh pipeline). The highlighter is multiplied in at 70%, so text under it keeps its colour. Electron paints it over with an alpha gradient and grain and no blend mode, which greys the text.
- Polish: group and page titles keep 11 px down to zoom 0.5 and shrink with the canvas below it (`title_scale`), and end in an ellipsis at the entity's width (`TextOverflow::Ellipsis`, cosmic-text's `Ellipsize`). The group title's hit box follows both. Electron's group title is a fixed 11 px with no truncation, and it has no page title on the canvas at all.
- Polish: selected text is `#b3d7ff`, the macOS highlight. Electron sets no selection colour, so Chromium paints the system's.
- Polish: emoji are cut out of a run and set to what CoreText measures: 1.25 em wide up to 16 px, down to 1 em at 24 px, with a pixel of tracking that is gone by 28 px, asked for from Apple Color Emoji by name so `U+FE0F` gets the colour glyph. The shaper alone draws them 0.8 em wide. The family emoji was right all along: one boxed silhouette is Apple's current glyph for it.
- Polish: a `file` that starts with `__SPECULAR_SPACE__/` is read from the canvas's own folder, so the starter space opens where it lies. Electron rewrites the token when it copies the space.
- Polish: an Option-drag's ghost is the entity drawn again at half opacity where the copy will land (`Item::translated`), inside the outline it will have. Electron draws empty boxes.

## Needs a human at a Mac

What nobody has done by hand. The scenario scripts (`native/fixtures/scenarios`) and the tests already prove what is drawn, every gesture's result in the document, undo, text editing, the saved file and the headless clipboard, so none of that is here. Everything below needs the real window. Start with `cargo run -p specular-app -- <a copy of fixtures/kitchen-sink.canvas>`.

1. Look. One menu bar (Specular, File, Edit, Tools, View, Window), with `--source cef` too. Text is sharp on a 1x and a 2x display. Cards have a soft shadow. Pinch with text on screen: glyphs should not shimmer or go blocky, and sharpen a frame after you stop (else narrow `MIN_STRETCH` and `MAX_STRETCH` in `scene_pass/raster_hold.rs`).
2. Drag. A move snaps to the grid and feels attached. Shift mid-drag, Option-drag (the ghost is the item, faded), every handle on each kind, the corner cursors, a marquee. Drag from where an edge crosses a sticky: the sticky moves. Click there: the edge is selected.
3. Groups and edges. Drag an item over a group: the ring shows and the release puts it in. Double-click a group to work inside it, double-click its title to rename it, also zoomed out past half. Drag from an anchor dot to another item. Double-click an edge to label it. Cmd+] and Cmd+[ restack.
4. Keys and menus. Cmd+D, Cmd+Z, then an arrow: the originals move. Cmd+Z, D, A, =, 1 each act once, not twice. The active tool is checked in Tools. Undo, Copy and Delete are grey with nothing to act on.
5. Pages, with `--source cef`. One click selects, a second enters, typing reaches the page, Escape leaves. Entered, Cmd+C, V, A and Z go to the page. A drag from empty canvas does not scroll a page. V, R and Backspace still work on the canvas and still type into a page.
6. Text. Double-click a sticky: the caret sits between glyphs at zoom 0.25, 1 and 3, blinks once a second, and typing does not lag in a few hundred words. With a Japanese or Pinyin input method: the marked text is underlined, the candidate window is by the caret, letters arrive once, and Escape cancels the composition before it ends the edit.
7. Documents. Tools > Document, click, type: `Untitled Note.md` appears and fills in a third of a second after you stop. Edit the file elsewhere while the edit is open: a conflict copy appears. Quit mid-edit: the last keys are in the file. Try Cmd+Option+1, which macOS may take.
8. Clipboard. Copy two shapes and their edge, paste, paste in a second window. A URL from a browser pastes as a page, a sentence as a sticky, a screenshot (Cmd+Ctrl+Shift+4) as a file in `assets/`.
9. Drop. A png and a `.md` from Finder, from inside and outside the space folder. They land at the pointer's last position before the drag, which may be wrong.
10. Files. A change is on disk a third of a second later with the camera in `appState`. An edit to the file from outside is followed and the camera kept. Quit within that third of a second: saved. File > Open replaces the canvas. The title shows ` — Edited`. R, Shift+R, quit, start: the shape tool is still a diamond. png, jpeg, webp and gif files appear; a missing file and an svg stay cards.
11. Speed. One `--bench --chrome on` run against an older build.
12. From M1: the egui checks at the end of ADR 0039.

Known gaps against Electron, not checks: the hand and mono fonts fall back to system fonts (Kalam and Geist Mono are not loaded), an edge label has the line running through it, a comment badge has no icon, a label that overflows a small shape is clipped to its middle line, and the highlighter has no gradient or grain.

- Groups, edges, S7: nothing was run in a window. Check Cmd+] and Cmd+[ with and without Shift, and the Arrange menu. Cmd+G on two items, Cmd+Shift+G, a child dragged out of and into a group (ring on the target, the group hugging what is left), double click into a group, Escape back out, double click a title and type. Hover an item and drag from the dot to another item's dot and to its body; grab an edge's end, drop it on nothing and check the edge goes and Cmd+Z brings it back; double click an edge and type a label. Drag a sticky onto a page and off it, and with Command held.

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

### Creation tools: S8, the interact half of K1, K2 and K3, and K7 on placement. See `git log -- native/crates/specular-interact/src/place.rs`

- S8: `keys.rs` is gone. `bindings.rs` holds `BINDINGS`, one const table of `Binding { chord, context, action, repeats }`, and `binding_for(app, input)`. The first row whose chord matches and whose `Context` holds wins. A key with no row goes to the entered page.
- Not in the table, because no `Action` exists for them: H and I (no hand or inspect tool), Cmd+1, Cmd+W, Cmd+G, Cmd+Shift+G, Cmd+Shift+A, Cmd+A, Cmd+T, the four stack-order chords and Enter. `src/shared/app-menu-shortcuts.ts` is the menu's accelerators (quit, hide, settings, devtools), which belong to the shell's menu.
- `tool_defaults.rs` ports `tool-defaults.ts`. `Action::SetToolDefault(patch)` is for the tool popup.
- `place.rs` is `Gesture::Place`, the one-shot tools. A click places the default size with its top-left on the grid. A shape drag of 24 units or more each way sizes the shape, and Shift squares it. The entity is selected and the tool returns to select. `add-document` still only holds the press.
- `draw.rs` is `Gesture::Draw`. Points are in canvas space, Shift holds them to 45 degrees from the first, and the rect is the points grown by half the width. The selection is cleared and the tool stays.
- No pressure: `PointerInput` has none and neither has the stroke on disk. Add both together.
- `anchor.rs`: `page_anchor_for(document, entity)` is `pageAnchorFor`. `live::create` calls it for every placement. Text, drawings and shapes anchor; pages, files, groups and grouped entities do not. A move still does not re-anchor on release.
- For the scene: nothing to draw for a preview. The shape or stroke in flight is an entity in `App::document()`. `App::creating()` gives its id if it should look different, for example no hover outline.
- For the shell: one arm was added to `specular-app/src/app/effects.rs` so `Effect::SaveToolDefaults` compiles, and it only logs. Write `defaults.to_json()` under `toolDefaults` in the preferences file, and send `Event::ToolDefaultsLoaded` at startup.
- For T1: start from `Session::editing`. An empty text left when editing ends is not deleted yet.
- Tests: `tests/bindings.rs`, `tools.rs`, `draw.rs`, `anchoring.rs`. `routing.rs` lost the C toggle test.
- Gate: fmt, clippy and tests pass for `specular-interact`, `specular-testkit` and `specular-scene` on HEAD plus this change, in an exported copy (362 tests). The working tree had two other agents' work in it, including notes hunks in `app.rs`, `effect.rs`, `event.rs`, `lib.rs` and `update.rs`, which are not in this commit.

### T3 — see `git log -- native/crates/specular-scene/src/markdown`

- Scene: `TextSpan` and `SpanStyle` on `TextRun`. `ColumnDraw`, `Row`, `RowRule` in `column.rs`. `markdown/` turns text into a flat `Block` list with quote and list depth on each block (18 tests). `view/document.rs` turns blocks into rows with Electron's sizes and colours. Six snapshots in `tests/documents.rs`.
- Compositor: `text_shape.rs` shapes spans with `set_rich_text` and reads underline and strike rects from cosmic-text's decoration spans. `column.rs` is the pure row stacking. `text_areas.rs` builds the glyphon areas. `tests/scene_rich_text_gpu.rs` has nine readbacks: span colour, weight, monospace, both lines, row stacking, scroll, clip, paint order.
- Interact: `notes.rs`, `Effect::LoadNote` and `DropNote`, `Event::Note`, `App::note` and `App::note_scroll`, and one branch in `update` so the wheel over a selected Document scrolls it. `tests/notes.rs`.
- Shell: `notes/watch.rs` (which files, what was last read) and `notes/mod.rs` (the thread). `app/note_run.rs` runs the effects. Paths resolve as image paths do.
- Testkit: the scene snapshot prints spans on a text line and a column as indented row, rule and cell lines.
- Dead travel: wheel past the end of a Document and the offset keeps growing, so scrolling back does nothing until it comes back under the end. The fix is for the renderer to report each column's height and `update` to clamp with it.
- A span's colour is set on the shaped glyphs, so item opacity does not fade it. Nothing fades a Document yet.
- The file card's drop shadow is not drawn. The scene has no shadow.
- Links do not open and nothing in a Document can be selected. T4 owns editing. `add-document` still creates nothing.
- For T4: parsing per frame is fine for notes. Cache the blocks beside the text if a large file shows up in a trace.
- `specular-app/src/app/mod.rs` is now 486 lines.
- Gate: fmt, clippy and `cargo test --workspace` pass, GPU tests included on this machine.

### T1 and T2, the interact half. See `git log -- native/crates/specular-interact/src/edit.rs`

- `specular-interact/src/edit.rs` and `edit/`: `buffer` (`TextEdit`), `history` (the editor's undo), `segment` (graphemes, words, paragraphs, on `unicode-segmentation`), `motion`, `keys`, `lists`, `ime`, `pointer` (`Gesture::TextSelect`), `measure` (the trait), `layout` (queries on a `TextLayout`), `frame` (`TextFrame`, the fitted rect). `edit.rs` is begin, end, paste and the `App` accessors.
- Starts on a double click with the select tool on a text, sticky or shape, and after add-text and add-sticky, with all the text selected. Ends as the Decisions say.
- For the scene: `App::text_edit()` (entity, text, caret, anchor, `selection()`, `composition()` as byte ranges), `App::editing_text(id)`, `App::text_frame(id)`, `App::caret_rect()` (zero width, there even with a selection), `App::selection_rects()`, `App::composition_rects()` (underline these). Rects are canvas space, one per line. `Session::now_ms` can drive a blink; nothing records when the caret last moved. An empty plain text should show the "Add text" prompt; its rect is already sized for it.
- For the real `TextMeasure`: implement it in `specular-compositor` on the `FontSystem` glyphon draws with, and install it with `App::set_text_measure` at startup. Build the `Buffer` exactly as `scene_pass/text_shape.rs` does for a `TextRun` with the same family, size (`Metrics::new(spec.size, spec.line_height)`), wrap width (`Wrap::WordOrGlyph` with one, `Wrap::None` without), at scale 1 in canvas units. Per `layout_runs()` run emit one `LayoutLine`: `range` from the run's glyph byte span mapped into the whole text (cosmic-text indexes per `BufferLine`, so add the paragraph's start), `top` as `line_top`, `height` as `line_height`, and a `CaretStop` at each grapheme boundary with `x` from the glyph's `x` (or `x + w` for the end stop; split a ligature's width evenly across its graphemes). Apply `spec.align` yourself so `x` matches the contract in `edit/measure.rs`. Empty paragraphs must still give a line. It is called on every key and from the accessors each frame, so cache by `(text, spec)`. It must be `Send + Sync`: a `Mutex` around its own `FontSystem`, or around one shared with the compositor.
- Testkit: `FixedAdvance` (10 a character, 20 a line, greedy wrap) is installed in every `TestApp`. New: `sticky`, `plain_text`, `labelled`, `triple_click`, `compose`, `commit`, `paste`, `editing_text()`, `caret()`. `Key::Home` and `Key::End` exist.
- Clipboard: copy and cut return `Effect::WriteClipboard`, Cmd+V returns `Effect::ReadClipboard`, and `Event::Paste(String)` inserts. The agent on S6 was adding `Event::Clipboard` and `Action::Paste` in the same files as this was committed; its text path should call `edit::paste` and can then replace `Event::Paste` (and `TestApp::paste`).
- Shell: two small hunks. `translate.rs` maps Home and End. `effects.rs` lists `ReadClipboard` in the arm that only logs. `SetImeAllowed(true)` and `SetImeCursorArea` are returned when an edit starts and on each composition.
- Tests: `tests/text_edit.rs`, `text_session.rs`, `text_pointer.rs`, `text_ime.rs` (42). `tools.rs` and `anchoring.rs` changed where a placed text used to be an undo step at once.
- Not done: the scene and compositor half (caret, selection, composition underline, the real measure). Auto-height only runs during an edit, so a text resized by a handle still keeps S4's stand-in height. No autoscroll, no drag-and-drop of selected text, no Option+Up or Down by paragraph, no Page Up or Down.
- Gate: fmt, clippy and tests pass for `specular-interact`, `specular-testkit` and `specular-scene`, and clippy for `specular-app` and `specular-compositor`, on HEAD plus this change in an exported copy. The working tree had the S6 agent's unfinished clipboard, zoom and menu work in `event.rs`, `effect.rs`, `update.rs`, `bindings.rs`, `lib.rs`, `place.rs`, `Cargo.toml` and `Cargo.lock`, which is not in this commit.

### Shell batch: P6's menu bar, S6, file drop, preferences, title. See `git log -- native/crates/specular-app/src/app/menu_bar`

- Interact, each in its own module: `clipboard.rs` (`ClipboardContent`, `Paste::of`, copy, cut, paste), `drop.rs` (`DroppedFile`), `asset.rs` (`AssetBytes`, asset names, `insert_selected`), `url.rs` (`looks_like_url`, `normalize_user_url` from `url.ts`), `zoom.rs`, `select_all.rs`, `menu.rs` (`menus`, `binding_of`). `clone.rs` now copies from any `Document`, which paste uses.
- New: `Action::{Copy, Cut, Paste, SelectAll, ZoomIn, ZoomOut, ZoomReset, ZoomToFit}`, `Event::Clipboard` and `Event::FilesDropped`, `Effect::WriteAsset` and `Effect::CopyAsset`. `Event::Paste` is removed and `TestApp::paste` sends `Event::Clipboard`.
- Shell runners under `specular-app/src/app/`: `clipboard_run.rs` (arboard), `asset_run.rs`, `drop_run.rs`, `settings.rs` with `src/prefs.rs`, `title.rs`, and on macOS `menu_bar/` (muda) and `file_menu.rs` (rfd). Every `Effect` now has a runner.
- `app/mod.rs` is 369 lines. The bench methods are in `bench.rs`, the LOD ones in `lod.rs`, page events in `page_events.rs`.
- The menu is brought in step with `menus(&App)` once per loop turn. An enabled item with a Command shortcut takes the key before winit sees it, and a disabled one lets it through to the binding table and on to an entered page. That is why the model disables by `Context`.
- For whoever adds an `Action`: `menu.rs`'s `has_target` matches on it with no wildcard, and `menu_bar/keys.rs` matches on `Key`.
- `tests/bindings.rs`: the "only Escape fires inside a page" test now also lists zoom to fit.
- Not done: the context menu from P6's line, Save As and New, a recent-files list, stack-order and group actions (no `Action` yet), copying an image or text out to other apps as anything but our own text, pasting files copied in Finder, menus on Windows and Linux (the `menu_bar` and `file_menu` modules are macOS only).
- A text edit open: Cut, Copy and Delete are greyed in the menu, since the editor takes those keys itself. Choosing them with the mouse does nothing. Paste and Undo work from the menu.
- URL normalising is by hand, not WHATWG: scheme and host lower-cased and an empty path made `/`. No punycode, no percent-encoding.
- Tests: `specular-interact/tests/clipboard.rs`, `drop.rs`, `menus.rs`, `view_actions.rs` (35), unit tests in `url.rs`, `zoom.rs`, `asset.rs`, and in the shell for the preferences file, the PNG encode, asset paths, drop paths, the title and the shortcut mapping.
- `tests/text_ime.rs`: one test no longer pastes with no edit open, since that now makes a sticky.
- Gate: fmt for the workspace, and clippy and tests for `specular-interact`, `specular-testkit`, `specular-scene` and `specular-app` (635 tests), pass on HEAD plus this change in an exported copy. The working tree had another agent's text-measure work in `specular-compositor`, `specular-scene`, `edit/`, and single hunks in `update.rs` and `app/mod.rs`, none of it in this commit. `Cargo.lock` is the one cargo wrote for the exported copy.

### T1 and T2, the scene and compositor half. See `git log -- native/crates/specular-compositor/src/scene_pass/text_measure.rs`

- Compositor: `fonts.rs` (`Fonts`, the shared font system), `scene_pass/text_measure.rs` (`GlyphMeasure`), `Compositor::text_measure()`. It caches the last 32 layouts. `GlyphMeasure::new()` loads fonts for itself and needs no GPU.
- Stops are read off glyph clusters. A ligature's width is split evenly across its graphemes, right-to-left glyphs lead from their right edge, and a space dropped at a wrap keeps the x of the glyph before it.
- Scene: `view/editing.rs` draws the selection, the caret and the underline. `view/text.rs` and `view/shape.rs` take the `TextFrame` from interact and draw `App::editing_text` when there is one. Eight tests in `tests/editing.rs`, seven of them snapshots.
- Interact: `edit/blink.rs`, `App::caret_visible()`, `edit::fit_all` on `DocumentOpened`, measured height in `resize_drag.rs`. Three small hunks in `update.rs`. `tests/text_fit.rs`, and `tests/resizes.rs` updated for measured heights.
- Shell: one hunk in `app/mod.rs` installs the measure before the document opens. Testkit: `TestApp::measure_with`.
- Tests: `tests/text_measure.rs` (14, no GPU: lines, wrapping, emoji, CJK, alignment, clicks through a `TestApp` on the real measure) and `tests/scene_caret_gpu.rs` (3 readbacks at zoom 0.5, 1 and 2). Moving the caret 5 px fails all three.
- Not done: the Document scroll dead travel. The measure sets plain text in one style, and a Document is rows of rich cells that `specular-scene` builds and the compositor stacks, so `update` still cannot know the column's height. It needs the T3 entry's fix: the renderer reports each column's height.
- Found, not fixed: `caps::min_size` gives text a 100 width floor but an auto-width text can be 64 wide, so a press and release on its handle with no movement widens it to 100.
- Still missing from T1: autoscroll, drag-and-drop of selected text, Option+Up and Down, Page Up and Down. The drag-copy preview is still not drawn.
- Gate: fmt, clippy and `cargo test --workspace` pass, GPU tests included on this machine.

### Visual check: headless snapshots, the kitchen sink, and what looking at them found. See `git log -- native/crates/specular-app/src/headless`

- `specular-app --snapshot OUT.png [--snapshot-size WxH] [--snapshot-scale N] [--snapshot-camera x,y,zoom|fit] [--script FILE] FILE.canvas` draws on the real adapter with no window. `src/headless/`: `mod.rs` (the run), `script.rs` (the line format), `target.rs` (texture to PNG). `native/CLAUDE.md` has a "Looking at what it draws" section; use it after any scene or compositor change.
- `native/fixtures/kitchen-sink.canvas` (163 entities, 18 edges, 8 annotations) with `kitchen-sink.md` and `assets/kitchen-sink.png`. It is in the writer's canonical form and in both tests of `specular-doc/tests/canvas_repo.rs`, byte test included. It was generated by a script that is not checked in; edit the JSON by hand or regenerate and re-save through `Document`.
- Fixed: holes in pen strokes at round caps, sharp corners and dots (fill rule, `scene_pass/mesh.rs`). Thin grey text and over-bright translucent fills (sRGB target, `gpu_window.rs`). The canvas was `#f4f4f4` with grey dots; it is now Electron's `#edebea` with `#a8a29e` dots (`DotGrid::default`). A stale hover outline during a drag (`view/session.rs`). The startup log said `pages=163` for 163 entities.
- Looked right against Electron's numbers and left alone: sticky, shape, group, edge, file card, Document, selection, marquee and badge colours and sizes; arrowheads; dashes; z-order; text baseline and line height; mips at zoom 0.25; glyph sharpness at zoom 3 and at scale factor 3.
- Not bugs, but they look like it: a sticky's height floor is `200 * size / 14`, as in Electron, so a size-32 sticky is 457 tall. Sparse stroke points shrink and round off, because streamline is 0.75. Resolved and dismissed comments are not drawn. `Welcome.canvas` opened from `resources/` shows "File not found" for its Document: the path is `__SPECULAR_SPACE__/Welcome.md`, which Electron's `starter-space.ts` rewrites when it copies the space.
- A page annotation's `offsetX`/`offsetY` are fractions of the page, not pixels.
- Another agent's unfinished work in `specular-doc` and `specular-interact` did not compile for most of this task. Everything here was built, snapshotted, launched and gated in a detached worktree of `763283f2` plus this change. `Cargo.lock` in the commit is that worktree's: HEAD's plus the testkit line.
- Gate: fmt, clippy and `cargo test --workspace` (1057 tests, GPU ones included) pass there.

### T4, T5 and the T1 leftovers. See `git log -- native/crates/specular-interact/src/edit/note.rs`

- Doc: `Command::SetNote`, `Document::note` and `notes`, transient.
- Interact, new under `edit/`: `source` (styler), `stack` (`source_rows`, the stacked layout and its cache on `App`), `note` (save, undo step, conflict, creation), `format` and `formatting` (the transforms and `Format`). `lists` knows numbered and task lines. `Target::Note`, `Key::PageUp` and `PageDown`, `Motion::{ParagraphUp, ParagraphDown, PageUp, PageDown}`.
- New: `Effect::{WriteNote, CreateNote}`, `Event::{NoteCreated, NoteHeights}`, `NoteNotice::Refused`, `Action::Format`, `Context::Editing`, `Placing::Document`.
- Scene: `TextRun::source`, `ColumnDraw::owner`, the edit view in `view/document.rs`. Compositor: `GlyphMeasure::layout_styled`, `column_heights`.
- Shell: the note thread also writes and creates (`notes/watch.rs`), `NoteLoader::finish` drains it on exit, `report_note_heights` runs after each frame. Two arms were added to the other agent's `headless/mod.rs` so it compiles; it ignores both effects.
- Testkit: `note(id, rect, path)` and `TestApp::note_text(file, text)`.
- Tests: `tests/note_edit.rs` (19), unit tests in `source`, `stack`, `note`, `format_tests`, `lists`, two scene snapshots in `tests/documents.rs`, three shell tests for refused writes, new names and the exit drain.
- For whoever is next: `assert_undo_returns_to_start` compares whole documents, held note texts included, so it fails after a Document edit. Compare entities instead, or teach it to ignore `notes`.
- Rows of the edit view are all emitted and the renderer culls them. `view` restyles the source every frame. Fine for notes, untested on a file of thousands of lines.
- Not done: the formats are not in the menu or a popup, links do not open, no drag-and-drop of selected text, no smart paste, `Cmd+K` link, no rename of the file, and a Document off screen reports no height. The undone file of a removed `add-document` stays on disk.
- Gate: fmt, clippy and `cargo test --workspace` pass (1112 tests), GPU tests included on this machine.

### QA: an end-to-end pass over everything so far. See `git log -- native/fixtures/scenarios`

- `native/fixtures/scenarios/`: ten session scripts, `run.sh`, `check.py` and a README. `run.sh` leaves 73 PNGs in `native/runs/qa/` and `check.py` compares the saved canvases as JSON. Regression tests are `specular-interact/tests/qa_session.rs`, `specular-scene/tests/qa_session.rs` and additions to `note_edit.rs`, `text_fit.rs`, `format_tests.rs`, `tool_defaults.rs` and `persist/app_state.rs`.
- Fixed, each with the test that failed first: `assert_undo_returns_to_start` after a Document edit (texts the start never held are left out on the way back). Cmd+B, a word, Cmd+B left `****` behind, and the stray stars then stopped Enter leaving a list. A save rewrote the rect of every text in a file from Electron. The zoom was saved as `0.800000011920929`. A dragged item and a marquee corner stayed behind when the canvas scrolled mid-drag, until the next pointer move. The hover outline stayed on an entity that undo had moved away, and on the wrong entity after a drag ended. Shift+M drew a 2 wide highlight at 30% alpha, which is close to invisible. An Option-drag drew nothing.
- Found, not fixed. Undo of a duplicate, a paste or an Option-drag leaves nothing selected: marquee three items, Cmd+D, Cmd+Z, press an arrow key. Cmd+D on a crowded canvas puts the copies off screen and the camera stays: marquee on the kitchen sink, Cmd+D (Electron's placement does the same). A press on an edge that crosses a sticky takes the edge, and the drag does nothing: kitchen sink, drag at the `sticky to shape` line over `sans font`. A drawing is hovered and hit anywhere in its bounding box: draw a diagonal stroke, move into the empty corner. Escape while composing ends the edit and keeps the marked text, where an input method would cancel the composition first (T2's decision, check on a real IME).
- Rough edges a designer would see in the PNGs. Stickies and Documents have no shadow, so a Document is a pale card with no edge. Group titles and page titles keep their pixel size, so at zoom-to-fit they are larger than what they name and overlap. The highlighter paints over text and greys it. The text selection is grey. The family emoji draws as one small boxed glyph. `Welcome.canvas` shows "File not found" for its Document. The copy ghost is an empty tinted rect, not the item.
- For the next agent: use `document_to_save()` for anything written to disk. `pointer::settle` is the place to add anything else that must follow a still pointer. The window run and the scenario runs started `specular-app` on purpose, against this file's usual rule, because the task asked for them.
- Gate: fmt, clippy and `cargo test --workspace` pass (1127 tests, GPU ones included), in a detached worktree of `e85b962c`.

### Interaction half of K4, K5 and S7, with re-anchor on move. See `git log -- native/crates/specular-interact/src/edge_drag.rs`

- S7: `stack_order.rs` (the math from `entity-order-math.ts`, group runs kept contiguous), `Action::{BringForward, SendBackward, BringToFront, SendToBack}` on Cmd+] and Cmd+[ with and without Shift, each one `SetOrder`. A new Arrange menu holds them with Group and Ungroup.
- Groups: `groups.rs` (`Action::Group` Cmd+G, `Action::Ungroup` Cmd+Shift+G, `enter`, `step_out`), `group_drop.rs` (the drop target, reparenting on a move's release, the contiguity fix-up), `group_fit.rs` (bounds follow members). `App::group_drop_target()` and `App::entered_group()` are for the scene.
- Edges: `edge_drag.rs` with `edge_drag/controller.rs` (the port of `edge-drag-controller.ts`), `Gesture::EdgeDrag`, `anchors.rs`, `App::anchors()`, `App::edge_preview()`, `App::rerouting()`, `Cursor::Crosshair`. Cancel, Escape and a drop on nothing during a re-route remove the edge as one step, by design.
- Label editing: `edit/title.rs` and `edit/edge_label.rs`. `App::edit_frame()` is where the edited line sits; `text_frame` stays body text only.
- Re-anchor: `anchor::then_reanchor` adds `SetAnchor` to the move's step. Command or Control at the release suppresses it and the group drop.
- Scene: `view/edge_chrome.rs` (anchor dots, the dashed preview, origin dot, snap ring), the drop ring and the entered group's dashed ring in `view/session.rs`, the edited title and edge label through `editing::edited_line`. The Option-drag copy ghost is the QA pass's; a second one drawn here was dropped in the merge.
- `live::commit` and `commit_with` are gone: `live::commit_following` is the drag commit, and it carries the followed groups. `gesture::cancel` takes `effects`. Testkit: `with_edge`.
- Tests: `tests/stack_order.rs`, `groups_verbs.rs`, `groups_drag.rs`, `groups_enter.rs`, `groups_fit.rs`, `edges.rs`, `edge_labels.rs`, more in `anchoring.rs` and `menus.rs`, and scene snapshots in `specular-scene/tests/groups.rs` and `edges.rs`. Each interaction was also scripted with `--snapshot --script` and the PNGs looked at.
- Not done: auto-layout, gap handles and reorder dots (deferred). A gap in the edge line under its label. Live refit of a group while a text inside it grows (it refits when the edit ends). Edge popup actions. Duplicate and paste do not re-anchor.
- Built in a detached worktree in three steps by subagents, then squashed and rebased onto the markdown-editing and QA commits. The merge needed `Target::Note` arms beside the two new targets.
- Gate: fmt, clippy and `cargo test --workspace` pass on the rebased commit (1284 tests, GPU ones included).

### Polish: what the QA pass left open. See `git log -- native/crates/specular-scene/src/translate.rs`

- Every item on the QA entry's two lists is closed or decided; the "Polish" lines under Decisions say how each differs from Electron and why.
- Doc: `History<S>` with `apply_from`, `settle`, `is_open`; `undo` and `redo` return the state to restore. Interact: `History<Selection>`, `title_scale`, `CopyPreview`, `Click::SelectEdge`, `zoom::reveal`, ink hit-testing in `hit.rs`.
- Scene: `ShadowDraw`, `Blend`, `TextOverflow`, `Item::translated`, `palette::card_shadow`. Compositor: the shadow kind in `fs_shape`, the `mesh_multiply` pipeline, `scene_pass/emoji.rs`, ellipsis in `text_shape.rs`. Shell: the starter-space token in `images/resolve.rs`.
- Tests, one or more a fix: `specular-interact/tests/history_selection.rs` and `polish.rs`, `specular-scene/tests/polish.rs`, `specular-compositor/tests/scene_effects_gpu.rs` (pixel readbacks of the shadow and the multiply), additions to `text_measure.rs`, unit tests in `emoji.rs`, `text_shape.rs`, `translate.rs`, `zoom.rs`, `resolve.rs`. 24 scene snapshots changed and were read: shadows, the title's wrap, the selection colour, one multiplied stroke.
- Scenario `b` no longer re-marquees after an undo, which is the selection fix in use, and puts the camera back after Cmd+D. All ten pass `check.py`. Before and after PNGs were read; the final set is in `native/runs/qa/`.
- For the next agent: select what a step made after `document_step`, never before, or undo restores the wrong selection. A script that goes on by screen position after Cmd+D needs a `camera` step.
- For the next agent: `Blend::Multiply` on anything but a path or polygon is ignored. A shadow's bounds reach 1.5 blurs out, so cards closer than that to a neighbour's text split a batch.
- Not done: the selection before a text tool's click is not what undoing the placed text restores. The emoji curve is Apple Color Emoji's; other platforms get the same numbers. The "Known gaps" line under "Needs a human" is what is still off against Electron.
- "Needs a human at a Mac" is now one ordered checklist. The per-task entries it replaced are in git history.
- Rebased onto the groups and edges commit: its title editor takes `title_scale`, and its verbs already select after their step. Gate: fmt, clippy and `cargo test --workspace` pass there (1325 tests, GPU ones included).
