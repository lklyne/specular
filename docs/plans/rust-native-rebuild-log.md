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
- A1: the reply slot on `Event::Api` is a ticket number, answered by `Effect::ApiReply`. `Event` stays plain data that can be cloned and compared, and the shell keeps the channel beside the ticket.
- A1: an act route selects what it acts on and leaves it selected, as a user who pressed the key would. Electron's stack-order routes leave the selection alone. A patch (`/canvas/apply`) does not touch the selection.
- A1: a patch item is turned into a `.canvas` node and read with the file reader (`Entity::from_node`), so the API and a saved file cannot disagree about a field. A value the reader cannot type is a 400 naming the field. Electron stores it.
- A1: ids are `<kind>_<16 hex>`, `edge_` and `ann_`, from a sequence in `specular_api::Api`. `auto-layout` in the CLI tells a group by its `group_` prefix.
- A1: deleting a group takes what is inside it, and deleting an entity takes the edges that lose an end (ADR 0034, what Delete does in the window). Electron's `delete <groupId>` removes the container only.
- A1: an edge to an entity the canvas does not hold is a 400. Electron stores it, and SKILL.md lists that as a known limitation.
- A1: a text the API adds or changes is resized to fit its words in the same undo step. A sticky only grows, so `--size` holds when the text fits. A group made in a patch is not refitted to a member moved later in the same patch (`group_fit` skips new groups).
- A1: a write while a drag is in flight is refused with 409. The user's drag owns the document until the button comes up.
- A1: the server is `tiny_http` on one thread with no async runtime. Requests queue on a channel and a winit user event (`ShellEvent::Api`) wakes the loop. A benchmark run and a headless run start no server.
- A1: port and discovery. The app takes `SPECULAR_PORT` or 29979 and `~/.specular/specular-mcp.json` unless a Specular already answers `/health` on the port that file names, or the port is taken. Then it binds a port the system picks, writes `~/.specular/specular-native-mcp.json`, and logs the `SPECULAR_DISCOVERY_FILE=` line that points the CLI at it. Started first, the Rust app holds 29979 and the Electron app then starts with no API.
- A1: `--tab` is accepted on the tab-scoped routes when it names the one open canvas (id is the file name, name is its stem). `tab new`, `switch` and `delete` are 501.
- A1: undo and redo are `POST /history/undo` and `/history/redo`. Electron has no route for either and the CLI has no verb. The canvas screenshot is Electron's `POST /window/screenshot`, drawn offscreen from the scene, with an optional `path` in the body to get a file instead of base64.
- A2: no second CLI. `src/main/cli.ts` is a thin HTTP client and runs against the Rust app unchanged. `tests/` holds no recorded HTTP requests, so the contract test replays the patch bodies of `tests/integration/canvas-apply.test.ts` case by case (`specular-api/tests/api/contract.rs`).
- C2: the agent chat panel is deferred, so a comment's text is typed in a small composer on the canvas beside its pin. It stands in for the right-panel composer and goes when that lands. Enter keeps the comment, Shift+Enter breaks the line, a press elsewhere keeps it if it has text, Escape drops it.
- C2: a draft is an `Annotation` in the session, not in the document. Keeping it is one `InsertAnnotation` step; an empty or escaped one leaves no step.
- C2: annotations are undoable, as in Electron (`DOC_MAP_ANNOTATIONS` is in the undo manager's scope). Create, resolve and delete are one step each.
- C2: page questions are an `Effect` answered by an `Event` that repeats the question, so nothing waits in the session. `QueryElement` is answered from `PageSource::element_at`, a synchronous probe only the synthetic source implements (a 160x48 grid of fake cells). `QueryRegionGrab` is answered with no grab by both shells. The CEF answers are marked `FOLLOW-UP(C2)`.
- C2: a click that finds no element on a page makes a canvas point, as Electron's does. Until CEF answers, every click on a real page is a canvas point and every region is canvas-bound.
- C2: Escape is staged: an open draft is dropped, else a focused comment loses focus, else what it did before. The comment tool stays armed after a comment is kept.
- C2: a comment and the selection are never both the target. Focusing a comment clears the selection, selecting clears the focus, so Delete deletes whichever there is. Delete on a grouped badge removes every comment under it, as Electron's popover does.
- C2: Electron has no key for annotate-selection or resolve (a popup button, a popover button, a route and a CLI verb). They are items in a Comment menu with no key.
- C3: a canvas-point comment draws the count pill centred on its point, replacing F5b's dot. Electron draws nothing there and lists it in the right panel, which is deferred.
- C3: the badge number is the message count of its group (`1 + replies`, summed over comments on the same element or page point), as in Electron. It is not an index.
- C3: a region is hit within 6 px of its edge, not across its interior, so what is under it stays reachable. Electron takes the whole rect.
- C3: focus is native chrome, since Electron shows it in the panel: a 2 px blue ring 3 px outside a pill, and a region at full opacity with a 10% fill (Electron's hover).
- C3: `App::comment_marks()` is the one set the hit-test and the scene share. It holds the status filter, the URL gate, the grouping and the geometry.
- CEF: a page says things about itself as `PageNotice`s into `Session.pages` (`App::page_state`): title, address, load, can-go-back and forward, scroll, devtools websocket. Only the address is saved: it is written to the page entity with no undo step, as Electron's `page.url = url` is.
- CEF: a changed page URL is `Effect::Navigate`, never a close and a create. Back, forward, reload and stop are the same effect, from `Action::Page*`. Cmd+[ and Cmd+] walk the history only of an entered page, because on the canvas they restack; `Context::overlaps` is what lets two rows share a key.
- CEF: a page is asked about its DOM over CEF's in-process devtools channel, one `Runtime.evaluate` a question, and answers as a `PageEvent`. The shell's `PageQueries` holds the question until then, and answers for a page that closed or crashed. `PageSource::element_at` is gone; the synthetic source answers the same way from its grid.
- CEF: a headless run hosts real pages when `--source cef` is named, with CEF pumped by the caller (`Pump::Caller`). A window still pumps from the run-loop timer.
- CEF: each process gets its own CEF root cache folder in the temp directory. On a shared root a second launch handed itself to the first and crashed it.
- CEF: `GET /pages/<id>/cdp-target` answers only while the canvas has one page. agent-browser drives the first page on the port whatever socket it is given, so with several pages the CLI's verbs would act on the wrong one. Electron's per-page CDP proxy is the missing piece.
- Scroll: a page-bound region's `docRect` is in document pixels and is drawn less the live scroll. An anchored entity or an element comment follows from the scroll stamped at placement; with no stamp it stays pinned, so older files draw as before. Out of the page it is hidden, and clipped at the edge, with no fade.
- P5: `App` owns one `Space`: the folder and every canvas in it, each whole (`Document`, `History`, camera, selection). The active canvas's three are `App`'s own `document`, `history` and `session` fields, so nothing that reads the app changed. The others are parked in their `Canvas` entry. A switch moves three structs out and three in, and a parked canvas keeps its undo stack. Electron serialises the tab it leaves and throws its history away.
- P5: every canvas of the space is read at open, not on first switch. The listing, the sidebar's entity counts and a `--tab` read then need no I/O, and a background canvas can follow its file.
- P5: a background canvas has no page hosts. A switch closes every page of the canvas left and creates every page of the one entered, even where two canvases share a page id. Nothing was measured. A page reloads when its canvas comes back and loses its scroll and its history.
- P5: the space's index is the Electron app's file, `.specular/workspace-meta.json`, read for ids, names, order and the last active canvas, and written on every switch, new, rename, duplicate and delete. Keys this app does not use are kept. Canvas files are named as Electron names them, `<name>-<4 of id>.canvas`, and an unsuffixed file from before that is read where it lies and not renamed. A `.canvas` file the index does not list is adopted with an id made from its file name, so the id is the same on every launch.
- P5: which space opens. A path on the command line wins: a folder is the space, a `.canvas` file opens its folder as the space and shows that file. Then `spacePath` in the Electron app's `preferences.json` (read, never written), then its old `workspaces/default`, then the folder last chosen here with File > Open space…, which this app keeps in its own preferences. With none of those, the demo grid. A folder from settings that is not there is not made or opened: Electron prompts for a missing space, and making an empty one would answer for the user.
- P5: a `--bench`, `--snapshot`, `--script`, `--pages` or `--annotations` run opens no space. It shows one document and writes nothing, as before.
- P5: canvas names are unique once trimmed, everywhere. Electron refuses a duplicate only in `tab new`. Here a rename to a taken name is refused too, a new canvas takes the next free `Canvas N`, and a copy is `<name> Copy`, then `Copy 2`.
- P5: a duplicated canvas keeps its entity and annotation ids. Electron remaps page ids because its page hosts are global. Ids here belong to a canvas.
- P5: deleting a canvas is not undoable. Its file goes to the system trash (the `trash` crate), which is the way back.
- P5: a `--tab` write to a background canvas runs with that canvas standing where the active one does and the user's whole session set aside, then everything is put back. It is one undo step in that canvas's own history, it may add pages (they are hosted when the canvas is shown), and it goes through while the user drags. Electron refuses pages there and keeps the write out of undo. A `--tab` read builds a one-canvas `App` from a copy of that document, so every route handler reads it unchanged.
- P5: the five canvas operations are one `Action::Canvas(CanvasAction)`. Rename, duplicate and delete take `None` for the active canvas, so their menu items never change.
- P5: `menus` now starts with a Canvas menu that ends in the space's canvases, and `MenuItem::label` is a `Cow`. The shell rebuilds a menu whose labels or actions changed and only updates states otherwise.
- P5: the menu's Rename canvas… opens the system save panel with the name filled in and takes what is typed. No system dialog asks for a line of text. It goes when the sidebar renames in place.
- P4: the sidebar is `sidebar(&App) -> SidebarModel`, with no renderer yet (the built-in chrome renderer had not landed). A group with notes and pages has a row in each section, as CONTEXT.md says. An entity whose `parent` names nothing is listed at the top level. Electron hides it.
- Scratch space: with no path the app opens a copy of the starter space in its own data folder. The user's space needs `--space user` or a path, because it is their real work and autosave writes into it. The folder remembered from File > Open space… counts as the user's space, so it also needs the flag.
- P1/P2: a property change is `Action::SetProperty(Property)`. One variant per property, not per kind: a colour, size or stroke-width pick goes to every selected item it means something for and skips the rest, as one `Command::Batch`. A pick that changes nothing records no step. Selection picks never write tool defaults; only the tool popup does (ADR 0008).
- P1/P2: a property set during a text edit does not end the edit. The edit's commit rebuilds the kind from the entity as it then is, so the property survives.
- P1/P2: a viewport preset keeps the page's recorded orientation, as Electron's `setDevicePreset` does, and no recorded orientation counts as portrait. Electron has no lock-aspect property and no sticky-versus-plain control; `Property::TextStyle` exists with no control in the popup.
- P1/P2: the models (`toolbar`, `popup_for`) hold no pixels, colours or hover state, and icons are named by an enum, so egui, GPUI or the built-in renderer can draw them. A mixed-kind selection, several edges and the page tool have no popup: everything Electron puts there needs an action this app lacks.
- P1/P2: built-in panels are off in a new `App` and turned on by `Event::BuiltinPanels(true)`, which the window and a headless run send. `view()` never draws them; the shell calls `draw_panels` after it. Existing tests and scene snapshots are untouched, and a UI-library shell sends nothing.
- P1/P2: layout and pointer state for the built-in panels live in `specular-interact` (`panel/builtin`), because `hit_test` needs the rects and the scene crate depends on interact. The scene side only paints the layout.
- P1/P2: a popup clamps under the toolbar and to the viewport edges and never flips below its item, as Electron's `popupStyle` does. Wheel and pinch over a popup move the canvas; the toolbar and an open list swallow them. A press outside an open list closes it and goes no further. Escape closes an open list before anything else.
- P1/P2: toolbar buttons run `SetTool`, and Draw and Comment switch back to Select on a second click, as in Electron. The Tools menu keeps running the key binding's action.
- P1/P2: text size in the popup is the named sizes and a stepper. Electron's typed field needs a text input the panels do not have.
- GPUI-SHELL: the effect runners are `specular_app::Runtime<W: ShellWindow>`, and `specular-app` is a library with a thin `main`. The files stayed where they were (`app/*_run.rs`) so the other agents' edits still apply. A new crate would have moved twenty files under them.
- GPUI-SHELL: the canvas view fills the window and never moves. The slot GPUI leaves unpainted is the app's viewport, and the surface draws with the camera and the screen-space items shifted by the slot's corner. The compositor is unchanged.
- GPUI-SHELL: the slot runs up under the Kit toolbar, because the app's layout already assumes a 44 px toolbar over the top of its viewport. Only the sidebar's width offsets it.
- GPUI-SHELL: `Event::BuiltinCanvasPopups` turns on the built-in popups beside a canvas item and nothing else. The Kit draws the toolbar and `PopupAnchor::Toolbar` popups. `PopupAnchor` is the dividing line, as ADR 0040 said.
- GPUI-SHELL: a model menu item's key is bound in GPUI under a context no element has. macOS shows it in the menu, and over the canvas the key goes to `update` as `Event::Key`, so the binding table decides, as in the winit shell. The shell's own keys (Cmd+Q, W, O, S, comma) are real GPUI bindings.
- GPUI-SHELL: the `NSEvent` monitor only notes each key event. GPUI still routes the key, and the slot's key handler turns the note into a `KeyInput` through `translate.rs`'s tables by way of winit's `PhysicalKey::from_scancode`. Composition comes through GPUI's input handler as `Event::Ime`.
- GPUI-SHELL: sidebar rows and swatches are plain GPUI elements in the Electron metrics. The Kit's `SidebarMenuItem` takes a string label, so it cannot hold the rename field, and the Kit has a colour picker but no swatch row. A `Stepper` is two Kit buttons around the value: the model has no action for a typed number.
- Performance: caches live in the compositor behind named types (`MeshCache`, `Laid` text layouts, `Batcher`) and in the shell (`FrameDemand`). `Scene`, `view` and `update` are untouched and still pure.
- Performance: a frame is drawn only when something it shows changed. Every `dispatch` owes one; the clock goes through `demand::tick`, which owes one only when the caret blinked or a held selection scrolled. Frames keep coming for 250 ms after input so a ProMotion display does not drop its rate inside a gesture.
- Performance: kept text is placed by moving the pass viewport, which lands on whole pixels. A pan by part of a pixel draws text up to half a pixel off while it moves, and the next frame at rest lays it out again.
- Performance: the idle memory sample is taken at 12 seconds, not 6. The spike's "16 to 20 percent more than Electron" was the 6 second sample.
- P3/P4: a typed value is `Control::Field` with a `FieldSubmit` that turns the text into an `Action`, so the model holds no editor. The built-in renderer edits it with the text editor as `Target::Field`; the GPUI shell uses its own input. Enter and a press elsewhere commit, Escape restores.
- P3/P4: Electron has no key or menu item for the left sidebar, only the toolbar button, and starts with it hidden. Native does the same. `Session.sidebar` holds shown, folds and opened rows outside `Session.panel`, and `App::covered_left()` is the width every fit, reveal and popup clamp reads. The canvas coordinates are not shifted.
- P3/P4: a sidebar row sends `Action::Reveal`, which selects and pans only when the item is not wholly in the uncovered area, keeping the zoom, as Electron's `focusCanvasBounds` does. The move is instant.
- P3/P4: the context menu is `context_menu(&App, &MenuTarget, at)`, a `PopupModel` of `Control::Choices` at `PopupAnchor::Point`. A right press selects its target first, because native actions act on the selection. Electron's is an OS menu with fewer items; this one adds the Edit items and a menu on empty canvas.
- P3/P4: arrange is one-shot (`Action::Arrange`, `span-arrange.ts` ported), one undo step. Focus is the camera framing only; Electron's focus session is deferred.
- P3/P4: the built-in layout is kept in `PanelUi.cache` behind a value stamp of everything cheap to compare, and forgotten at the end of every `update` except idle moves, wheels, pinches and ticks. Hover and press are patched into the kept layout.

## Needs a human at a Mac

What nobody has done by hand. The scenario scripts (`native/fixtures/scenarios`) and the tests already prove what is drawn, every gesture's result in the document, undo, text editing, the saved file and the headless clipboard, so none of that is here. Everything below needs the real window. Start with `cargo run -p specular-app -- <a copy of fixtures/kitchen-sink.canvas>`.

1. Look. One menu bar (Specular, File, Edit, Tools, View, Window), with `--source cef` too. Text is sharp on a 1x and a 2x display. Cards have a soft shadow. Pinch with text on screen: glyphs should not shimmer or go blocky, and sharpen a frame after you stop (else narrow `MIN_STRETCH` and `MAX_STRETCH` in `scene_pass/raster_hold.rs`).
2. Drag. A move snaps to the grid and feels attached. Shift mid-drag, Option-drag (the ghost is the item, faded), every handle on each kind, the corner cursors, a marquee. Drag from where an edge crosses a sticky: the sticky moves. Click there: the edge is selected.
3. Groups and edges. Drag an item over a group: the ring shows and the release puts it in. Double-click a group to work inside it, double-click its title to rename it, also zoomed out past half. Drag from an anchor dot to another item. Double-click an edge to label it. Cmd+] and Cmd+[ restack.
4. Keys and menus. Cmd+D, Cmd+Z, then an arrow: the originals move. Cmd+Z, D, A, =, 1 each act once, not twice. The active tool is checked in Tools. Undo, Copy and Delete are grey with nothing to act on.
5. Pages, with the CEF bundle (`native/README.md`, "Build", with `debug` for `release`) on `native/fixtures/input.canvas`, then on a copy of `native/fixtures/pages.canvas`. In this order:
   1. One click selects, a second enters, Escape leaves. A click in the page flips its background.
   2. Type into the text input: letters arrive once, Backspace and the arrows work, Cmd+A, C, V and Z act in the page. V, R and Backspace typed in the page do not switch tool or delete it.
   3. With a Japanese or Pinyin input method in that input: marked text is underlined before commit and the candidate window is by the caret.
   4. Open the `<select>`: the list draws over the page at the right place and picking an option closes it. Then the same at zoom 0.5 and 2.
   5. Scroll the tall page with the trackpad while entered: it feels like a browser, with momentum, and does not pan the canvas. Not entered, the same gesture pans the canvas and the page stays put.
   6. The title above a page reads `Title — address` and says `Loading…` during a load. Click a link in example.com: the title follows, the saved file has the new URL, and Cmd+[ goes back, Cmd+] forward, Cmd+R reloads, Cmd+. stops. With the page only selected, Cmd+[ restacks it and Page > Back still goes back.
   7. Press C, click the heading of the tall page: the composer opens with the heading outlined. Drag a region round the button: it belongs to the page. Scroll the page: the region moves with the button, is cut off at the page's edge and gone past it. A drag over blank page makes a canvas region that stays put.
   8. Drop a sticky on the tall page, scroll: the sticky moves with the content and can be grabbed where it is drawn. Drag it and scroll again.
   9. Start a second copy of the app while the first runs: both stay up.
6. Text. Double-click a sticky: the caret sits between glyphs at zoom 0.25, 1 and 3, blinks once a second, and typing does not lag in a few hundred words. With a Japanese or Pinyin input method: the marked text is underlined, the candidate window is by the caret, letters arrive once, and Escape cancels the composition before it ends the edit.
7. Documents. Tools > Document, click, type: `Untitled Note.md` appears and fills in a third of a second after you stop. Edit the file elsewhere while the edit is open: a conflict copy appears. Quit mid-edit: the last keys are in the file. Try Cmd+Option+1, which macOS may take.
8. Clipboard. Copy two shapes and their edge, paste, paste in a second window. A URL from a browser pastes as a page, a sentence as a sticky, a screenshot (Cmd+Ctrl+Shift+4) as a file in `assets/`.
9. Drop. A png and a `.md` from Finder, from inside and outside the space folder. They land at the pointer's last position before the drag, which may be wrong.
10. Files. A change is on disk a third of a second later with the camera in `appState`. An edit to the file from outside is followed and the camera kept. Quit within that third of a second: saved. File > Open replaces the canvas. The title shows ` — Edited`. R, Shift+R, quit, start: the shape tool is still a diamond. png, jpeg, webp and gif files appear; a missing file and an svg stay cards.
11. Speed. One `--bench --chrome on` run against an older build.
12. From M1: the egui checks at the end of ADR 0039.
13. The API. Quit the Electron app, run `cargo run -p specular-app -- FILE.canvas`, then `specular canvas`, `specular add note "hi"`, `specular add page https://example.com`, `specular focus <id>`: each shows in the window and Cmd+Z takes it back. `curl -X POST -H "x-specular-secret: $SECRET" localhost:29979/window/screenshot -d '{"path":"/tmp/shot.png"}'` writes what the window shows (the BGRA swap and the 2x size are unchecked). Start the Electron app first and the Rust app second: the log names the fallback port and file, and the CLI with that `SPECULAR_DISCOVERY_FILE` reaches the Rust app.
14. Comments. Press C. Click the canvas, type, Enter: a blue pill with 1 appears and the tool stays armed. Drag a region, type, click away: a dashed rose rect. Click a pill: it gets a ring; Escape takes it off; Delete removes the comment and Cmd+Z brings it back. Select two items, Comment > Annotate selection. Comment > Resolve comment hides one. Type Japanese in the composer.
15. Spaces. Quit the Electron app. `cargo run -p specular-app` with no path opens the scratch space, titled `Welcome (scratch space)`. `cargo run -p specular-app -- --space user` opens the space Electron has in Settings, on the canvas Electron last showed, and the window title is that canvas's name. Then, in this order:
   1. Canvas menu: the canvases are listed with the active one checked. Choose another: its pages load, the first canvas's pages stop. Come back: the camera and selection are where you left them and Cmd+Z still undoes what you did there before the switch.
   2. Canvas > New canvas, draw something, Canvas > Duplicate canvas, File > Rename canvas… (the save panel: type a name, press Save), Canvas > Delete canvas. After each, look at the folder in Finder: the file is made, copied, renamed, and in the Trash. Put it back from the Trash and reopen the space.
   3. Edit a background canvas's file in a text editor, then switch to it: it shows the edit.
   4. File > Open space…, pick an empty folder: the Welcome canvas and `Welcome.md` appear and the note's text shows. Quit and start with `--space user`: with `spacePath` set in Electron, Electron's space opens, not this one.
   5. Start the Electron app on the same space after the Rust app made and renamed canvases: it lists the same canvases under the same names.
   6. `specular tab`, `specular tab new scratch`, `specular add note "hi" --tab scratch`, `specular canvas --tab scratch`, `specular tab switch scratch`, `specular tab delete scratch`: the note is there when you switch, and you were not moved before that.
16. Panels. Pick each tool from the toolbar, recolour a sticky, change a shape's kind and border, restyle an edge, change a page's size. Check the glyphs against the Electron toolbar side by side, that a popup follows its item through a pan and a zoom without lag, and that nothing under a popup or the toolbar takes the click.

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

### A1, A2 and the canvas half of A3. See `git log -- native/crates/specular-api`

- New crate `specular-api`, with no socket in it. `Api::plan(&App, &Request)` returns `Plan::Answer` for a read, `Plan::Run { event, pending }` for a write, or `Plan::Screenshot`. The event is `Event::Api(ApiCall)`. After `update`, `Pending::finish` reads the response off the `App`. `Api::answer(&mut impl Host, &Request)` does the round trip, and the shell and the tests each implement `Host`.
- Interact: `ApiCall`, `ApiRun::{Act, Apply}`, `ApiOutcome`, `Effect::ApiReply`. The window's verbs now share their builders with the API: `delete_commands`, `move_commands`, `group_command`, plus `fit_camera`, `iso8601`, `App::scope_of`. Doc: `Entity::from_node`, `Entity::to_node`, `tidy_json`.
- Shell: `src/api/` (server thread, discovery file, port fallback), `app/api_run.rs` (`impl Host for Shell`), `offscreen.rs` (the old `headless/target.rs`, now also the screenshot target).
- Tests: `specular-api/tests/api/` has `plan.rs` (request to event, nothing changed), `contract.rs` (15 cases from the Electron suite), `verbs.rs` (through `update`). `specular-app/src/api/tests.rs` runs real HTTP on an ephemeral port against a `TestApp` loop and snapshots a session. By hand, once: the installed `specular` CLI against that test server ran `add note`, `add page`, `update`, `link`, `group`, `delete`, `apply` with a layout, `focus`, `annotate`, `annotate-selection`, `annotations`, `find-placement`, `tab`, `canvas`.
- SKILL.md verbs, done: `canvas`, `selection`, `add page`, `add note`, `update` (`--at`, `--size`, `--preset`, `--text`, `--color`, `--url`), `delete`, `group`, `ungroup`, `focus`, `link`, `unlink`, `find-placement`, `apply`, `upsert`, `annotate`, `annotations` with `--status` and `--all`, `ack`, `resolve`, `dismiss`, `reply`, `annotate-selection`, `tab` (list), `--tab` naming the open canvas.
- Partial: `add note` never routes long text to a `.md` file, it stays a sticky. `add file` takes the path as given with a 400x300 box, with no copy into the space and no sizing from the image. `annotation <id>` has `selection.members` but `priorFeedback` is empty and there is no region screenshot. `update --gap` is ignored (no managed layout). Pages get no device-frame metadata, and `--landscape` only swaps the size. Placement counts every entity as occupied and knows no device bezels beyond the insets the CLI sends.
- Not implemented, each a 501 that names the verbs: `snapshot`, `screenshot -f`, `print-pdf`, `click`, `fill`, `type`, `select`, `scroll`, `back`, `forward`, `reload`, `wait`, `find`, the agent-browser passthroughs, `record` (all need CEF, which is the rest of A3), `arrange`, `auto-layout`, `breakpoints`, `component-states`, `design-system`, `tab new`, `tab switch`, `tab delete`. `skills` never reaches the app.
- For the next agent: a new route is one arm in `Api::route` and a handler that returns `Step`. A write builds a `Command` or picks an `Action` and never touches the `App`. Add a 501 row in `unported.rs` for anything the CLI can call that you leave out.
- For the next agent: the page verbs all start with `GET /pages/<id>/cdp-target`, which wants a CDP websocket URL. CEF's remote debugging port is the likely way to answer it.
- Gate: fmt, clippy and `cargo test --workspace` pass. The scenario scripts were not run, since they start `specular-app`.

### C1, C2 and C3: comments on the canvas. See `git log -- native/crates/specular-interact/src/comment`

- Doc: `Annotation::new`. The model, its three commands and the `.canvas` shape were F2's and F3's; `tests/annotation_shape.rs` holds what this app writes against Electron's fields.
- Interact, `comment/`: `drag` (the one gesture: a click under 4 px, a region past it; `Gesture::Comment` replaces `CommentRegion`), `create`, `grab` (the pure rule, first page with elements wins), `draft` (the draft and the composer's frame), `selection` (`selectionEntityIds`, `selectionTarget`), `shown`, `marks`, `actions` (focus, resolve, delete).
- New: `Effect::{QueryElement, QueryRegionGrab}`, `Event::{ElementAt, RegionGrab}`, `PageGrab`, `PageRegion`, `Hit::Comment`, `Target::Comment`, `Action::{AnnotateSelection, FocusComment, ResolveComment, DeleteComment}` (`None` means the focused mark), `App::{comment_draft, comment_composer, comment_marks, focused_comment}`, `matches_page_url`, `ScreenRect` made public. Core: `PageElement`, `PageSource::element_at`.
- Scene: `view/annotations.rs` draws the marks and `view/comment_draft.rs` the draft marker and the composer. Page-bound marks move and scale with their page and are gone when its URL differs from `pageAnchor.pageUrl` (hash ignored), when the page is gone, and when resolved or dismissed.
- Shell: both effect runners answer the two questions. The script format has `act annotate-selection` and `act resolve-comment`. A Comment menu holds the three items.
- Tests: `specular-interact/tests/{comments,comment_drafts,comment_state,comment_marks}.rs`, `specular-scene/tests/comments.rs`, more in `gestures.rs` and `menus.rs`. Testkit: `comment`, `with_comment`, `answer_element`, `answer_grab`, `comment_draft`.
- Fixtures: four more annotations in the kitchen sink (a grouped element badge, a region on a stale URL, a selection region, a point with a reply) and scenario `i-comments`, with its PNGs looked at.
- For the next agent: the CEF half is the two `FOLLOW-UP(C2)` notes. The answer should arrive as a page event, and `element_at` then goes. Page scroll is still taken as zero, so a `docRect` and an element box do not follow a scrolled page.
- Not done: the hover outline still shows on a page under the comment tool. No way to edit a kept comment's text or to dismiss one. No `metadata.pageName`. Deleting a page leaves its comments bound to it, so they stop drawing; Electron frees them. A resting region is faint over a dark page (50%, as Electron). The API crate's routes do not reach comments yet.
- A headless run's clock starts at zero, so the scenario's comments are dated 1970.
- Built in a detached worktree in three steps by subagents, squashed and rebased onto the polish and API commits. The rebase needed one `Hit::Comment` arm and re-accepted scene snapshots for the new title and selection lines.
- Gate: fmt, clippy and `cargo test --workspace` pass on the rebased commit (1459 tests, GPU ones included). `fixtures/scenarios/run.sh` passes.

### CEF path: real pages through the new architecture. See `git log -- native/crates/specular-cef/src/devtools.rs`

- The CEF build had not rotted: clippy and a debug build with `--features cef` passed on HEAD, and the bundled app ran `input.canvas` and a two-page canvas with no panic, import failure or wgpu error. One real bug found by running it: a second launch crashed the first (shared CEF root cache); fixed. The debugging port falls back from 9222 to a free one.
- Core: `PageEvent::{Title, Url, Loading, Scrolled, ElementAt, ElementsInRect, DevtoolsTarget}`, `PageNav`, `PageSource::{navigate, query_element, query_elements_in_rect}`, `CssRect`. The synthetic source has a history, a scrolling three-viewport document and a grid in document space.
- CEF crate: `dom_query` (pure, the devtools messages and their answers), `devtools` (the channel), a display handler, loading state and scroll in `client`, `host_call` split out of `source`, `Pump`.
- Interact: `page_state.rs` (`PageState`, `App::{page_state, page_scroll}`), `PageNotice` variants, `Effect::Navigate`, `Action::Page{Back,Forward,Reload,Stop}`, `Context::{EnteredPage, PageTarget}`, a Page menu, `scroll_follow.rs`. Scene: the title line, shifted and clipped anchored items and marks. Testkit: `page_reports`.
- Shell: `page_queries.rs`, `page_notice.rs`. `--snapshot` and `--script` take `--source cef`; script actions `act page-back`, `-forward`, `-reload`, `-stop`. `fixtures/pages.canvas` and `fixtures/scenarios/cef/pages.txt` are the real-page scenario (not in `run.sh`, it needs the bundle); its PNGs were looked at: pages beside stickies in stack order, an element comment on `#top`, a region bound by the button it grabbed and following a scroll.
- API: `POST /pages/<id>/{back,forward,reload}`, `GET /pages/<id>/cdp-target`, and `update --url` navigates in place. Run against the real app: back, forward and reload moved example.com and example.org through one host.
- Not done: the CLI's browse verbs (`snapshot`, `click`, `scroll`, ...) with more than one page. They reached CDP but drove the first page, so `cdp-target` refuses then (see Decisions); it needs a port of `src/main/cdp-proxy.ts`. `specular back` and friends go the same way, so only the HTTP routes work for those.
- Not done in scroll-follow: a resize does not fold the shift, the caret of an anchored text edited while shifted is off by the shift, edges attach to stored rects, element attachment (ADR 0032) is not tracked, and the cut at the page edge is hard where Electron fades.
- Rough edges seen: a `data:` URL fills the title line; sync sets (`syncId`) do not navigate together; the profile is in memory, so logins do not survive a quit.
- `i-comments`'s check now expects its second region to be page-bound: the synthetic grid answers the grab.
- Built with one subagent on the pure half (interact, scene, testkit, API) while the CEF half was written and run. Gate: fmt, clippy with and without `specular-app/cef`, `cargo test --workspace` (1526), `fixtures/scenarios/run.sh`.

### P5 and the model half of P4: the space folder and its canvases. See `git log -- native/crates/specular-interact/src/space.rs`

- Interact: `space.rs` and `space/` (`Space`, `Canvas`, `CanvasId`, `OpenedSpace`, the operations in `ops.rs`, `in_background`, `resolve_tab_ref`, the names and file names). `App::{space, canvas_document, canvas_camera, canvas_to_save, background}`. `Event::{SpaceOpened, CanvasFileChanged}`, `Action::Canvas(CanvasAction)`, `Effect::{WriteCanvas, RenameCanvasFile, TrashCanvasFile, SaveSpaceMeta}`. `Effect::Save` now means the active canvas. `ApiCall` has a `canvas`.
- Interact: `sidebar.rs`, the pure sidebar model: the Canvases list, Notes and Pages, a page's hooked items and open comments, selected and dimmed, and each row's `Action`.
- Shell: `space/` (`locate`, `listing`, `starter`, `files`, `open`), `app/space_run.rs`. `SpaceFiles` holds one `Persistence` a canvas, so autosave and the 500 ms file check cover every canvas, shown or not. File has Open space… (Cmd+Shift+O) and Rename canvas…, and Open… switches when the file is in the open space.
- API: `GET /tabs`, `POST /tabs`, `/tabs/switch`, `/tabs/delete`, and `--tab` on `/canvas`, `/canvas/apply` and `/layout/*` for any canvas. `Api::new` takes no tab. Errors are Electron's resolver texts.
- Testkit: `TestApp::with_space([(name, document)])`, `space(..)`, `canvas_names`, `active_canvas`, `canvas_id`, `switch_to`.
- For the sidebar renderer: draw `sidebar(app)` and send each row's `action`. Rename in place is `CanvasAction::Rename`. Nothing selects and zooms to a row's entity yet. A row sends `Action::Select`, so add the camera move there.
- Not done: a `.canvas` file added to or removed from the folder by another tool is not seen until the space is opened again. The starter space is found in the repository's `resources/`, or in `Contents/Resources/starter-space` of a bundle, which `bundle-macos.sh` does not copy yet. Both apps open on one space will overwrite each other's index. Electron unlinks an unsuffixed canvas file after reading it, so after Electron has run, `Welcome.canvas` has its suffixed name.
- Changed for others: with no path, the app opened the user's real space and saved to it. The scratch-space entry below reverses that. Use `--pages N` for the demo grid. The default canvas of an app with no space is `tab_1`, `Canvas 1` (the API tests said `Canvas`).
- Nothing was run in a window. Gate: fmt, clippy, `cargo test --workspace` (1576). New tests: 18 on the space and 7 on the sidebar through the testkit, 8 on the tab routes, 14 in a temp folder on what each operation leaves on disk.

### Scratch space by default. See `git log -- native/crates/specular-app/src/space/locate.rs`

- With no path and no `--space`, `specular-app` opens the scratch space: `<app data>/scratch-space`, seeded from the starter space, kept between launches. The startup log names the folder and the title reads `Welcome (scratch space)`.
- `--space user` opens the Electron app's space, else the folder remembered from File > Open space…. `--space PATH` is the same as a bare `FOLDER` or `FILE.canvas`. `--space user` with a path is refused.
- Electron's `preferences.json` and this app's remembered folder are read only under `--space user`.
- `SpaceChoice` and `SpaceStart::scratch` in `space/locate.rs`; `Shell::scratch` drives the title, and File > Open space… to another folder drops the mark.
- Run once with no arguments against a throwaway `SPECULAR_NATIVE_CONFIG_DIR`: it seeded and opened `scratch-space` there. Gate on `specular-app`: fmt, clippy, 186 tests.
### P1 and P2: toolbar and item popup, with a built-in renderer. See `git log -- native/crates/specular-interact/src/panel`

- Actions: `Action::SetProperty(Property)` in `property.rs` and `property/{apply,page,read}.rs`. `Property::applies_to(&App)` and `property::read::*` give what a control needs to show its state. The viewport preset table moved to `specular_doc::VIEWPORT_PRESETS`.
- Models: `specular-interact/src/panel/`. `toolbar(&App) -> ToolbarModel` and `popup_for(&App) -> Option<PopupModel>`, an exhaustive match in `popup.rs` with one file per subject under `popup/`. Controls are `Button`, `Toggle`, `Swatches`, `Dropdown`, `Stepper`, `Separator`, each with a `ControlId` and its `Action`.
- Built-in renderer: `panel/builtin/` in interact (`layout(&App) -> PanelLayout`, `Hit::Panel`, `Session.panel`, `Event::BuiltinPanels`) and `specular-scene/src/panel/` (`draw_panels`, screen-space items only). Icons are scene paths from a small SVG path parser; the data in `paths_*.rs` was extracted from Lucide and the toolbar's SVGs.
- A UI library replaces: `specular-interact/src/panel/builtin*`, `specular-scene/src/panel*` and `draw_panels`, `Session.panel`, `Event::BuiltinPanels`, `Hit::Panel` and its arms, the `builtin::` calls in `pointer.rs`, `update.rs`, `cursor.rs` and `hit.rs`, `specular-testkit/src/panels.rs`, the `control` script steps, `tests/panel_builtin*.rs` and `specular-scene/tests/panels.rs`. It keeps the models, `property*` and every `Action`.
- Tests: `tests/properties*.rs`, `tests/panels*.rs` (model dumps through `assert_popup_snapshot!` and `assert_toolbar_snapshot!`), `tests/panel_builtin*.rs`, `specular-scene/tests/panels.rs`. Testkit: `with_panels`, `click_control`, `control_rect`, `panel_layout`.
- Scripts: `control <id>`, `hover-control <id>`, `press-control <id>`, `panels on|off`; a wrong id lists the ids shown. Scenario `j-toolbar-and-popups` with nine checks in `check.py`. About seventy PNGs were read against the Electron CSS.
- Still off against Electron: no hand, inspect or theme buttons; the page tool has no preset list; the page popup has no navigation, URL or sync (`Action::PageBack`, `PageForward`, `PageReload` and `PageStop` landed beside this work and are not in the model yet); no annotate, focus or arrange-as-layout anywhere; formatting buttons show no on state; gradients in the tool glyphs are flat and their drop shadows are missing; tool buttons sit at y 8, not 7.5.
- For the next agent: a new popup control is a line in its `popup/*.rs` and, if it needs one, a `Property` variant. `layout` is rebuilt about three times per pointer move with text measured each time; cache it if a trace shows it. Changing a sticky's text size scales its height by the ratio, which is the S4 stand-in.
- Built in a detached worktree in four steps by subagents and rebased onto the pages, comments and spaces commits; the merge needed `Hit::Comment`, `Gesture::Comment` and `Target::Comment` arms. Gate: fmt, clippy and `cargo test --workspace` pass there (1711 tests); all twelve scenarios pass `check.py`.

### GPUI-HYBRID: GPUI Kit as the shell around our canvas view. See `git log -- native/bakeoff/gpui-kit-hybrid`

- A spike outside the task list. [ADR 0040](../adr/0040-gpui-kit-hybrid-shell.md), Proposed, verdict: works with named caveats. Nothing under `native/crates` changed.
- `native/bakeoff/gpui-kit-hybrid/`: its own workspace and target dir. gpui-kit 0.7.1 on gpui-pre 0.3.8 opens the window and draws toolbar, sidebar, menus and dialogs. `specular-compositor`, `specular-scene`, `specular-interact` and `specular-cef` are linked by path, unchanged, and draw into a child `NSView` under GPUI's.
- Measured: 120 fps for both renderers in every combination, with synthetic and with real CEF pages (zero-copy). Keys reach a CEF field from an `NSEvent` monitor and a composition from GPUI's input handler.
- The canvas view goes below GPUI's view, with a transparent GPUI window and the Kit root's background cleared. Above, overlays are hidden and the view leads GPUI by a frame on resize.
- `toolbar(&App)` and `sidebar(&App)` drove Kit buttons and sidebar rows with no adapter. Popups anchored to the canvas should stay in our pass.
- `scripts/run-all.sh` reruns every scenario into `out/` and copies the cited evidence to `shots/`. Each run opens a floating window for a few seconds. The CEF runs need `CEF_PATH`, a `--features cef` build into `target/cef` and `scripts/bundle.sh`.
- Pitfalls met: a covered window gets no drawables (the spike floats its window); objc2's debug checks reject a wrong struct encoding at the first send; events posted with `CGEventPostToPid` arrive with no window.
- Not in the gate: the spike is not a workspace member, has no tests and does not follow the workspace lints. It builds with no warnings and `cargo fmt` is clean.
- Needs a human at a Mac: the list at the end of the ADR (live window resize, real trackpad and input method, shortcuts with a page entered, VoiceOver, 40 pages).

### GPUI-SHELL: the GPUI Kit shell as a real crate. See `git log -- native/crates/specular-shell`

- `native/crates/specular-shell`, binary `specular`: one GPUI window, Kit toolbar, sidebar, tool popups, menus and a settings dialog stub from the pure models, and the unchanged compositor drawing the canvas into a view under GPUI's. `specular-app` (winit) still builds and runs, and stays until this one reaches parity.
- Shared, not copied: `specular_app::{Runtime, ShellWindow, launch, Launch, native_key_input, offscreen}`. `Runtime` has every effect runner, the API server, page hosts and the per-turn work. `launch` parses the command line and runs `--snapshot` and `--script`, so both binaries write the same PNG (compared byte for byte).
- Layout of the crate: `shell.rs` (open, tasks, exit), `canvas.rs` (the runtime beside GPUI, the model cache, the wake), `surface.rs` (`ShellWindow`), `native.rs` and `keys.rs` (AppKit), `view/` (slot, IME, toolbar, sidebar, popup), `view/controls.rs` (the one adapter from `Control` to Kit components), `menus.rs`, `theme.rs` (Electron light tokens), `pins.rs`.
- Pins: `gpui-kit = "=0.7.1"`, `gpui-pre = "=0.3.8"` in `native/Cargo.toml`. `NativeCanvas::install` checks the three unpromised macOS facts on the live window and the launch fails with the one that broke. A test fails if the lockfile has other versions.
- `specular-scene` exports `icon_svg` and `panel_color`, so the Kit draws the built-in renderer's own glyphs and swatch colours.
- Run and looked at, debug builds: kitchen-sink (synthetic) and pages and input (CEF, bundled with `bundle-macos.sh debug specular`). Canvas pacing, 4 s windows at 120 Hz: synthetic 120.0 fps, p99 9.5 ms, 0 late. CEF 119.6 to 120.0 fps, p99 9.0 ms, 1 late a window. Typed `hi11` into a CEF field with digit 1 and numpad 1. Typed into a sticky, Enter included. Tool popup, zoom list, item popup with handles, rename in place and the settings dialog all captured and read.
- For an agent with no hands: `SPECULAR_SHELL_SCRIPT="wait 2000; click 587 22; shot /tmp/a.png; quit"` posts real `NSEvent`s and captures the window. `SPECULAR_FLOAT_WINDOW=1` keeps it uncovered, `SPECULAR_FRAME_LOG=4` logs pacing. A covered window logs `window occluded` and draws nothing, which looks like blank pages in a capture.
- Not done: greyed menu items (one action type means AppKit validates them all as available; a disabled item does nothing when chosen), the context menu, collapsing sidebar groups, drag and drop inside the sidebar, the right panel, Linux and Windows. Rename canvas… in the File menu renames in the sidebar.
- Seen and not chased: Backspace in a CEF field removed two characters in one scripted run. The key text is winit's own (`\x08`), so check the winit shell before blaming this one. Opening `fixtures/x.canvas` makes `fixtures/.specular/`, as the winit shell does. Copy the fixture out first.
- `--bench` is refused here and stays in `specular-app`. The winit dependency also stays in the library until the key tables stop using `winit::keyboard::KeyCode`.
- Gate: fmt, clippy for the workspace and for `specular-shell --features cef`, `cargo test --workspace` (1727), `fixtures/scenarios/run.sh`.
- Needs a human at a Mac: the list at the end of ADR 0040, which now says what was checked by script.

### Performance, part 1: the bench and the numbers before any change. See `git log -- native/crates/specular-bench/src/work.rs`

- `--bench` runs on any canvas and each line has a `work` object: mean, p95 and max milliseconds a frame in update, view, cull, shaping, batching, tessellation, build, glyphs (glyphon's layout and raster), upload and submit, plus the most items, batches, draw calls, glyphs and triangles a frame drew. `--bench-target headless` draws into a texture with no vsync and times each frame until the GPU is done; `window` is the old path. `idle` is a seventh profile and `--bench-duration-ms` sets every profile's length.
- `SceneStats` carries `StageTimes`, `draw_calls`, `glyphs` and `triangles`. `specular-bench` has `work.rs`.
- Canvases in `native/fixtures/bench/`, written by `generate.py`: 500 and 2,000 stickies, 300 drawings, 200 edges between 400 shapes, 50 Documents over ten markdown files, and `mixed` (20 pages and 300 items). `run.sh LABEL headless|window synthetic|cef` runs pan, zoom and idle on all of them and the kitchen sink into `native/runs/perf/LABEL/`; `table.py` prints the table; `idle.py` is the 30 second idle run with the process tree's CPU.
- Before any change, M3 Max, 120 Hz, 1600x1000 at 2x, release build, CEF pages in a window. Milliseconds of CPU a frame; the budget is 8.3 and the pages need most of it. The full tables, the headless ones and the synthetic ones are in `native/runs/perf/baseline/table-*.md`.

| canvas | pan cpu mean / p95 | zoom cpu mean / p95 | idle cpu mean | largest steps (pan, ms) | items | batches | glyphs | triangles |
|---|---|---|---|---|---|---|---|---|
| documents-50 | 3.40 / 3.65 | 3.04 / 5.38 | 3.54 | glyphs 2.36, view 0.68, cull 0.27 | 150 | 2 | 62720 | 200 |
| drawings-300 | 6.36 / 7.11 | 4.55 / 6.98 | 7.17 | tessellation 5.06, view 0.86, upload 0.19 | 300 | 2 | 0 | 48186 |
| edges-200 | 1.71 / 2.68 | 2.19 / 3.05 | 1.87 | tessellation 0.55, batching 0.42, glyphs 0.27 | 1300 | 5 | 3463 | 8270 |
| kitchen-sink | 1.29 / 2.13 | 2.33 / 4.13 | 1.27 | tessellation 0.54, glyphs 0.24, submit 0.18 | 356 | 27 | 1999 | 6913 |
| mixed | 0.71 / 0.91 | 2.78 / 3.79 | 0.79 | glyphs 0.37, submit 0.25, view 0.05 | 191 | 62 | 4594 | 258 |
| stickies-2000 | 2.08 / 2.48 | 1.41 / 2.49 | 2.28 | glyphs 1.35, batching 0.32, view 0.20 | 1280 | 2 | 35653 | 1728 |
| stickies-500 | 1.94 / 2.27 | 1.35 / 2.09 | 1.83 | glyphs 1.35, batching 0.30, submit 0.10 | 1200 | 2 | 34604 | 1600 |

- Tessellation is the largest cost: 300 strokes take 5 to 6 ms every frame, panned or not. Glyph layout is next: 2.4 ms for 50 Documents (62,720 glyphs) and 1.3 ms for 1,200 sticky runs. Batching is 0.3 to 0.5 ms at 1,200 items, `view` 0.7 to 0.9 ms for Documents and drawings.
- Nothing is skipped. An idle canvas draws every refresh at full cost: 3,601 frames in 30 seconds, and the app's CPU was kitchen-sink (synthetic) 23%, stickies-500 (synthetic) 32%, drawings-300 (synthetic) 88%, documents-50 (synthetic) 47%, mixed (synthetic) 50%, kitchen-sink (cef) 23%, mixed (cef) 21%, static-20 (cef) 17%.
- Headless frame intervals are paced by a sleep and mean nothing; read `work`. In a window `gpu` is mostly the wait for vsync. Synthetic pages upload on the CPU, so `mixed` with them is not the real cost.

### Performance, part 2: what the numbers led to, and after. See `git log -- native/crates/specular-app/src/app/demand.rs`

Same machine and settings as part 1: CEF pages in a 1600x1000 window at 2x, milliseconds of CPU a frame. Full tables are in `native/runs/perf/after/` (not in git, like the rest of `runs/`).

| canvas | pan cpu, before → after (mean / p95) | zoom cpu, before → after | idle frames in 5 s | largest steps left in a pan frame |
|---|---|---|---|---|
| documents-50 | 3.40 / 3.65 → 1.04 / 1.14 | 3.04 / 5.38 → 0.96 / 1.08 | 601 → 2 | view 0.63, cull 0.19 |
| drawings-300 | 6.36 / 7.11 → 1.64 / 1.79 | 4.55 / 6.98 → 2.55 / 3.97 | 601 → 2 | view 0.89, upload 0.23 |
| edges-200 | 1.71 / 2.68 → 0.92 / 1.01 | 2.19 / 3.05 → 1.68 / 2.26 | 601 → 2 | tessellation 0.33, view 0.14 |
| kitchen-sink | 1.29 / 2.13 → 0.67 / 1.30 | 2.33 / 4.13 → 1.44 / 2.28 | 601 → 5 | submit 0.17, view 0.14 |
| mixed | 0.71 / 0.91 → 0.42 / 0.50 | 2.78 / 3.79 → 2.37 / 2.84 | 601 → 16 | submit 0.18, glyphs 0.14 |
| stickies-2000 | 2.08 / 2.48 → 0.51 / 0.61 | 1.41 / 2.49 → 0.45 / 0.52 | 601 → 2 | view 0.19, submit 0.13 |
| stickies-500 | 1.94 / 2.27 → 0.55 / 0.91 | 1.35 / 2.09 → 0.54 / 0.87 | 601 → 2 | view 0.12, submit 0.15 |

The after rows also draw the toolbar and popups, which did not exist for the before rows.

- Idle (`shell: app/demand.rs`, `app/turn.rs`). The loop slept never and drew every refresh. Now a turn with nothing owed draws nothing and the loop waits 50 ms, or 16 ms while text is edited. 30 second idle, app CPU and frames: 500 stickies 31.9% and 3,601 → 0.1% and 0; 300 drawings 87.9% → 0.2%; kitchen sink with CEF 23.4% → 0.6%, 0 frames; `mixed` with CEF 20.5% → 1.7%, 3 frames. `static-20` drew 664 frames because some of its live sites were animating. Synthetic pages always animate, so canvases with them are never idle.
- CEF's pump timer fired 240 times a second. It now fires when CEF asks (`OnScheduleMessagePumpWork`) with a 30 Hz fallback. `animated-20` still receives its frames (1,092 in a slow pan against 1,116 before).
- Tessellation (`scene_pass/mesh_cache.rs`). Canvas polygons and paths are tessellated once at the zoom rounded up to a power of two and copied under the camera each frame. A pan tessellates nothing. A zoom re-tessellates at each power of two, and every frame for strokes thinner than a device pixel and for drawings whose outline `view` widens as the zoom falls.
- Glyph layout (`scene_pass/text_hold.rs`, `text.rs`, `text_key.rs`). A text batch is laid out once with a 384 px margin and later frames move the pass viewport. It is laid out again when an item joins or changes, the camera passes the margin, or the glyph size changes. Canvas and screen text have an atlas each so each can be trimmed when its batches are all laid out.
- Batching (`scene_pass/batch.rs`). The overlap test goes through a 32 by 20 grid. 2,400 alternating items compare under 16 pairs each, where the plain rule compared 2.9 million pairs. A test checks the grid against the plain rule on scattered items.
- Memory, 20 static pages (`fixtures/bench/memory.py`, `specular-bench rss --per-process true`). Before: 4,372 MB at 6 s, 2,944 MB settled (renderers 1,798, GPU process 745, the app 328, others 73). After: 3,127 MB at 6 s and 2,584 MB settled (GPU process 536, the app 239). Electron's recorded idle is 3,623 MB. Three changes: the 4x multisample target is a transient attachment (about 100 MB of the app's), a page that has not painted for 2 s keeps one imported surface and not its pool of up to 8, and a page is created at the texture scale its zoom earns it.
- The spike's fixtures, three runs each, against the pre-fix build on the same day: worst-profile p95 is 8.78, 8.93, 9.28 and 9.17 ms for `static-9`, `-20`, `-40` and `animated-20`, against 8.81, 9.08, 9.13 and 9.12. No `drawsWithoutTexture`, every line representative. Idle footprint 1,373, 2,546, 4,203 and 2,370 MB against the spike's 2,428, 4,333, 6,522 and 2,683. Output in `native/runs/perf/spike/`.
- Not settled: long frames. The machine lost power during this task and Spotlight was reindexing for the reruns. Both builds dropped frames that day: 51 long frames over the twelve runs for the old build, 48 for the new, where the spike recorded none. The new build's are bunched in `slow-pan` on 20 and 40 pages (27 of them), the first profile after the warmup. Rerun `native/README.md` step 3 on a quiet machine before trusting either number.
- Tests that pin each change: `demand.rs` (30 idle seconds draw no frame, an open caret draws two a second, a tail after input), `mesh_cache_tests.rs` (hits on pan and in-octave zoom, misses on change), `text_hold.rs` and `tests/scene_text_hold_gpu.rs` (reuse counts, and a panned frame pixel-equal to a fresh one), `batch_tests.rs`, `import_cache.rs`, `paint_lod.rs`, `target.rs`, `bench_run.rs`, `recorder.rs`, `work.rs`.
- Known costs left, largest first: `view` rebuilds freehand outlines and parses markdown every frame (0.6 to 0.9 ms; needs a cache inside `specular-scene`, which this task stayed out of). Screen-space text sized by the zoom, page titles and edge labels, is shaped and rasterised at every zoom step (0.5 + 1.4 ms on `mixed`). Edges are screen-space paths, so they are tessellated every frame (0.33 ms for 200). A zoom across a power of two re-tessellates every visible path in one frame. Any input event owes a frame, including a pointer move over nothing. The mesh copy and upload are 0.2 ms each for 48,000 triangles.
- For a human at a Mac: pan slowly over stickies and Documents and watch whether text shimmers against its note (the half-pixel placement). Pause mid-gesture and resume: the first frames should not stutter. Type in a sticky and watch the caret blink while everything else is still.
- Rebased onto the `Runtime` refactor and the GPUI shell. The demand lives in `Runtime` (`app/frames.rs`: `turn`, `frame_wanted`, `next_turn`, `input`, `draw`), and the winit shell's `app/turn.rs` sleeps on it. `specular-shell` still calls `turn` and `draw` every frame, which works and never rests: to get the idle saving there, draw only when `frame_wanted()` and call `input()` on pointer, wheel and key events. All the numbers above were taken before the rebase, on the winit shell; after it a pan and an idle run were repeated as a check.
- Gate: fmt, clippy with and without `specular-app/cef`, `cargo test --workspace`.

### P3 and the renderer half of P4: page chrome, the sidebar, popup leftovers, the context menu and the layout cache. See `git log -- native/crates/specular-interact/src/panel/builtin/cache.rs`

- Models: `Control::Field` and `Control::Choices` (`panel/field.rs`, `model.rs`), `PopupAnchor::Point`, `context_menu` and `MenuTarget` (`panel/context.rs`), `ToolbarModel.sidebar`, and `SidebarModel` with section heads, folds, row glyphs, trailing text, each canvas's rename `Field` and delete. Popups end in `panel/popup/actions.rs` (arrange, annotate, focus), and a mixed selection has one.
- Actions: `PageNavigate`, `Reveal`, `RevealComment`, `Sidebar(SidebarAction)`, `Arrange(ArrangeMode)`, `FocusSelection`, `CanvasAction::BeginRename`; `Property::{ViewportWidth, ViewportHeight, Label}`; `ToolDefaultPatch::{PagePreset, PageCustom}` (not stored, as in Electron). New modules: `viewport.rs`, `reveal.rs`, `arrange.rs`, `sidebar/`, `edit/field.rs`.
- Page popup: back, forward, reload or stop, the address (a URL or a search, as `resolveAddressInput`), the size list with W and H fields, and the page tool's preset list, which placement uses.
- Built-in renderer: `Surface::{Sidebar, SidebarList}`, a clipped scrolling list, rename in place, the context menu in the dropdown slot, formatting buttons that show their state and keep the edit, `cache.rs`. A pointer move builds the layout 0 times warm (it was 3 to 5, plus 1 for the draw); a press or release 1. `tests/panel_builtin_cache.rs` pins the counts and compares against an uncached build after each kind of change.
- GPUI shell: it compiled against the new models after three arms (`Field` drawn as text, `Choices` as sections, `Point`). It does not draw the sidebar toggle, folds, the context menu or the new row fields yet. `canvas_only` keeps the built-in sidebar off there.
- Scripts: `sidebar on|off`, `right-click x y`, `act arrange-row|-column|-grid`, `act focus-selection`, `act zoom-to-fit`; a field is `control <id>`, `key cmd+a`, `type ..`, `key enter`. Scenarios `k-page-chrome`, `l-sidebar`, `m-context-menu-and-arrange`, `n-chrome-session`, 16 rows in `check.py`. `d-text-edge-cases` had one double click made a click: the wider popup sat under its second press.
- Skipped as deferred: hand, inspect and theme buttons, sync and unsync, the focus session, repo binding, auto-layout, the right panel. Skipped for the compositor: gradients and shadows in the tool glyphs. Tool buttons stay at y 8 (Electron's 7.5 snaps to 8 at 1x).
- Off against Electron: no spin on reload while loading (a stop button instead); the address shows the old URL until the page reports the new one; W and H fields are an addition; whole-note bold, strike and bullets on a selected note outside an edit; the camera move is not tweened; no drag-reorder, entity-row menus or favicons in the sidebar; sidebar visibility is not saved; a Document's name is not renamed in its popup; menu rows are 24 px where Radix has 28; a popup clamped under the toolbar covers the page's title.
- Known gap in the cache: a read in the middle of one event, after that event changed what the sidebar rows show, can see the rows from before it. They are rebuilt when the event ends. At 3000 entities the sidebar model costs about 6.6 ms a build in debug; it is built once an event now, not once a read.
- Built in a detached worktree in four steps by subagents and rebased onto the GPUI shell and performance commits; `panel/builtin.rs` and `update.rs` needed merging by hand. Gate after the rebase: fmt, clippy, `cargo test --workspace` (1904), `fixtures/scenarios/run.sh` (all rows ok). Nothing was run in a window.
- Needs a human at a Mac: scroll the sidebar with a trackpad, rename a canvas with an IME, type a URL into a real page's popup with `--source cef`, right click every kind, and check the formatting buttons keep a selection made by dragging.

### AUDIT, part 1: what the tree measures, before any cut

Measured at `61630e20` with scripts over `native/crates` (inline `#[cfg(test)]` modules counted as test lines). `cargo-udeps` and `cargo-machete` are not installed, so unused items come from a name scan.

| crate | source | test lines | tests | pub items | pub items no other crate names |
|---|---|---|---|---|---|
| specular-interact | 20,383 | 15,775 | 864 | 359 | 92 |
| specular-app | 8,325 | 3,003 | 199 | 49 | n/a |
| specular-compositor | 5,956 | 4,164 | 218 | 34 | 9 |
| specular-scene | 5,871 | 2,501 | 167 | 89 | 19 |
| specular-shell | 3,607 | 170 | 11 | 0 | n/a |
| specular-cef | 2,860 | 579 | 46 | 56 | 45 |
| specular-bench | 2,649 | 1,255 | 113 | 90 | 42 |
| specular-api | 2,607 | 1,670 | 51 | 30 | 2 |
| specular-doc | 2,543 | 1,407 | 60 | 105 | 2 |
| specular-testkit | 1,926 | 146 | 10 | 103 | n/a |
| specular-core | 1,384 | 453 | 39 | 71 | 8 |
| total | 58,111 | 31,123 | 1,778 | 986 | 219 |

- Largest modules, source lines: interact `panel` 3,998, interact `edit` 3,635, compositor `scene_pass` 4,009, app `app` 2,920, scene `view` 2,612, scene `panel` 1,593, app `headless` 1,136, interact `comment` 1,120.
- Files over 400 source lines: 7. `cef/source.rs` 497, `interact/edit.rs` 489, `bench/compare.rs` 485, `compositor/scene_pass/text.rs` 439, `compositor/compositor.rs` 423, `cef/client.rs` 418, `shell/view/controls.rs` 406.
- Functions over 80 lines: 18 of 2,533. `Runtime::render` 132, `Runtime::capture` 122, `scene_pass::build` 110, `update` 109, `freehand::outline_points` 108, `run_action` 106.
- `update()` is 109 lines in a 366-line file and does no I/O: no `std::fs`, `net`, `process`, `env`, `thread` or clock read anywhere in doc, interact, scene or api.
- `match` blocks naming a variant: `Kind` 40 (27 files), `Tool` 10, `Hit` 9, `Action` 6, `Effect` 4, `Event` 3, `Gesture` 1. Plus 11 `matches!` and 17 `if let` on those enums.
- Wildcard arms over those enums: 6. `page_state.rs:132` and `menu.rs:244` (`Action`), `api/annotations.rs:273` and `:281` (`Kind`), `testkit/comments.rs:50` and `:69` (`Effect`). All six pick one or two variants out; none hides a missing arm in a full match.
- Rect types: six. `doc::Rect` (f64, the file's numbers), and five f32 ones: `core::CanvasRect` (2 callers), `core::CssRect`, `scene::Rect`, `interact::ScreenRect`, `interact::PanelRect`. `interact/geometry.rs` re-implements `union`, `intersection` and `contains` as free functions over `doc::Rect`.
- Two shells: the winit one is 1,781 lines in `specular-app` (9 files that name `winit`), the GPUI Kit one 3,607 in `specular-shell`. Two panel renderers: the built-in one is 1,995 lines in `interact/panel/builtin` and 1,612 in `scene/panel`, and the Kit draws the same models in about 1,300 lines of `shell/view`.
- Dead, no reference outside the definition: `Camera::pan_by`, `Space::canvas_camera`, `Tool::is_one_shot`, `Color::TRANSPARENT`. Referenced only by tests: `MoveDrag::is_dragging`, `property::text_style`, and testkit's `panel_snapshot`, `layout_snapshot`, `toolbar_snapshot`, `press_button` once the prune ran.
- Dependency direction against `doc <- interact <- scene <- render/ui <- shell`: `specular-compositor` depends on `specular-interact` (it implements `TextMeasure`) and on `specular-doc` (`EntityId`, `TextAlign`), though the plan says the renderer knows nothing about entities. `specular-app` depends on `specular-testkit` outside tests (the headless runner is built on `TestApp`). `specular-shell` depends on `specular-app`. There is no `specular-ui`: panel models and layout live in interact, panel drawing in scene. `specular-core` is not in the plan and sits under interact.
- Caches: `Scene` holds none. `scene/panel/icons.rs:219` has a process-wide parsed-icon map, and `App` holds `StackCache` (`interact/edit/stack.rs`, a `Mutex` over text layouts), which is a cache inside the state `update` owns.
- Tests: 963 in `tests/` directories, 815 inline, 167 snapshot assertions over 99 `.snap` files. Of the 704 the prune removed, by the pruning agents' own classing (it overlaps, so it sums past 704): about 250 restated another test, 215 were folded into a table or a neighbouring test, 95 tested a trivial helper, 85 restated a scenario script, 80 pinned an implementation detail, 13 restated the type system.

### AUDIT, part 2: the test prune

1,778 tests to 1,074 (31,123 test lines to 24,737). Six agents pruned one crate group each against `tests/README.md`'s bar and the brief's keep list; the scenario scripts, `check.py`, the fixtures and every `.canvas` round-trip and byte test are untouched. `native/CLAUDE.md` now says one behavior test per feature, a snapshot only for a new draw rule, no unit tests on trivial helpers.

| crate | before | after |
|---|---|---|
| specular-interact | 864 | 482 |
| specular-compositor | 218 | 168 |
| specular-app | 199 | 122 |
| specular-scene | 167 | 101 (86 `.snap` files to 58) |
| specular-bench | 113 | 60 |
| specular-doc | 60 | 35 |
| specular-api | 51 | 46 |
| specular-cef | 46 | 32 |
| specular-core | 39 | 15 |
| specular-shell, specular-testkit | 21 | 13 |

What each deleted group is still covered by:

- Escape at each stage of each gesture (21): scenario `h`, and `escape_steps_out_one_level_at_a_time`.
- Undo, redo and "no step recorded" restated per verb (14): the `assert_undo_returns_to_start` that ends each remaining test, and scenarios `a` and `b`.
- One gesture repeated per kind, handle, side, modifier or tool (about 95 in select, hit_test, resizes, moves, gestures, routing, groups): the table or neighbour in the same file (`every_kind_gets_handles_when_selected`, `each_handle_moves_its_own_edges...`, `shift_flips_whether_a_resize_keeps_the_ratio`), and scenario `g` for the zoom limits.
- Comment drafts, pills, focus and delete variants (27): scenario `i`, `a_pill_sits_by_its_anchor_and_stays_inside_its_page`, `backspace_and_delete_remove...`.
- Edge create, re-route and label variants (11) and the edge-drag controller's states (9): `tests/edges.rs` and `tests/edge_labels.rs`.
- Text editor keys, IME and session variants (13) and Document edits (10): scenarios `c` and `d`, `a_document_takes_every_format...`, `ending_the_edit_writes...`.
- Property, popup and built-in panel variants and per-kind layout snapshots (39): scenario `j`, `a_color_reaches_every_kind...`, `a_sticky_has_size_font_color...`, `the_toolbar_and_a_sticky_popup`.
- Canvas, sidebar, menu and page-notice variants (22): `switching_closes_the_pages_left...`, `every_shortcut_is_the_binding_tables_chord...`, `each_notice_lands...`.
- Per-command apply and undo, refusals and history in `specular-doc` (21): one round-trip test over every command, one refusal test, and the fixture undo tests.
- Camera, page-spec, resize, stack-order, format and small-helper unit tests in core and interact `src/` (about 95): folded into one test per rule, with `tests/resizes.rs` and `tests/stack_order.rs` covering the behavior end to end.
- Scene snapshots of a rule already drawn in another state or kind (26) and helper tests in scene (40): the kept snapshot of the rule in the same file, and `documents.rs` for markdown.
- Compositor helper tests (50): one test per rule. Every performance pin and all but two GPU readbacks are kept.
- CLI flags (36 to 7 tables), script steps (9 to 2), paint tiers, key translation, titles in `specular-app` (77): the tables in the same files.
- Bench statistics, compare, profile, memory and report tests (53) and cef translate, config and coords checks (14): one or two per rule.

Known weak spots. In `specular-doc`, `specular-core` and interact `src/`, about 100 tests were folded by pasting their bodies into one test as blocks, not by writing a table: every assertion survives, so the count fell more than the code did. The scene and compositor group stopped at 30% because the pins and readbacks are most of what is left. No test was mutation-checked. The scenario scripts assert on saved canvases only, so a deleted test that a scenario "covers" is covered for the document it leaves, not for cursors or mid-gesture state.

### AUDIT, parts 3 and 4: the chrome branch landed, structural cuts, and the list of what is left

- `claude/p3p4-chrome` is merged in under the cuts (a merge, not a rebase: the rebase was refused by the session's permission rules). Conflicts were in test files both sides touched; the branch's versions won and were pruned again. Its 177 tests are 98: the context menu per target, arrange and the no-popup cases are one table each. The suite is 1,140 tests, from 1,778 plus the branch's.
- One f32 rect: `specular_core::{Point, Size, Rect}`, which is what `specular-scene` had. `CanvasRect` is gone, `CssRect` is an alias, and `specular_scene::Rect` is a re-export, so no caller changed.
- Deleted, with no caller: `Camera::pan_by`, `Space::canvas_camera`, `Tool::is_one_shot`, `Color::TRANSPARENT`, `PlaceDrag::placing`, `MoveDrag::is_dragging`, testkit's `menu_snapshot`.
- Wildcards: `page_state::navigate` takes a `PageNav` and matches it in full, and `run_action` has an arm per page action. The two `Kind` picks in `api/annotations.rs` are `let ... else`. Left: `menu.rs` `tool_action` and two in testkit, each picking one `Action` or `Effect` variant out of a list.
- `Chord::text` replaces a copy in the built-in dropdown and one in the Kit's controls. The built-in one now writes `Space` and the Home, End and page keys as the Kit does.
- Split: `interact/edit.rs` (503 to 342, with `edit/fit.rs` and `edit/read.rs`), `compositor/compositor.rs` (423 to 293, `compositor/ingest.rs`), `scene_pass/text.rs` (439 to 319, `text/prepare.rs`), `bench/compare.rs` (485 to 320, `compare/markdown.rs`). Not split: the two cef files (a branch is open there) and four files at 402 to 413 lines with no clean seam.
- The larger cuts not made are in the plan under "Cleanup tasks", ranked, with lines and risks. The first two are the built-in panel renderer (about 7,000 lines with its tests) and the winit window (about 1,700).
- Next agent: rebase onto this. Tests you add follow the new rule in `native/CLAUDE.md` ("Tests"). `specular_core::CanvasRect` is `specular_core::Rect`; `page_state::navigate` takes a `PageNav`.
- Gate: fmt, clippy for the workspace, `cargo test --workspace` (1,140 pass), `fixtures/scenarios/run.sh` (every check ok). Clippy with `specular-app/cef` also passes.
