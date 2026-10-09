# Rust native rebuild: run log

Appended by each agent as it finishes a task from
[`rust-native-rebuild.md`](./rust-native-rebuild.md). Newest entry last.

## Decisions

Choices made during the run that the plan did not settle, grouped by area. The task that made each is in brackets, and its entry below has the detail. This section was condensed on 2026-10-08 from about 290 one-line decisions. A decision that a later one reversed is left out of its area and listed under "Reversed" at the end, so the areas describe the code as it is. The full original list is in git history before that date.

### How the run worked

- No PRs or pushes during the run. One commit a task on the current branch. [Orchestrator]
- Task F1, the up-front crate split, was folded into F4 and F5. New crates were made beside the spike's and the old ones renamed or absorbed as their replacement landed. [Orchestrator]
- `native/bakeoff/` builds its dev profile at `opt-level = 3` so timings mean something without `--release`. [M1]
- Tests on the testkit live under a crate's `tests/`. A `src/` unit test would see two copies of its own crate's types. `specular-interact`'s are one binary, `tests/it/`, and a new file needs its `mod` line in `main.rs`. [F6, CLEANUP-A]

### Document and `.canvas`

- yrs is dropped. The document is typed structs with inverse-command undo ([ADR 0041](../adr/0041-typed-document-inverse-command-undo.md)). [Orchestrator]
- Ids are the `.canvas` id strings, and entity and edge ids share one namespace. The stack order is `Vec<ItemId>`, because edges interleave with entities in `specular.entityOrder` (ADR 0014). [F2]
- `specular-doc` has its own `Rect` and `Point` in `f64`. An `f32` rect changed numbers on a load and save. [F2]
- Commands are primitive and never cascade. References may dangle. Delete builds a `Command::Batch` from `Document::children` and `Document::edges_touching`. [F2]
- `History` is separate from `Document`. `Document::apply` alone is not an undo step, which is how loading and a drag's frames are done. [F2]
- `History<S>` carries the caller's state from each side of a step, and the app's `S` is the selection. Undo restores the selection from before the step and redo the one after. Select what a step made after `document_step`, never before. [Polish]
- `label` sits on `Entity`, not in each kind. [F2]
- The writer is canonical, not byte-preserving. It writes what the Electron writer would: `entityOrder` whenever the stack is non-empty, `annotations` only when there are some, floats rounded to a hundredth except under `zoom`, keys in Electron's order (`preserve_order`). [F3, F6]
- A node, edge or annotation that cannot be typed is kept as raw JSON in `Document::extra` and written back after the typed items. A leftover such as `"syncId": null` is written in its typed field's slot. A load fails only on invalid JSON, a non-object top level, or `nodes`, `edges` or `annotations` not being an array. [F3, S1]
- Group `pageIds` and `entityIds` are dropped on load and not regenerated. Membership is each member's `parent`. Page `groupId` and group `groupColor` are still written. [F3]
- A save writes `App::document_to_save()`, not `App::document()`. A text measured at open goes back to the size it was read with unless the session changed its rect, so the first change to an Electron file does not rewrite every text. [QA]
- A Document's text is a transient `notes` map on `Document`, changed by `Command::SetNote` and never written to `.canvas` (ADR 0023 without a Y.Doc). [T4]
- A page annotation's `offsetX` and `offsetY` are fractions of the page, not pixels. [Visual check]

### The loop: events, effects and time

- `Event` and `Effect` name a page by its `EntityId`. The shell keeps the table to the backend `PageId`. [F4]
- Time comes in as `Event::Tick { unix_ms }` once a loop turn, so there is no timer effect. New ids come from a seeded sequence in `Session`. [F4]
- `Action` is the one enum for key bindings, menus, panels and API act routes. [F4]
- A drag writes rects into the document as it goes, through `Document::apply`. The release puts the start rects back and records one `History` step. Escape just puts them back. A gesture that creates an entity works the same way, and `App::creating()` names it. `live.rs` is the shared plumbing. [F4, Tools, S3]
- A page's viewport is not stored. It is the rect's rounded size, held at the starting size while a handle is dragged. Undo, redo and opening a document diff the pages and return create, close and viewport effects. [F4]
- `update` compares `History::revision()` before and after an event and returns `Effect::Save` when it moved, so no command site has to remember to. [S9]
- After any event that moved the camera or the document, or ended a drag, `update` runs the pointer again where it stands (`pointer::settle`). A drag follows a wheel or a pinch, and the hover is found again after an undo. [QA]
- The cursor is recomputed after every event but a tick and returned as `Effect::SetCursor` only when it changes. [S5]
- A question to a page is an `Effect` answered by an `Event` that repeats the question, so nothing waits in the session. [C2]
- `update` holds no cache. `view` takes a `ViewCache` its caller owns, and the text-layout memo is in the shell's `GlyphMeasure`. [CLEANUP-A]

### Scene and rendering

- Canvas items are SDF shapes, glyphon 0.12 (cosmic-text 0.19) and lyon in the compositor's pass. Vello is turned down. ADR 0039, Proposed. [M1]
- Every scene item is in canvas space or screen space. Chrome that hugs an entity at a fixed pixel size is a screen-space item that `view` projects. Colours are 8-bit sRGB with straight alpha. Clip and opacity are per item. [F5a]
- `Scene` names a page by `EntityId` and an image by `ImageId(u64)`. `render_scene` takes a closure from entity to backend page. [F5a]
- A text run has an origin plus an optional wrap width and box height. The renderer shapes and measures, and `view` never measures text. [F5a, F5b]
- An item joins the earliest batch of its kind at or after the last batch it overlaps, and a page is a batch of its own. Border and title chrome beside 40 pages is one shape batch and one text batch. The overlap test goes through a grid. [F5a, Performance]
- Dashed borders and edges are paths with a `Dash`. The SDF layer draws solid rects and ellipses. A stroke thinner than a device pixel is drawn one pixel wide and faded by the same ratio. [F5a]
- While the camera is zooming, canvas glyphs keep their raster size until the zoom has moved 0.75x to 1.25x from it, and the pass viewport stretches them. The shell renders one frame with `zooming: false` when the gesture ends. [F5a]
- `view` culls entities outside the viewport plus 64 px. `view_without_chrome` is what `--chrome off` draws. `--chrome off` only stops the drawing, and gestures and keys still act. [F5b, F4]
- `specular-scene` depends on `specular-interact`. Edges are drawn from `App::edge_curve`, the curve hit-testing uses. The compositor no longer depends on interact or doc outside its tests: the text types moved to `specular_core::text`, and the id on a page or column draw is `specular_scene::OwnerId`. [F5b, CLEANUP-A]
- Colours are the light theme only. The vivid inks are constants in `view/palette.rs`. Blue is stored as `"7"`, read as `Color::Custom("7")`, and the palette maps it. [F5b]
- Drawings are outlined in canvas space, so a stroke has the same shape at every zoom. Electron outlines them in screen space. [F5b]
- A page keeps the 8-unit corner radius and gets a title above it as the title-bar stand-in. Electron draws neither on the canvas. [F5b]
- The window surface and the snapshot target are not sRGB formats, so colours blend encoded, as in a browser. Paths and polygons fill with the non-zero rule. [Visual check]
- `Draw::Shadow` is a rounded rect's blurred shadow drawn as one more SDF instance. `Item::blend` is `Normal` or `Multiply`, and only paths and polygons honour `Multiply`. The highlighter is multiplied in at 70%, so text under it keeps its colour. [Polish]
- Group and page titles keep 11 px down to zoom 0.5, shrink below it, and end in an ellipsis at the entity's width. Selected text is `#b3d7ff`. Emoji are set to what CoreText measures. [Polish]
- A markdown Document is `Draw::Column`: rows of text cells the renderer stacks. `TextRun::spans` sets weight, italic, family, colour, underline and strike on byte ranges, with no per-span size. Underlines, strikes, quote bars and table lines go through glyphon's custom-glyph path. [T3]
- Page and region screenshots go through the compositor, not CEF. A page at half texture scale comes out soft. [A3]

### Selection, hit-testing, move and resize

- Hit-testing runs in screen space with Electron's sizes. Pages and other items share one stack order, so a page in front of a note covers it, where Electron always puts notes above pages. [S1]
- `hit::body_at` picks the innermost group under the point, then the frontmost. Group tints are drawn before every entity. [Groups/edges/S7]
- The per-kind rules are `min_size`, `aspect_mode` and `has_anchors` in `caps.rs`. The page minimum is Electron's 320x200. [S1]
- The entered page of ADR 0022 is `Focus::Page`. It stays entered only while it is the whole selection. Entering happens when the click is released, because a press on the selected page may become a drag. [S2, S3]
- Escape is staged. It backs out of a drag, an open draft, an armed tool, an entered page or an entered group one level at a time and keeps the selection. With none of those it deselects. It matches whatever modifiers are held. [S2, Tools, C2, Groups/edges/S7]
- A marquee changes the selection on release. `Session::hover` is the entity under the pointer, and the hover outline is not drawn during a gesture. [S2, Visual check]
- A plain drag on a body moves the selection, and Option makes it a copy. A move snaps the pressed entity's top-left to the 20-unit grid and moves every other operand by the same delta. Electron snaps each on its own. A pressed drawing does not snap. [S3]
- Resize is computed from the start rect and the pointer each frame, not from accumulated deltas. Option does nothing. Shift follows the kind's `AspectMode`. A resize floor is the kind's minimum or the size the entity started at, whichever is less. [S4, T1 leftovers]
- A text resize measures the height on every frame of the drag. A text has a width floor and no height floor. [T1 scene]
- Resizing a page writes no `pageSizeMode` or device metadata. [S4]
- Copies keep their group unless the group is copied too, lose a page anchor unless that page is copied, and take an edge only when both ends are copied. Duplicate places the copy 80 units right, else below, else at the first free grid spot, and pans the camera by the least that shows it. [S5, Polish]
- An Option-drag's ghost is the entity drawn again at half opacity where the copy will land. [Polish]
- An unselected drawing is hit within 6 px of its ink, not anywhere in its box. Where an edge crosses an entity the press is shared: a drag moves the entity and a click selects the edge. [Polish]
- Select all takes every entity with no parent. Edges are not selected. [Shell batch]
- Guides do not pull (ADR 0012). The grid is the only magnet, the tolerance is 0.5 canvas units at every zoom, and guides are derived each frame, not stored. A selection resized by its shared bounds shows none. [ARRANGE]

### Tools, keys and creation

- A new page is Electron's `P` then click: preset 0 (375x667), `about:blank`, with Electron's device metadata. [Tools]
- Each stroke is its own drawing entity and its own undo step. Changing the brush moves the stroke width to the nearest width that brush is offered in. [Tools, QA]
- Tool defaults are `App::tool_defaults()`. A variant key (R, O, Shift+R, M, Shift+M) is `Action::SetToolVariant`, which arms the tool and writes the default. A tool's key pressed again does nothing. [Tools]
- A chord's `cmd` is Command or Control. `BINDINGS` is one const table, first matching row wins, and `Context::overlaps` lets two rows share a key. Canvas bindings go to the page while a page is entered. Cmd+1 is `Context::Always`. [Tools, F4, Shell batch, CEF]
- `I` is the inspect tool's only key. Electron's `H` is the hand tool, which this app does not have. [INSPECT-LOOP]

### Text editing and input methods

- The editor is the app's own, in `specular-interact/src/edit/`. `Session::editing` is a `TextEdit`. The working text lives only there until the edit ends, and `App::editing_text(id)` is what to draw. [T1]
- A text entity's fitted rect is the exception: it is written into the document while typing, so the outline and hit-testing follow. Ending the edit makes one step of text plus rect. [T1]
- A text or sticky placed by its tool has no undo step until the edit ends. Ended with text, it is one step that undoes to nothing. Ended empty (whitespace only), it is taken out with no step. An emptied shape label keeps its shape. [T1]
- Escape, a press elsewhere, a tool change, a verb and a selection change all keep the edit. Only `DocumentOpened` throws it away. [T1]
- `TextMeasure` is one method returning a `TextLayout` of plain data. The compositor's `GlyphMeasure` shares one `FontSystem` with the text renderer, and `TextRun::set` is the only place a `TextSpec` becomes a run. `is_exact()` says the layouts are the renderer's, and only then are a loaded file's texts refitted. The testkit's `FixedAdvance` returns false. [T1, T1 scene]
- No caret affinity. End on a wrapped line stops one grapheme short. The keys are the macOS set. CodeMirror's Emacs keys are not in. [T1]
- The editor's own undo joins consecutive typing into one step and consecutive deletes into another. `Action::Undo` goes to it while an edit is open. [T1]
- Bullet lists apply to text and stickies, not shape labels. Formatting is `Action::Format` under `Context::Editing`. Cmd+B, I, E, Shift+X and Shift+8 are Electron's, and the numbered, task and heading chords are new. [T1, T5]
- The formatting shortcut with the caret just before a closing marker steps past it, where Electron nests a second pair. [QA]
- While an input method is composing, every key but Escape is ignored. A composition is one editor undo step. Escape while composing ends the edit and keeps the marked text, as Electron's editors do. A real input method takes Escape first, which only a person can check. [T2, Polish]
- `App::handles()` is `None` while text is edited. The caret is a screen-space rect in the text's colour, blinking 500 ms on and off. An empty plain text draws "Add text" at 40% alpha. [T1 scene]
- A property set during a text edit does not end the edit. [P1/P2]

### Documents

- A Document is edited as its source, one row a source line, not in the read view's layout. The syntax styler is by hand in `edit/source.rs`. The text shifts a little when an edit opens. [T4]
- The renderer reports each column's height and the shell sends `Event::NoteHeights`, which `update` uses to stop a scroll at the end. [T4]
- Saving is debounced in `update`, 350 ms after the last change, as `Effect::WriteNote`. Ending the edit, opening another canvas and quitting write at once. [T4]
- The shell refuses a `WriteNote` when the file holds a text it never read or wrote. An outside change while editing keeps both texts: ours takes the file, theirs goes to `<name> (conflict <id>).md` with a Document beside the first. [T4]
- A finished edit that changed the text is one history step. An outside change while not editing resyncs the held text with no step. [T4]
- `add-document` is two-phase because only the shell knows what names are taken: `Effect::CreateNote`, then `Event::NoteCreated`. [T4]
- The note thread polls stamps every 500 ms and reports a file only when its text reads differently. Table columns are equal widths, a code block has no background, an image is the text `[image: alt]`. [T3]

### Groups, edges, stack order and auto-layout

- Stack-order verbs have no Notes and Pages sections, because the native stack is one. A selected group moves as its whole run. A new group goes just in front of its frontmost member's run. [Groups/edges/S7]
- A freeform group's rect follows its members (union plus 24) in the same undo step, through `group_fit::then_fit`, which every step goes through. Electron only refits auto-layout groups. A hand-sized group tightens the first time a member changes. [Groups/edges/S7]
- The group drop target is tested against the group rects captured when the drag began. [Groups/edges/S7]
- `Session::entered_group` holds while the selection stays inside the group. Electron has no such state. [Groups/edges/S7]
- Delete takes a group with its descendants and their edges (ADR 0034's operands), in the window and through the API. Electron removes the container or member pages only. [Groups/edges/S7, A1]
- A group title and an edge label are edited with the text editor, one line each. A double click on an edge opens its label in place, where Electron edits it only in a popup. [Groups/edges/S7]
- An edge drag changes nothing in the document until the release. A drop over another item's body connects to the side facing the fixed end. Cancel, Escape and a drop on nothing during a re-route remove the edge as one step, by design. [Groups/edges/S7]
- Re-anchoring on a move's release covers the selected entities only. Command or Control at the release suppresses it and the group drop. [Groups/edges/S7]
- A managed group's layout is not a command of its own. `then_fit` lays it out before refitting, so no caller can forget and the document never holds an unresolved rect. A member's rect is an output. [ARRANGE]
- The layout sequence is the stack order of the group's members (ADR 0015 D2). There is no child-order field, no padding field and no managed grid. [ARRANGE]
- A body drag from a member of a managed group moves the whole group. Only the dot reorders. A reorder or gap drag writes the line into the document each frame. [ARRANGE]
- The group popup's row and column toggles and gap stepper are new. Electron's popup has none. [ARRANGE]
- Arrange is one-shot, one undo step. Focus is the camera framing only. [P3/P4]

### Clipboard, drop, assets and images

- Copied entities go on the clipboard as text, `specular:canvas:` plus a small `.canvas`. It is not Electron's payload, so the two apps do not paste each other's entities. [Shell batch]
- `Paste::of` decides in Electron's order: copied entities, an image, a one-line URL (a page), other text (a sticky). Not ported: copied file references, SVG, HTML and JSON text, long text becoming a Document. [Shell batch]
- `update` names a pasted or dropped file `assets/<entity id>.<ext>`. A dropped file already inside the space folder is shown by its relative path with no copy. Only images and `.md` are taken. One drop is one undo step. [Shell batch]
- Images are a table in `Session` keyed by the `file` string. Which files are images is Electron's extension list. What can be decoded is the shell's business: svg, bmp and ico fail and stay cards. An image nothing shows is kept until another document opens. [K6]
- The decode thread premultiplies, builds mips and scales down an image over the texture limit. `contain` draws nothing in the letterbox bars. [K6]
- A `file` that starts with `__SPECULAR_SPACE__/` is read from the canvas's own folder. [Polish]

### Persistence, spaces and canvases

- The file watch is a `stat` every 500 ms on the loop, not an OS watcher. Our own write compares equal to what we hold and is ignored. Nothing is saved or reloaded while a gesture is in flight. [S9]
- The camera is written only when a document change saves. Panning alone does not write the file. A pending save is flushed on exit. [S9]
- Preferences are the native app's own `preferences.json` in `~/Library/Application Support/Specular Native` (`SPECULAR_NATIVE_CONFIG_DIR` overrides), not Electron's. A save keeps the file's other keys. [Shell batch]
- `App` owns one `Space`, every canvas whole ([ADR 0043](../adr/0043-per-canvas-state-in-a-space.md)). Every canvas is read at open. A background canvas has no page hosts, and a switch closes and recreates them. Nothing was measured. [P5]
- The space index is Electron's `.specular/workspace-meta.json`, with unknown keys kept and Electron's file names. A `.canvas` the index does not list is adopted with an id made from its file name. [P5]
- Canvas names are unique once trimmed, everywhere. A duplicated canvas keeps its entity ids. Deleting a canvas is not undoable, and its file goes to the system trash. [P5]
- A `--tab` write to a background canvas runs as one undo step in that canvas's own history, may add pages, and goes through while the user drags. [P5]
- A `--bench`, `--snapshot`, `--script`, `--pages` or `--annotations` run opens no space and writes nothing. [P5, S9]
- With no path and no `--space`, `specular-app` (winit) opens the scratch space, and `specular` (GPUI) opens the folder chosen in it or asks. The user's real space needs `--space user`, a path, or a choice made in the first-run view, because autosave writes into it. Electron's `preferences.json` is read for `spacePath` and never written. [Scratch space, APP-BUNDLE]
- A folder from settings that is not there is not made or opened. The app asks. [P5, APP-BUNDLE]

### Pages, CEF and sync sets

- A page's rect is its screen. The device frame (`showDeviceFrame`) is drawn outside it, from `Page::shell()` and the table in `specular-doc/src/presets.rs`. The `.canvas` file places a framed page by its shell's corner, as Electron does, so the reader adds the bezel's left and top and the writer takes them off. A new page and a property that changes the bezel keep the shell's corner where it was. Hit-testing, the selection outline, zoom to fit, arrange and the API's rects still go by the screen, not the shell. [DEVICE-FRAMES]
- A page says things about itself as `PageNotice`s into `Session.pages`. Only the address is saved, written to the page entity with no undo step. [CEF]
- A changed page URL is `Effect::Navigate`, never a close and a create. Cmd+[ and Cmd+] walk the history only of an entered page, because on the canvas they restack. [CEF]
- A page is asked about its DOM over CEF's in-process devtools channel, one `Runtime.evaluate` a question. The synthetic source answers the same way from its grid. [CEF]
- Each process gets its own CEF root cache folder in the temp directory. On a shared root a second launch crashed the first. [CEF]
- A headless run hosts real pages when `--source cef` is named, with CEF pumped by the caller. [CEF]
- A page-bound region's `docRect` is in document pixels and drawn less the live scroll. An anchored entity follows from the scroll stamped at placement, and with no stamp it stays pinned. Out of the page it is hidden, clipped at the edge with no fade. [Scroll]
- The page profile is on disk only for a launch with no path argument, and only for the process that holds its lock file. [APP-BUNDLE]
- A synced page has no mark on the canvas. One `Action::ToggleSync` serves the chain button and the unsync button. A set of one reads as unsynced. [SYNC-SETS]
- Navigation is followed from any page of a set, with Electron's 1500 ms quiet window. Scroll, hover and click are followed only from the entered page. Followers jump, where Electron eases. [SYNC-SETS]
- Capture is a long poll over the devtools channel, not `Runtime.addBinding`, so it keeps no state a CLI client could reset. Candidates are scored in Rust (`specular_core::locator`). A replayed click gets 1000 ms to navigate a peer before navigation sync does. [SYNC-SETS]

### Comments

- A comment region over a page is stored as Electron stores it: a `docRect` in the page's CSS pixels plus a `pageAnchor`. [F4]
- A draft is an `Annotation` in the session, not the document. Keeping it is one `InsertAnnotation` step. Annotations are undoable, as in Electron. [C2]
- A comment and the selection are never both the target. Delete removes whichever there is. The comment tool stays armed after a comment is kept. [C2]
- A click that finds no element on a page makes a canvas point, as Electron's does. [C2]
- A canvas-point comment draws its count pill on the point. Electron draws nothing there. The badge number is the message count of its group. A region is hit within 6 px of its edge, so what is under it stays reachable. [C3]
- `App::comment_marks()` is the one set the hit-test and the scene share. [C3]
- With a chat panel, a comment draft is a passive marker and the panel's field is its composer. The winit shell keeps a small composer on the canvas. [C2, RIGHT-PANEL]
- A region comment's picture is drawn when `GET /annotations/<id>` is asked, not stored in the `.canvas`. [A3]

### HTTP API and CDP

- The reply slot on `Event::Api` is a ticket number, answered by `Effect::ApiReply`, so `Event` stays plain data. [A1]
- No second CLI. `src/main/cli.ts` runs against the Rust app unchanged. [A2]
- An act route selects what it acts on and leaves it selected. A patch does not touch the selection. [A1]
- A patch item is turned into a `.canvas` node and read with the file reader, so the API and a saved file cannot disagree. A value the reader cannot type is a 400 naming the field. An edge to an entity the canvas does not hold is a 400. Electron stores both. [A1]
- A text the API adds or changes is resized to fit in the same undo step. A sticky only grows. [A1]
- A write while a drag is in flight is refused with 409. [A1]
- The server is `tiny_http` on one thread with no async runtime. A benchmark run and a headless run start none. [A1]
- The app takes `SPECULAR_PORT` or 29979 and `~/.specular/specular-mcp.json` unless a Specular already answers there. Then it binds a free port and writes `specular-native-mcp.json`. Started first, it holds 29979 and Electron starts with no API. [A1]
- Undo and redo are `POST /history/undo` and `/history/redo`. Electron has neither. [A1]
- Each page has its own CDP websocket backed by that browser's CEF devtools channel. The proxy answers the browser-level `Target` domain itself and refuses `Page.close`, `Target.closeTarget`, `Target.createTarget` and `Browser.close`. The websocket server is hand-written on std sockets. [A3]
- `print-pdf` and `record` stay 501. The first needs an API answer that waits across turns. [A3]

### Panels, menus and the sidebar

- Panels are pure models with no pixels, colours or hover state ([ADR 0044](../adr/0044-ui-as-pure-models-with-replaceable-renderers.md)). [P1/P2]
- A property change is `Action::SetProperty`, one variant a property, applied to every selected item it means something for as one batch. Selection picks never write tool defaults (ADR 0008). [P1/P2]
- Built-in panels are off in a new `App` and turned on by `Event::BuiltinPanels(true)`. `view()` never draws them. Their layout and pointer state live in `specular-interact`, because `hit_test` needs the rects. [P1/P2]
- A popup clamps under the toolbar and to the viewport edges and never flips below its item. Toolbar buttons run `SetTool`, and Draw and Comment switch back to Select on a second click. [P1/P2]
- A typed value is `Control::Field` with a `FieldSubmit`. Enter and a press elsewhere commit, Escape restores. [P3/P4]
- The sidebar starts hidden and has no key, as in Electron. `App::covered_left()` is the width every fit, reveal and popup clamp reads. Canvas coordinates are not shifted. A row sends `Action::Reveal`, which pans only when the item is not wholly in view. [P3/P4]
- The context menu is a `PopupModel` of choices at a point. A right press selects its target first. [P3/P4]
- The built-in layout is cached behind a value stamp and forgotten at the end of every `update` except idle moves, wheels, pinches and ticks. [P3/P4]
- The Edit, Tools, View, Canvas, Arrange, Comment and Page menus are data, `menus(&App)`. An item's shortcut is the first `BINDINGS` row with its action, and it is enabled when that row's `Context` holds. [Shell batch, P5]
- Electron has no key for annotate-selection, resolve, the sidebar or the right panel, so native has none either. [C2, P3/P4, RIGHT-PANEL]

### The two shells

- The effect runners are `specular_app::Runtime<W: ShellWindow>`, shared by both shells. `specular-app` is a library with a thin `main`. [GPUI-SHELL]
- In the GPUI shell the canvas view fills the window and never moves. The slot GPUI leaves unpainted is the app's viewport, and it runs up under the Kit toolbar. Only the sidebar's width offsets it. [GPUI-SHELL]
- `PopupAnchor` is the dividing line. The Kit draws the toolbar and its popups, and `Event::BuiltinCanvasPopups` keeps the built-in popups beside a canvas item. [GPUI-SHELL]
- A model menu item's key is bound in GPUI under a context no element has, so macOS shows it and the key still goes to `update`. The shell's own keys (Cmd+Q, W, O, S, comma) are real GPUI bindings. [GPUI-SHELL]
- The `NSEvent` monitor only notes each key event, and the slot's key handler turns the note into a `KeyInput`. Composition comes through GPUI's input handler. [GPUI-SHELL]
- Sidebar rows and swatches are plain GPUI elements, because the Kit's row takes a string label and has no swatch row. [GPUI-SHELL]
- A folder dialog the app asks for is the shell's to show, not the runtime's, because a modal dialog inside a GPUI event would run window callbacks under it. `PickRepoFolder` still calls `rfd` from the runtime and has that risk. [APP-BUNDLE]
- In the winit shell, muda's menu replaces winit's default. Quit and Close are the shell's own items so the pending save is written. [Shell batch]
- The window title is the canvas name, plus ` — Edited` while a save is pending. [Shell batch]
- A headless run's clock starts at a fixed time and only `wait` moves it. It keeps the clipboard and new Documents in memory. [Visual check, QA]

### Agent chat, inspect and repos

- The thread model is its own pure crate, `specular-agent`. Threads and the active thread are per canvas. `index.json` adds `activeByCanvas`, which Electron ignores and drops when it writes. [RIGHT-PANEL]
- Close archives (`status: closed`) and the thread stays on disk. Electron has since replaced Close with Delete and reads `closed` as `open`, so a thread closed here shows up open there. [RIGHT-PANEL]
- No SDK. The runner spawns `claude -p --output-format stream-json`. A failed resume is retried fresh by `update`. [RIGHT-PANEL]
- The panel sits beside the canvas slot, so the viewport gets narrower and nothing is offset in canvas coordinates. [RIGHT-PANEL]
- The inspect tool asks the page once per whole CSS pixel of pointer movement, over any page, entered or not. Electron's page-locked mode is not ported. The picked node outlives the tool, because it is the chat's pill. [INSPECT-LOOP]
- The write target follows a DOM node's origin or a page-bound comment's, and nothing else. A selection always writes to the space folder. A repo is inferred only from a binding. [INSPECT-LOOP]
- A run on a bound origin starts in the repo, where Electron starts every run in the space folder. A thread whose turns move between the two folders cannot resume its session and falls back to the full prompt. [INSPECT-LOOP]
- Repo bindings are written to this app's own `repos.json`. Electron's is read only while this app has none, and never written. [APP-BUNDLE]
- The agent's canvas cursor chip is left out. There is no presence layer. [INSPECT-LOOP]

### Settings, first run and the bundle

- Settings > General > Change… opens another folder and leaves the old one. ADR 0033's "Move my canvases" is not built. [APP-BUNDLE]
- "Show and hide defaults" was read as what is shown at launch: the sidebar and the right panel. [APP-BUNDLE]
- `.canvas` is claimed as an Editor with rank Alternate under an imported type `org.jsoncanvas.canvas`. No URL scheme is registered. [APP-BUNDLE]

### Themes

- `App.theme` is a `ThemeState` (the choice, plus what the system last said), changed by `Action::SetTheme`, `Event::ThemeLoaded` (no save) and `Event::SystemAppearance`. `App::appearance()` is what `view` draws. Colours are one `specular_scene::Colors` per appearance (`colors/light.rs`, `colors/dark.rs`), read through `Frame.colors`; `Scene.appearance` carries it to the compositor, whose `DotGrid::themed` takes the canvas and dots from it. [THEMES]
- The page's `prefers-color-scheme` is `Effect::SetPageColorScheme { page, scheme }` with the page's own `colorScheme` (`None` follows the app). The runner resolves it, so a shell only needs `PageSource::set_color_scheme`, which the CEF source sends as `Emulation.setEmulatedMedia` (`applyPageColorScheme`) and the synthetic one ignores. The runner gives a new page its scheme as it makes it, so `CreatePage` effect lists are unchanged. The per-page setting was stored and shown before this but never applied. [THEMES]
- The toolbar button is Electron's theme button (system, light, dark, round again; the sun-moon, sun and moon glyphs the page popup already had), set before the zoom readout. Electron has no View menu item for it, so native has none. [THEMES]
- The dark highlighter is painted over, not multiplied: multiplying a pastel into a dark canvas is near black. The sticky keeps dark text in both themes (`StickyBodyLayer`). Comment marks, the edge default, the pink distribution and rearrange guides, and the preview blue are the same in both themes in Electron, so they stay fixed. [THEMES]
- Where Electron has no dark value to port, a choice was made and is named in `colors/dark.rs`: the page title and file name grey (`muted_text`, stone-400), the text selection (`#3f638b`, macOS's own), the right panel's zinc scale (the opposite end of the scale) and the run bar's wash (the same gradient ends under a black veil). [THEMES]

### Page anchoring

- An element question is `Effect::CaptureElement { page, request, point }` answered by `PageNotice::ElementCaptured`, with the request held in `Session.attach`, as a sync set's candidates are. A newer question for the same item takes the older one out, which is Electron's per-entity token. The shell only forwards. [ANCHORING]
- Nothing calls "capture" by hand. After every step, undo and redo, `attach::settle` asks about each anchored entity whose rect, page or URL is not what it was last asked with, so a new verb cannot forget to. Electron re-asks about every anchored entity after an undo; this asks about the ones the undo moved. A document just opened is taken as already asked. [ANCHORING]
- The stamp is `Document::apply(SetAnchor)` with no `History` step, and `element` stays nested in `pageAnchor` on disk. A step made before the stamp replays without it on redo, and the move it replays is asked about again. [ANCHORING]
- A stamp never moves what is drawn. The place recorded for the new element is where it is, plus whatever the element before it had travelled. [ANCHORING]
- Tracking is a long poll over the page's devtools channel, the shape the sync capture already uses: the script keeps the selectors' last places in the page, wakes on resize, load, a 150 ms debounced `MutationObserver` and (only while a tracked element is fixed or sticky) scroll, and answers with what changed or after two seconds. Electron pushes from its preload on the same events; CEF's channel has no push that survives a client toggling domains. [ANCHORING]
- An item on a fixed or sticky element gets the scroll shift and the element shift, which cancel. `page_state` moves such an element's place with each scroll report so the item does not lurch until the page says. A sticky element that is not yet stuck is wrong for one poll. [ANCHORING]
- A resize works on the rects as seen and folds them into the stored ones on release, in the same step, without re-resolving the page. Electron's resize does not fold: it writes seen coordinates under an old scroll stamp, so the item jumps. This is the rebase its drag end does. [ANCHORING]
- The fade is steps, not a mask. `PageBand::through` draws an item again in each of 16 strips above and below the page, clipped and at that strip's share of its opacity, so shapes, meshes and glyphs fade alike with no shader change (glyphon takes no per-pixel alpha). It reaches Electron's 48 px and the sides cut hard. Only an item its page has carried is drawn through the band; Electron bands every anchored item. [ANCHORING]
- An item in the fade but off the page is seen and cannot be pressed. Electron's band takes presses to its edge. An edge with an end scrolled out of its page is not drawn or hit. [ANCHORING]
- Deleting a page no longer deletes what is hooked to it (S5's reading of ADR 0034). It frees it, as Electron does, and an entity is first stored where it is seen so it does not jump; Electron leaves the stored rect. A freed region keeps its `docRect` and stops drawing, as in Electron. `delete_commands`, which the API calls with a bare document, frees without the fold. [ANCHORING]
- A copy is made of what is seen, and the clipboard holds anchors as page and URL only, as Electron's payload does. A copy that came with its page is hooked to the page's copy at scroll zero; any other is hooked by where it lands. [ANCHORING]

### Images

- An svg is rastered by `resvg` (text and system fonts on, embedded raster images off) on the loader thread at the size it is drawn, in logical pixels that the shell scales to device pixels, capped at 4096 on the long side. `update` asks with `Effect::RasterImage` and remembers the width it asked for (`Image::raster_width`); the raster is kept while the drawn width is 0.75x to 1.5x of it. Nothing is asked for while the zoom is moving: it must be the same at two looks in a row (two ticks, or a tick and an answer), so a gesture rasters once when it ends. Only svgs in the viewport are looked at. [IMAGES]
- A gif's frames are separate textures, frame `n` under `ImageKey::texture(n)` (frame 0 is the key itself, so stills are unchanged). The frame comes from `Event::Tick` time in `update`, only for gifs in the viewport, and `App::next_frame_in_ms` lets the loop sleep until the next frame, so an idle canvas or an off-screen gif draws nothing. A delay of 10 ms or less is shown as 100 ms, as browsers do. Frames are held at full canvas size; over 64 MiB of RGBA the frames are scaled down together as they are read. Mips are built for every frame. [IMAGES]
- A changed file is found by a stamp (mtime and length) every 500 ms on the decode thread, as the note thread does; `notify` is not a dependency. `ImageNotice::Changed` makes `update` return `LoadImage` again and leave the state alone, so the old texture draws until the new one lands. A failure while a picture is on screen keeps the picture (a file caught half written, or deleted); the next change tries again. [IMAGES]

### Performance

- Caches live in the compositor behind named types (`MeshCache`, `Laid` text layouts, `Batcher`) and in the shell (`FrameDemand`). `Scene`, `view` and `update` stay pure. [Performance]
- A frame is drawn only when something it shows changed. Frames keep coming for 250 ms after input so a ProMotion display does not drop its rate inside a gesture. [Performance]
- Kept text is placed by moving the pass viewport, which lands on whole pixels. A pan by part of a pixel draws text up to half a pixel off while it moves. [Performance]
- The idle memory sample is taken at 12 seconds, not 6. [Performance]

### Reversed

Decisions a later one replaced. The areas above have the later one.

- **Panels are egui 0.36** [M1]. Never built. Panels became pure models with a built-in renderer [P1/P2] and GPUI Kit [GPUI-SHELL, ADR 0040]. GPUI was turned down as the canvas renderer and that still holds.
- **Text editing uses cosmic-text's `Editor`** [M1]. The editor is the app's own, on a `TextMeasure` trait [T1].
- **`Tool` has eight variants without `inspect`, and `Gesture` three** [F4]. `Inspect` came with INSPECT-LOOP, and there are nine gestures.
- **The spike's Alt+drag move, and C toggling the comment tool off** [F4]. A plain drag moves [S3] and a tool's key pressed again does nothing [Tools].
- **Groups are hit after everything else** [S1]. The innermost group under the point wins [Groups/edges/S7].
- **`Hit` has no reorder dots or gap handles** [S1]. `Hit::Layout` came with ARRANGE.
- **A scaling text resize writes a stand-in height** [S4]. The height is measured each frame [T1 scene].
- **Scroll is taken as zero**, and **an anchor written at placement has no scroll offset** [F4, Tools]. Pages report scroll and anchored items follow it [CEF, Scroll].
- **A canvas-point comment draws a 12 px dot, and the highlighter is a flat 30% alpha** [F5b]. The first is a count pill [C3], the second is multiplied in at 70% [Polish].
- **`update` stops a Document's scroll at the top only** [T3]. The renderer reports heights and it stops at the end too [T4].
- **`Event::Paste(String)`** [T1]. Replaced by `Event::Clipboard` [Shell batch].
- **`specular-app` depends on `specular-testkit` outside tests** [Visual check]. The headless runner holds a `specular_interact::Driver` and the testkit is a dev-dependency [CLEANUP-A].
- **Page questions are answered from a synchronous `PageSource::element_at`, and every click on a real page is a canvas point** [C2]. CEF answers over the devtools channel and `element_at` is gone [CEF].
- **`cdp-target` answers only while the canvas has one page** [CEF]. Each page has its own socket [A3].
- **`--tab` only names the one open canvas, and `tab new`, `switch` and `delete` are 501** [A1]. All three are routes [P5].
- **A mixed-kind selection has no popup** [P1/P2]. It has one, with arrange, annotate and focus [P3/P4].
- **With no path the app opens the user's real space** [P5]. It opens the scratch space [Scratch space], and the GPUI shell then changed again to open the folder chosen in it or ask [APP-BUNDLE].
- **The menu's Rename canvas… opens the system save panel** [P5]. Still true in the winit shell. The GPUI shell renames in the sidebar [GPUI-SHELL].
- **The sidebar is a model with no renderer** [P4]. Both renderers draw it [P3/P4, GPUI-SHELL].
- **The write target is always the space folder** [RIGHT-PANEL]. It follows a bound origin [INSPECT-LOOP].
- **Repo bindings are read from and written to Electron's `repos.json`** [INSPECT-LOOP]. This app writes its own file [APP-BUNDLE].
- **`App` holds `StackCache` behind a `Mutex`, and the compositor depends on `specular-interact`** [T4, F5b]. Both undone by cleanup tasks 13 and 6 [CLEANUP-A].
- **Colours are the light theme only** [F5b]. They are one `specular_scene::Colors` per appearance, with a dark set [THEMES].

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
- SYNC-SETS: the seven decision lines that sat here are under "Decisions", in "Pages, CEF and sync sets". One was about the headless runner: a `wait` runs the pages before it moves the clock.

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
- For K1 to K6: the colours were Electron's light-theme values, each kind's own constants; THEMES moved them into `specular_scene::Colors` with a dark set. `text_vertical_align` on a shape is honoured, which Electron does not do.
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

### A3, the page half: per-page CDP, the browse verbs, page screenshots. See `git log -- native/crates/specular-api/src/cdp.rs`

- Each page has a CDP websocket of its own, backed by that browser's CEF devtools channel. `GET /pages/<id>/cdp-target` answers with it for any number of pages, with `generation` and `lastSnapshotGeneration`, and `POST /pages/<id>/snapshot-seen` records a read. Why this shape and not Electron's proxy is under Decisions.
- Where it lives. `specular-core`: `PageSource::{devtools_send, set_devtools_sink}`, and the synthetic source answers `Runtime.evaluate`, `Page.navigate`, `Page.reload`, `Page.getNavigationHistory`, `Page.getLayoutMetrics`, wheel input and the `enable` calls. `specular-api`: `cdp::PageProxy` (pure routing), `CdpAsk`, `ShotArea`, `Host::cdp`. `specular-cef`: `devtools_route`, `Devtools::send_raw`, the observer's `on_dev_tools_message`. `specular-app`: `cdp/` (frames, handshake, sockets, the hub), `app/shots.rs`, `ShellWindow::capture_area`. Both shells get all of it through `Runtime::start_api` and `serve_api`.
- Run end to end with two pages open, on the bundled debug CEF app with the installed CLI (agent-browser 0.21.2), in both shells, 37 checks each: `snapshot`, `screenshot -f`, `click` (selector and `@ref`), `fill`, `type`, `select`, `scroll`, `back`, `forward`, `reload`, `wait` (`--text`, `--url`, `--load`), `find text .. click`, and the passthroughs `get text`, `get title`, `get url`, `eval`, `console`, `errors`. Each check that could tell the pages apart did: a click on one page left the other untouched. The script is `native/fixtures/scenarios/cef/cli-pages.sh`, and its transcripts are `native/runs/a3-cli-pages/` (winit) and `native/runs/a3-cli-pages-shell/` (GPUI).
- Screenshots: `POST /pages/screenshot` is the page's pixels at its own size, `POST /pages/screenshot-composite` the page with border and title, and `GET /annotations/<id>` carries `metadata.regionScreenshot` for a region comment, so `specular annotation <id>` prints an image. All three PNGs were opened and read.
- `add note` now makes a `.md` Document from text over 300 characters or with a heading, a table row or a code fence, named from the heading. `add file` keeps a path inside the space relative, copies one from outside into `assets/` and sizes an image from its pixels. Each is one undo step, and undo leaves the file on disk, as undoing a drop does. New `ApiRun::ApplyWithFiles`, `Host::{inspect_file, space_entries}`, `Facts`.
- SKILL.md verbs, done: everything in the A1 entry, plus `snapshot`, `screenshot -f`, `click`, `fill`, `type`, `select`, `scroll`, `back`, `forward`, `reload`, `wait`, `find`, the agent-browser passthroughs, `add note` (both forms), `add file`, and the region picture in `annotation <id>`.
- Partial: `annotation <id>` still has an empty `priorFeedback` and no `regionElements`. `update --gap` is ignored. Pages get no device-frame metadata and `--landscape` only swaps the size. `--echo` and chained `&&` commands go through agent-browser unchanged and were not run.
- Not implemented, each a 501 that names the verb: `print-pdf` and `record` (what each needs is under Decisions), `arrange`, `auto-layout`, `breakpoints`, `component-states`, `design-system`. `/pages/snapshot`, `/pages/agent-snapshot`, `/pages/query-elements` and `/pages/find-target` are Electron routes the CLI does not call, and they say to use the browse verbs.
- For the next agent: there is no presence cursor, so the proxy does not sniff box-model answers as Electron's does. A deferred API reply (keep the job's sender, answer on a later turn) would unlock `print-pdf` and a fresh-paint page screenshot.
- Seen and not chased: the CLI leaves one agent-browser daemon per page for 60 seconds after its last command. A canvas switch gives a page a new host, and its old socket address stops working until the next `cdp-target`, which the CLI asks for on every command.
- Built with two Sonnet subagents in their own worktrees (the websocket transport, the two file verbs) while the proxy, the CEF channel and the routes were written here. Gate: fmt, clippy for the workspace and with `--features cef`, `cargo test --workspace` (1193 after the audit's prune), `fixtures/scenarios/run.sh`.
- Needs a human at a Mac: watch a page while `specular click` and `specular fill` run on it, to see the change arrive in the window. Run `specular snapshot -f` against a real site with iframes.

### CLEANUP-A: cleanup tasks 13, 6, 5, 9 and 10. See `git log --grep 'cleanup task'`

One commit a task, in that order. Each part lists what moved or was renamed, for the branches that rebase onto it.

**Task 13, the caches.** `App` holds no cache, and `view` takes one.

- `specular_scene::view(app, viewport)` is `view(app, viewport, &ViewCache)`, and `view_without_chrome` likewise. `ViewCache` is new (`specular-scene/src/cache.rs`, `Default`). Whoever draws frames owns one: `Runtime::view_cache` and `Headless::view_cache`. A one-off frame (a test, `TestApp::scene_snapshot`, a page screenshot of another view) passes `&ViewCache::default()`.
- It keeps a Document's markdown rows by text and width, each stroke's outline by the stroke and the width it draws at, and the edited Document's layout. An entry is used only while what it was made from is equal to what the app holds, and what a frame does not draw is dropped on the next. `ViewCache::built()` counts what the latest `view` had to build; `a_frame_like_the_last_parses_no_document_and_outlines_no_stroke` pins it at 0 and compares every scene against one from an empty cache.
- `App::stacks` is gone. `specular_interact::StackCache` is public, holds a `RefCell` (it was a `Mutex`) and starts again when the measure is a different `Arc`. `App::edit_marks(&StackCache) -> EditMarks` is new and is what `view` reads; `caret_rect`, `selection_rects`, `composition_rects` and `editing_layout` are unchanged and keep nothing. `edit::geometry` is split into `frame_of` (no measuring; `App::edit_frame` uses it) and `geometry_in(app, edit, &StackCache)`.
- Decision: `update` lays a Document out with nothing kept, 2 times for a typed character and 3 for an arrow key (counted with a counting measure on 101 rows: 202 and 303 row layouts). So the row memo is where the shell owns it: `GlyphMeasure` keeps 4,096 layouts in a hash map (it was 32 in a list scanned in order), cleared when full. `update` stays a pure function of its arguments.
- Not measured: the frame time in a window. The 0.6 to 0.9 ms is the audit's figure, and this run did not start the bench to compare.

**Task 6, the renderer's dependencies.** `specular-compositor` depends on `specular-core` and `specular-scene` and names neither `specular_interact` nor `specular_doc` in `src/`. Both are dev-dependencies still: its GPU tests build documents and drive them through the testkit.

- Moved to `specular_core::text` (new module, `specular-core/src/text.rs`): `TextSpec`, `TextMeasure`, `TextLayout`, `LayoutLine`, `CaretStop`, `SourceSpan`, `SourceStyle`. `specular-interact/src/edit/layout.rs` is `specular-core/src/text/layout.rs`, so `TextLayout`'s queries (`line_of`, `x_of`, `offset_at`, `caret_box`, `range_boxes` and the rest) and `LineBox` are `pub` where they were `pub(crate)`. `specular_interact` re-exports all of them under the names it had, so `specular_interact::TextMeasure` and the others still resolve. `edit/measure.rs` keeps `Measurer` and the estimate; `edit/source.rs` keeps `style_lines`.
- New: `specular_core::text::{TextFont, TextAlign}`, with the variants `specular_doc`'s have. `TextSpec::font` and `TextSpec::align` are these, since core cannot name `specular-doc`. Code that builds a `TextSpec` imports them from `specular_core::text`; `edit::frame::font` and `align` convert a document's token where one is read (a text's font, a shape label's alignment, a panel label's font).
- New: `specular_scene::OwnerId`, an alias of `specular_doc::EntityId`, for the id on `PageDraw::page` and `ColumnDraw::owner`. The compositor's `page_of` closures and `column_heights()` are typed with it, so no shell changed.
- Decision: the plan asked for an opaque `u64` on the text areas. Not done: both shells key page hosts and Document heights by `EntityId`, and scene snapshots print it, so a number would have needed a table back to the id in each shell. The alias gets the manifest right and leaves the compositor reading nothing from the id; a newtype is the step after, if wanted.
- Decision: the trait went under interact, into `specular-core`, not into `specular-scene` as the plan's first option had it. `App` holds the measure and `update` calls it, and scene sits above interact.

**Task 5, test support out of the binary.** `specular-testkit` is a dev-dependency of `specular-app`; `cargo tree -p specular-app -e normal` names neither it nor `insta`.

- New: `specular_interact::Driver` (`specular-interact/src/driver.rs`, which is `specular-testkit/src/input.rs` moved). It holds the `App`, the effects not yet drained, the pointer and the held modifiers, and has every scripted input (`press`, `drag_to`, `key`, `chord`, `type_text`, `wheel`, `select` and the rest) plus `send`, `act`, `measure_with`, `show_sidebar`, `effects`, `take_effects` and `set_effects`. `SHIFT`, `CTRL`, `ALT`, `CMD` and `CMD_SHIFT` are in `specular_interact::driver`; the testkit re-exports them.
- `TestApp` is `{ driver, start }`. Its API is unchanged: `specular-testkit/src/input.rs` is now a macro that declares each `Driver` method on `TestApp` and returns the `TestApp`. No test changed.
- The headless runner (`specular-app/src/headless/`) holds a `Driver`. It opens a document with `Event::BuiltinPanels(true)` and `Event::DocumentOpened`, and reads the panel layout from `panel::builtin::layout`.

**Task 9, one test binary for `specular-interact`.** Its 66 files in `tests/` (63, and the three `chat_*` files the right panel added) are modules of one binary, `it`.

- Every `specular-interact/tests/<name>.rs` is `tests/it/<name>.rs`, unchanged, and `tests/it/main.rs` is a `mod` line for each. A new test file needs its line there, or it is not compiled.
- Snapshots moved from `tests/snapshots/<file>__<test>.snap` to `tests/it/snapshots/it__<file>__<test>.snap`, with the `source:` line in each updated. `native/CLAUDE.md` points at `tests/it/gestures.rs`.
- A branch that added a file under `tests/` moves it to `tests/it/` and adds the `mod` line; one that changed an existing file follows the rename.

**Task 10, the second test pass, with a mutation check.** 1,194 tests to 1,159. Test code only; no file outside a test module or `tests/` changed. Five Sonnet subagents took a slice each in its own worktree. For each test they broke the rule it names in the production code, ran the test, and restored the file.

| slice | tests | mutation-checked | got past a mutation and were strengthened |
|---|---|---|---|
| `specular-doc`, `specular-core` | 59 to 59 | all 59 | 6 |
| `specular-interact` `src/` | 76 to 63 | all 63 | 2 |
| `specular-api` | 63 to 60 | all 60 | 7 |
| `specular-compositor` `src/` | 99 to 80 | all 80, and 9 readbacks | 14 |
| `specular-interact` `tests/it/` | 466 to 466 | 124, two a file | 3 |

- So 32 of the 386 tests checked would not have caught the break their name describes: a fixture that sat on the default (a group at y 0, a paste at the origin, `cols: 2` where 2 is the default), a count where the identity mattered, a rule a second guard also holds. Each now fails under its mutation. None of the 124 behavior tests was deleted.
- Pasted blocks are tables now: an array of rows and one loop. `document/tests.rs` is one table of 15 commands and one of 10 refusals; `camera.rs`, `page.rs`, `resize.rs`, `edit/format_tests.rs`, `raster_hold.rs` and `api/plan.rs` likewise. `contract.rs` is 15 to 12.
- Deleted where a named test fails under the same mutation: 13 interact unit tests the `tests/it/` files cover (`is_note_file`, `is_image_file`, `snap`, `snapped_to_45`, three of `arrange`'s four), and 8 compositor helper tests under a GPU readback of the same rule (the shader parse, gamma, opacity, column scroll, dash gaps).
- Deleted with nothing covering them: `shape_instance_is_sixty_four_bytes` (the stride is `size_of`) and `failed_import_is_not_cached` (the `?` returns before the insert, so no one-line change breaks it).
- Guarded twice, so one break alone passes and both together fail: delete while text is edited, a tab ref naming the active canvas, a group's members when it joins another group. Left as they are.
- Not mutation-checked: the other 342 tests in `tests/it/`, `specular-scene`, `specular-app`, `specular-bench`, `specular-cef`, and the compositor's `tests/` apart from the 9 readbacks. The subagents' tables are their own word; two rows were re-run here and held (`css_to_pixels` under `floor`, `arrange` without `MIN_GAP`).

- Gate, after rebasing tasks 9 and 10 onto the right panel's commits: fmt, clippy for the workspace and with `specular-app/cef`, `cargo test --workspace` (1,232 pass, with the right panel's tests), `fixtures/scenarios/run.sh` (every check ok). The scenarios also passed on tasks 13, 6, 5 and 9 before the rebase. The 1,194 and 1,159 above were counted before it.

### RIGHT-PANEL: the canvas agent chat. See `git log -- native/crates/specular-agent`

- `specular-agent`, pure: `Threads` (per canvas; draft, open, closed; the queue; one pin to one thread), `Thread::to_json`/`from_json` in Electron's file shape, `Pill`, the two prompts, `parse_line` over the `claude` stream, `claude_args`.
- Interact, `chat/`: `Action::Chat(ChatAction)`, `Event::{ChatPanel, ThreadsLoaded, Agent}`, `Effect::{LoadThreads, WriteThread, WriteThreadIndex, RunAgent, CancelAgent}`, `chat(&App) -> ChatModel`, `ToolbarModel.chat`. A saved comment queues into a draft thread and carries `metadata.threadId` in the step that inserts it. With `ChatPanel(true)` a comment draft is a passive marker and the panel's field is its composer; the winit shell keeps the on-canvas one.
- Runner, `specular-app/src/agent/`: `AgentBackend` and `AgentProcess` deal in raw lines, so `ClaudeCli`, `Scripted` (`SPECULAR_AGENT_SCRIPT=<file>`) and `Disabled` (tests, benchmarks, headless runs) share every rule in `AgentRuns`. Files go to `.specular/threads/<canvas id>/<id>.json` and `index.json`.
- Shell, `view/chat.rs` and `view/chat/`: Kit `Textarea` (`submit_on_enter`, `on_paste`), `MessageScroller`, `Bubble`, `TextView::markdown`, `ShimmerText`, `Collapsible`, menus. Plain elements for the chips, the thread rows, the 12 px resize strip and the run bar's wash.
- `SPECULAR_SHELL_SCRIPT` gained `type`, `shift-key`, `drag`, `right-click` and `paste-image`. A synthesized right click needs its button number set on the Core Graphics event.
- Run and read, 27 PNGs against the scripted runner: draft chip, queued comment, Shift+Enter, the run bar and streaming text, a follow-up queued behind a run, New, Back, Close, a badge click selecting its thread, resize clamps. Two real runs of `examples/agent_once` in a scratch folder: "reply with the word ok" gave `Finished { text: "ok" }`, and a resume of that session with a pasted PNG named its colour.
- Measured, debug build, kitchen sink: 120 fps with the panel open and idle, about 44 fps while a run's label shimmers, about 105 with the shimmer taken out. Every GPUI frame renders the whole root view on the canvas's thread. Measure in release before giving the shimmer a cached view.
- Left out: auto-fix per origin, the model chip, origin to repo bindings (the write target is always the space folder), the DOM-node pill (no inspect tool yet), the agent cursor chip, the run label's lift and fade, TIFF-only clipboards, thread routes in the HTTP API.
- For the next agent: `ThreadsLoaded` replaces the store, so it must arrive before the user queues anything. A field's Cmd+Z sometimes arrives as the menu bar's Undo, and it undid the canvas under the composer once in the full scripted run. `menus.rs` now holds Undo, Redo, Cut, Copy, Paste, Select all, Delete and Duplicate back from the canvas while a Kit field has the keys (`ShellView::typing`). Other menu keys (Cmd+D is covered, Cmd+G is not) still reach the canvas while typing.
- Needs a human at a Mac: Cmd+V with a real screenshot, an input method composing in the field (Enter must commit and not send), scrolling up during a run, dragging the edge by hand.

### SYNC-SETS: sync sets, with navigation, scroll and interaction sync. See `git log -- native/crates/specular-interact/src/sync.rs`

- Interact, `sync.rs` and `sync/interaction.rs`: `Action::ToggleSync`, `App::{is_synced, selection_synced}`, `PageNotice::{ScrollProgress, Pointed, Candidates}`, `Effect::{AskScrollProgress, ScrollPage, CapturePage, AskCandidates, ReplayPointer}`. The page popup has `page.sync` (`Icon::Sync`, Lucide `Link2`). Back, forward, reload and the address field go through `page_state::drive`, which sends the set the same way.
- Core: `locator.rs` (the port of `locator-kernel.ts`, with its test table), `PointKind`, `PageEvent::{ScrollProgress, Pointed, Candidates}`, `PageSource::{scroll_progress, scroll_to, set_capture, query_candidates, replay_pointer}`. The synthetic source scrolls and answers its fraction; it has no elements, so it captures nothing and offers no candidates.
- CEF: `sync_query.rs` (pure: the capture, candidates and replay messages and their parsers), `sync_host.rs` (the calls), `Asked::{ScrollProgress, Pointed, Candidates, Done}`. Replay is `Input.dispatchMouseEvent` on the peer's own channel.
- API: `POST /tasks/apply` (`specular breakpoints <url>`): a group, one page a preset in a row, one `syncId`, `breakpoint_variant` edges, one undo step. Electron's reply shape and error texts.
- Run on the bundled debug CEF app, `fixtures/scenarios/cef/sync.sh` (its own web server; 800 and 400 wide). PNGs read: a Menu click on the wide page opened the narrow page's panel, whose button sits elsewhere; a link click took both to the second page; Back took both back; a scroll to 1600 of 2100 px put the narrow page at 2819 of 3700. The notices show one load a page for each step.
- Not ported: the synced cursor, its halo and the refused-click wiggle (there is no presence layer), the hover resolution cache, skipping agent-driven pages, text input sync. The breakpoints route does not move the camera, writes no `generatedAt`, and normalises URLs with a small port of `normalizeUserUrl` in `tasks.rs`. `component-states` is still a 501.
- For the next agent: interaction sync skips pages with no origin (`file:`, `data:`), as Electron does, so test it over http. A candidates answer is the whole visible DOM as JSON when the bundle has no unique id; nothing caps it.
- Gate: fmt, clippy for the workspace and with `specular-app/cef`, `cargo test --workspace` (1246 after the rebase onto cleanup tasks 9 and 10), `fixtures/scenarios/run.sh`. Built with two Sonnet subagents (the locator port, the breakpoints route).
- Needs a human at a Mac: hover a menu on one page of a set in a window and watch the peers' `:hover`; scroll with a trackpad and judge whether followers need Electron's lerp; press the chain button in the GPUI shell.

### ARRANGE, part A: alignment and distribution guides. See `git log -- native/crates/specular-interact/src/guides.rs`

- `specular-interact/src/guides/`: `alignment_guides` and `distribution_guides`, ported with Electron's unit cases as two tables, and `GuideCapture`, the neighbours a move or a resize takes once at its press: what is in the viewport, not the selection, a group in view standing for its members.
- `App::guides() -> Guides` for the gesture in flight: a move's rects where they are, an Option-drag's copies with each original as a neighbour, a resize's moving edges only. Empty before a press travels.
- `specular-scene/src/view/guides.rs` draws them in the session layer, in screen space: a 1 px line in the selection colour, and `#ec4899` 1.5 px measures with 18 px caps in each even gap.
- Tests: `tests/it/guides.rs` (the grid and guides composing, Shift, the capture, copies, resize, even gaps), one scene snapshot at half zoom, `fixtures/scenarios/o-guides.txt` with seven PNGs read.
- Left out: the half-second flash of guides after an arrow-key nudge, and leaving the sidebar's width out of the viewport the neighbours are taken from.

### ARRANGE, part B: auto-layout groups, gap strips and reorder dots. See `git log -- native/crates/specular-interact/src/layout.rs`

- `specular-interact/src/layout/`: `reflow` packs a managed row or column from its members' least corner (snapped) at the group's gap, a member group travelling whole; `row.rs` is Electron's reorderable-row kernel with its unit cases as three tables; `act.rs` the verbs; `handles.rs` the dots and strips both hit-testing and `view` read; `drag.rs` the `Gesture::Line` for both.
- Actions: `AutoLayout` (Shift+Cmd+A, Arrange menu), `GroupLayout(Option<LayoutAxis>)`, `GroupGap(f64)`. `Hit::Layout(LayoutHandle)` sits under the anchors and over bodies.
- A loose selection that reads as an even row (gaps within 4) gets the same dots and strips with no group, and commits positions only. Two items always qualify, as in Electron.
- API: `POST /selection/arrange` (tidy in place, or pack at `gap` in reading order), `/groups/auto-layout`, `/groups/reorder-child`. Their `unported.rs` rows are gone. `arrange_command`, `place_command`, `auto_layout_command`, `gap_command` and `reorder_command` are the shared builders.
- Tests: `tests/it/auto_layout.rs` (one table of six changes that each re-lay the row in one step), `tests/it/layout_handles.rs`, three scene snapshots in `tests/groups.rs`, two API cases, `fixtures/scenarios/p-auto-layout.txt` with ten PNGs read and a `check.py` block on the saved file shape. Mutation-checked: without the reflow in `then_fit` eight tests fail; with the guide tolerance at 12 three do.
- Left out: the grayscale placeholder in a reordered box's slot (no desaturating draw; the box shows in its slot and floats at half strength), the dot's hover growing by animation, re-anchoring to a page after a loose reorder or regap, `specular update <group> --gap` through `/canvas/apply`, and live reflow of siblings while a member is resized (they settle at the release, as in Electron).
- For the next agent: a member's rect is an output. Change the group (`SetKind`), the order (`SetOrder`) or a member's size and let the step lay it out; a `SetRect` that only moves a member is put back. `fit_in_place`, which a drag calls every frame, never lays out.
- Needs a human at a Mac: the feel of the half-slot swap threshold while dragging a dot, and whether the 2 px gap bar is findable at 50% zoom.
### INSPECT-LOOP: the inspect tool, the DOM-node pill, repo bindings and auto-fix. See `git log -- native/crates/specular-interact/src/inspect.rs`

- Core: `InspectedNode`, `PageSource::inspect_at`, `PageEvent::Inspected`. The synthetic source answers from its grid (`synthetic_inspected_at`). CEF: `inspect_query.rs`, a port of `inspectionPayload` as one `Runtime.evaluate` on the page's devtools channel.
- Interact, `inspect.rs` and `inspect/popover.rs`: `Tool::Inspect` (`I`, the toolbar button after Comment), `Effect::InspectAt`, `PageNotice::Inspected`, `App::{inspect, inspected}`, the port of `placeInspectPopover`. Scene: `view/inspect.rs` draws the dashed outline and the card. The picked node is the pill (`chat/pill.rs`) and the prompt's focus line.
- Repos: `specular_agent::Repos` (the `repos.json` shape), `origin_of`, `Join`. Interact: `Action::Repo(RepoAction)`, `Event::ReposLoaded`, `Effect::{SaveRepos, PickRepoFolder}`, `repos_pane`, `chat::prompt::write_target`, `Composer::{folder_path, auto}`, the page popup's `page.repo` dropdown. `RunRequest.cwd` carries the repo to the runner (`AgentBackend::start(request, space, cwd)`); images are still read from the space folder.
- Auto-fix: `chat/commit.rs` joins the active thread and sends at once for a comment on an auto origin. A run in flight keeps it queued and the existing drain on `Finished` sends it, aimed at that pin.
- Shell: `app/repos_run.rs` (the file, the folder dialog, `SPECULAR_REPO_PICK` to answer it in a scripted run). GPUI: the Auto and Queue chip, the folder chip's tooltip, `settings_repos.rs` (connect, disconnect, add a URL, remove a binding).
- Verified in the GPUI shell on a bundled debug CEF build, scratch space, scratch config folder and the scripted runner, PNGs read: hovering an `h1` on a `data:` page outlines it with `h1 #hello.title.big 512 x 37`, Georgia 32px 700 and its text colour; a click puts `h1 "Hello data page"` in the pill; the page popup's folder control bound `http://127.0.0.1:<port>` to a scratch folder and `repos.json` held it; the chips read `site-repo` and `Queue`; with Auto on a comment ran at once with the scratch folder as its cwd, a second placed during the run waited and ran when the first ended; the settings pane listed the repo with its `auto` badge.
- Not done: the agent cursor chip (see Decisions). Electron's inspect tree and detail pane in the right panel, the box-model strips (margin, padding) on the outline, React component names and source locations. `fixPendingAnnotationsForOrigin`. The folder dialog has no parent window in either shell.
- Tests: `specular-interact/tests/it/{inspect,repos}.rs`, more in `chat_runs.rs` and `chat_pill.rs`, `specular-agent/tests/repos.rs`, `specular-scene/tests/inspect.rs`, the parser tests in `inspect_query.rs`. Testkit: `answer_inspect`, `inspect_model`, `with_repos`, `bind`.
- For the next agent: a page popup snapshot for a page with an origin now has the repo control, so its address field is narrower. `PageQueries` holds inspect questions by request, and a hover asks one a pixel, so a slow page answers late and the answer for another point is dropped. `fixtures/scenarios/d-text-edge-cases.txt` typed ` tiny` at the canvas with no text open; its `i` now arms the inspect tool, so the script types ` tny`.
- Built with four Sonnet subagents in their own worktrees over a hand-written skeleton commit (the vocabulary and stubs), then cherry-picked together. Gate: fmt, clippy for the workspace and with `specular-app/cef`, `cargo test --workspace` (1306 after the rebase onto ARRANGE), `fixtures/scenarios/run.sh`.
- Needs a human at a Mac: move the pointer across a real site with the inspect tool and judge whether the outline keeps up; bind a repo through the real folder dialog; run one real `claude` turn on a bound origin and check it edits the repo; quit the Electron app before changing bindings here.

### APP-BUNDLE: first run, settings and `Specular Native.app`. See `git log -- native/crates/specular-shell/scripts/bundle-app.sh`

- Interact: `first_run.rs` (`SpaceAsk`, `SpaceAction`, `onboarding(&App)`), `settings.rs` (`AppSettings`, `SettingAction`, `settings(&App) -> SettingsModel` with General, Repos, Shortcuts from `BINDINGS`, About). `Event::{SpaceNeeded, SettingsLoaded, About}`, `Action::{Space, Setting}`, `Effect::{ChooseSpace, OpenSpace, RevealSpace, Quit, SaveSettings}`. `BoundOriginRow::toggle_auto_fix`.
- App: `space/locate.rs` returns `Startup::{Open, Ask}`; `app/space_choice.rs` runs the space effects; `prefs` reads and writes `show`; `source_select::claim_profile`. `SPECULAR_SPACE_PICK` answers the folder dialog.
- Shell: `view/onboarding.rs`, `settings.rs` rebuilt from the model, `spaces.rs` (the folder dialog, a `.canvas` from Finder through `on_open_urls`), `pins::about`. The script driver has `choose NAME`.
- Bundle: `crates/specular-shell/scripts/bundle-app.sh` makes `target/release/bundle/Specular Native.app` (`com.lyleklyne.specular.native`, 614 MB, ad hoc signed, `codesign --verify --deep --strict` passes). [`docs/native-app-bundle.md`](../native-app-bundle.md) has the layout and what distribution still needs.
- Verified on the release bundle with `fixtures/scenarios/app/first-run.sh`, on a throwaway HOME: the first launch asks, "Create a new space…" seeds a scratch folder with the placeholder rewritten and remembers it, Electron's preferences and folder are untouched, a relaunch reopens the space, a cookie set by a local site comes back after a quit, and a moved space is named and not recreated. PNGs read: the first-run view with all three choices, and the missing-space view.
- Not seen: the screen was locked for the whole run, and the Kit then draws only its first frame. The settings dialog and the canvas after a space is made were never on a capture, and the choice was run with `choose`, not a click. Run the scenario again with the screen on and read `2-space-created.png` and `3-settings.png`.
- Not done: Finder opening a `.canvas` was not run (it goes through LaunchServices, which would register the bundle on this Mac). Skills and Models panes. Theme in General (no theme preference at this base). Everything under "What remains" in the doc.
- For the next agent: the Kit sidebar is drawn whether or not `SidebarModel::visible`, so "Show the sidebar at launch" changes the model and nothing on screen until the shell honours it. Cookies are encrypted with Chromium's mock keychain key (`use-mock-keychain`), so the profile on disk is not protected by the login keychain.
- Needs a human at a Mac: click each first-run button and use the real folder dialog; open a `.canvas` from Finder with the app running and with it closed; Settings > General > Change… with the dialog open; log in to a site, quit, reopen.

### SHELL-PARITY: the GPUI Kit shell brought level with the winit one. See `git log --grep 'Kit' -- native/crates/specular-shell`

Nothing was retired. Cleanup rows 1 and 2 in the plan say what now stands between each and deletion.

- Kit view. The sidebar is drawn from the whole model and only when `SidebarModel.visible`: the toggle, section folds, group and page chevrons, glyphs, trailing text, dimmed rows, comment rows, rename in a Kit field, the row menu from `context_menu`. It lies over the slot's left edge, so `covered_left` is what is drawn. A disabled menu item is a `MenuUnavailable` action nothing handles, which is what AppKit greys. Read from the live `NSApp.mainMenu`: items flip with the selection, each chord shows, a choice runs once. `Field` is a Kit input, drops land anywhere in the window, and the click that closes a Kit list no longer starts a gesture. The context menu and the popups beside an item stay in the canvas pass. The sidebar has no drag and drop in the model or the built-in renderer, so none was added.
- Bench. `--bench` runs in the Kit window through `specular-app/src/bench_drive.rs` (`Bench::start`, `step`, `presented`), which names no window. Each line carries `shell` and `work.processCpu`. `fixtures/bench/run.sh` takes `winit|kit` as a fourth argument.
- The Kit canvas draws only owed frames, pauses its display link a second after the last one, and follows a frame that ran past a tick with one more at once. Models are read at most ten times a second while the app keeps changing: building them every frame cost 1.6 ms on `stickies-2000`, and a GPUI redraw about 5 ms.
- Compared, release builds, 1600x960 at 2x, three runs a shell alternating, load average 10 to 36, so under 15% is noise. winit / Kit, medians:

| canvas | pages | pan cpu ms | pan fps | zoom cpu ms | zoom fps | long frames, pan |
|---|---|---|---|---|---|---|
| documents-50 | none | 1.57 / 1.53 | 120.1 / 120.0 | 1.49 / 1.44 | 119.7 / 118.8 | 2,0,0 / 0,0,1 |
| drawings-300 | none | 2.19 / 2.24 | 118.7 / 118.0 | 2.97 / 2.81 | 118.5 / 118.6 | 6,3,2 / 4,4,6 |
| edges-200 | none | 1.48 / 1.36 | 119.6 / 119.9 | 2.08 / 2.04 | 118.1 / 115.3 | 0,1,1 / 2,1,0 |
| stickies-2000 | none | 0.90 / 0.97 | 119.6 / 119.1 | 1.05 / 0.98 | 119.7 / 119.2 | 2,1,0 / 0,2,3 |
| stickies-500 | none | 0.34 / 0.50 | 120.1 / 120.0 | 0.35 / 0.50 | 120.1 / 120.1 | 0,0,0 / 0,0,0 |
| kitchen-sink | synthetic | 1.10 / 0.90 | 119.6 / 119.5 | 1.92 / 1.93 | 117.6 / 116.6 | 1,1,0 / 0,1,1 |
| mixed | synthetic | 0.49 / 0.52 | 120.4 / 118.5 | 2.05 / 2.09 | 120.2 / 116.0 | 30,43,30 / 31,35,44 |
| mixed | CEF | 0.46 / 0.53 | 120.1 / 120.0 | 2.49 / 2.49 | 120.0 / 119.6 | 0,0,0 / 0,0,0 |
| kitchen-sink | CEF | 1.09 / 1.07 | 120.0 / 119.9 | 2.07 / 2.11 | 118.5 / 117.0 | 0,1,0 / 0,0,3 |

- The Kit is behind in three places. Synthetic `mixed`, the row above, measured at the final head: 118.5 fps against 120.4 on the pan and 116.0 against 120.2 on the zoom, with p99 intervals of 21 ms against 16. Synthetic pages paint on the main thread and overrun a refresh in bursts. A display-link tick that passes during a frame is lost, where winit's loop starts the next frame at once. The Kit follows a late frame with another for up to 100 ms and then waits for the link, so input still arrives: with no limit it measured level (120.3 and 120.0) and hung the window when every frame overran, as a debug build's do. Without any catch-up it was 105 fps. Idle: 0 frames in both, and 0.44 to 0.61% of a core against 0.15 to 0.27%, which is GPUI's own display link firing every refresh. A light frame costs about 0.1 to 0.15 ms more CPU (`stickies-500` above), not sampled.
- The `stickies-500` and CEF `mixed` rows are from after the first catch-up frame and before the rebase; the other rows but synthetic `mixed` are from before it. CEF was not measured again at the final head. Latency after an overrun was not measured. A 1600x1000 winit window is 968 high on this Mac, so older winit numbers were taken at that. Output is in `native/runs/perf/shell-parity/`.
- Keys. `specular_interact::PhysicalKey` and `mac_key_input` hold the key tables, read from the `kVK` code by both shells. No table names winit.
- Backspace. It reproduced on every press in both shells, and on every letter: a press or release sent to a page with no character reaches it as a key-down on macOS, so each key arrived as two presses. `KeyInput.character` now carries it and `forward_key` sends it. 40 characters and 30 Backspaces leave 10. Test: `a_key_reaches_the_entered_page_as_one_press_and_one_release_with_its_character`.
- Sidebar sections. Not a bug: `sidebar-builder.ts:321` lists every item that is not a page under Notes, shapes included, and the Rust `partition` does the same. Two labels differ: Electron shows a note's whole text and names an untitled page by its viewport preset.
- Scripts. `SPECULAR_SHELL_SCRIPT_FILE=steps.txt` runs a headless script file in the Kit window with real `NSEvent`s, and `control NAME` clicks a control wherever it is drawn (`view/named.rs`, then the canvas pass's layout). `fixtures/scenarios/run-kit.sh` runs all eighteen scenarios there and they pass. A scripted run keeps its clipboard in memory.
- Headless without panels. `specular_interact::control_named` and `Event::Control` activate a control from the models with nothing laid out. `--script-panels off` and `PANELS=off fixtures/scenarios/run.sh` run that way: fifteen scenarios save the same bytes, and `d`, `l` and `n` differ because they click panel pixels by position. `hover-control` and `press-control` do nothing there.
- Fixed on the way: text gone from the scene while in sight stayed drawn from the kept layout, a deleted sticky's included (`text_hold.rs`). A double click zoomed the window only in the top 28 pt of the toolbar strip. `Action::ZoomTo(percent)` replaces a camera computed into every zoom option, which changed the toolbar model on every frame of a pan.
- Seen and not fixed: Cmd+Z in a page's text field undoes nothing there. "Delete canvas" trashes through Finder, blocked the main thread for two minutes and timed out (`space/files.rs`, both shells). One shutdown of two with 40 pages logged 26 browsers still open. A canvas row's control name is `sidebar.canvas.tab_1` headless and a hash of the file name in a window.
- One scripted run lost focus and sent Cmd+C and Cmd+V to the canvas, which pasted the real clipboard into a scratch canvas. Scripted runs now keep off the real pasteboard, and a Kit field refuses those keys by name in a script.
- The winit `Shell` and `run_window` are in `app/winit_window.rs`. `i-comments` clicks the canvas before its undos, because the right panel's composer keeps the keys.
- Gate at the rebased head: fmt, clippy for the workspace and with `--features cef`, `cargo test --workspace` (1,326), `fixtures/scenarios/run.sh`.
- Needs a human at a Mac: the nine items at the end of ADR 0040.

### THEMES: light, dark and system. See `git log -- native/crates/specular-scene/src/colors.rs`

- A theme choice (`Theme`: system, light, dark) lives in the app with the system's appearance; `Action::SetTheme` changes it, the toolbar's theme button steps it, `preferences.json`'s `themeMode` keeps it (Electron's key), and the winit shell and the GPUI shell send `Event::SystemAppearance` at start and on change, so `system` follows the OS live. `view`, the built-in panels, the dot grid and the Kit's theme all read one `Colors` per appearance; the light values are what was there and the light scene snapshots did not change.
- Pages: `Effect::SetPageColorScheme` and `PageSource::set_color_scheme` (CEF: `Emulation.setEmulatedMedia` over the devtools channel). Headless takes `--theme light|dark|system` and a `theme dark` script step, and applies it to real pages with `--source cef`.
- Tests: `tests/it/theme.rs` (button, appearance, SaveTheme, page effects), the prefs round trip, one dark scene snapshot, and the panel snapshots that moved because the toolbar gained a button. Looked at: the kitchen sink in both themes, the sticky popup and the context menu in dark; `fixtures/scenarios/run.sh` passes.
- Next agent: a colour on a drawn surface goes in `Colors`, both columns, not a constant in a view. `ViewCache` drops its Documents and strokes when the appearance changes, because they carry colours. `specular-shell`'s `theme.rs` colours are now functions of the theme in force (`theme::panel()`), `0xRRGGBBAA`.
- Not done: the native title bar and traffic lights follow the OS, not the app's choice. The CEF feature build is clippy-clean (`CEF_PATH=~/.local/share/cef cargo clippy -p specular-app --features cef`), but no real page has been seen switching scheme.
- Needs a human at a Mac: open a page that has a `prefers-color-scheme` stylesheet in a bundled CEF build and press the theme button; set the OS to dark with the choice on system and see the app follow without a restart; look at the GPUI shell's sidebar, chat panel and popups in dark.

### ANCHORING: element attachment, the scroll rebase, edges, the caret and the fade. See `git log -- native/crates/specular-interact/src/attach.rs`

- The "Not done in scroll-follow" line of the CEF entry is done, with re-anchoring of duplicates and pastes and the freeing of a deleted page's items and comments.
- Core: `PageSource::{capture_element, track_elements}`, `PageEvent::{ElementCaptured, ElementPlaces}`, `CapturedElement`, `ElementPlace`. The synthetic source answers from its grid (`synthetic_capture`, `synthetic_place`); the grid never reflows, so a headless run sees captures and stamps but no correction.
- CEF: `attach_query` (pure: the capture and tracker scripts, ported from the two Electron preloads, and their parsers) and `attach_host` (the poll, beside `sync_host`).
- Interact: `attach.rs` (questions, stamps, tracking), `Effect::{CaptureElement, TrackElements}`, `PageNotice::{ElementCaptured, ElementPlaces}`, `PageState::elements`. `scroll_follow` adds the element shift to the scroll shift (`anchor_shift`), `restamped` resets both, and `shown_rect`, `placed_rect` and `out_of_page` are what edges, anchors, the text frame, popups, reveal and focus now read. `anchor::{rebase, freed}`, `scope::contained`, `clone::Source`.
- Scene: `fade.rs` (`PageBand`), used by `view::show_through` and by region marks. `PAGE_FADE` is in interact because hiding reads it.
- Testkit: `capture_asked`, `answer_capture`, `answer_capture_with`. A test that stamps cannot end with `assert_undo_returns_to_start`: a redo replays the anchor from before the stamp. Call it before the answer, as `attachment.rs` does.
- Tests: `tests/it/attachment.rs`, more in `page_scroll.rs`, `anchoring.rs`, `resizes.rs`, `verbs.rs`; one scene snapshot of the fade; one GPU readback of it in `scene_effects_gpu.rs`; `attach_query`'s two. Each new test was mutation-checked. Scenario `q-anchoring` with `anchoring.canvas`, its PNGs looked at: items and the edge following a scroll, the fade at the page top, a resize, the caret and selection of a text edited while carried, a region, a duplicate, the page deleted and undone.
- Found by a test: an item wholly in the fade was still pressable (`hit.rs` fell back to the unclipped rect). Found by a PNG: the item popup sat over the stored rect.
- Needs a human at a Mac: none of this has run against a real page. The capture and tracker scripts are only parsed in tests, and the bundle was not built. Check on a real site: a sticky placed on a heading follows it when the page is resized to another preset; one on a fixed header stays on it while scrolling; the poll does not keep an idle page awake.
- Not done: the synthetic grid does not reflow. Selection outlines and handles of a carried item are not faded. Sidebar reveal does not scroll the page to an anchored item.
- Gate: fmt, clippy with and without `specular-app/cef`, `cargo test --workspace` (1298 pass), `fixtures/scenarios/run.sh` (all pass).

### IMAGES: svg, animated gifs and reloading a changed file. See `git log -- native/crates/specular-interact/src/images`

- Interact: `images/vector.rs` (the raster band and `Effect::RasterImage`), `images/animation.rs` (frame choice from the clock), `ImageNotice::{Animated, Changed}`, `App::{image_frame, animation_epoch, next_frame_in_ms}`. `ImageNotice` is no longer `Copy`.
- Scene: `view/file.rs` draws the current frame's texture. Shell: `images/{svg,gif,upload}.rs`, `Content` (still, animated, vector) out of the loader, `Uploaded` counting each image's textures for both the window and the headless run; the loader thread watches loaded files. `demand.rs` wakes for a gif at its next frame and counts frame changes as a reason to draw.
- Tests: svg band, reload keeps the picture, gif frames and off-screen silence in `tests/it/images.rs`; the gif performance pin in `demand.rs` (67 frames in 2 s for 30 ms delays, 0 off screen); `svg.rs` and `gif.rs` decode; the loader reports a rewritten file once. Each was mutation-checked.
- Fixture: the kitchen sink's Files row has `kitchen-sink.svg` and a four-frame `kitchen-sink.gif` (250 ms a frame).
- Looked at: svg at 3x zoom and 2x scale is sharp (text, dashes, edges), light and dark; the gif on different frames across `wait` steps in a script. The headless runner cannot overwrite a file between snapshots, so the live reload is tested only at the loader and pure halves.
- Not checked: a window (gif animating, a live reload, an svg re-raster during a real pinch), bmp and ico (still cards), svg text in fonts the system lacks, an svg with embedded raster images (they do not draw).
- Gate: fmt, clippy (also with `specular-app/cef`), `cargo test --workspace` (0 failed), `fixtures/scenarios/run.sh` all ok.

### MUTATION: cleanup task 10 finished, every test CLEANUP-A left unchecked. See `git log --grep 'mutation pass'`

1,353 tests to 1,403. Test files and test modules only, plus one fix in `specular-agent/src/describe.rs`. No fixture, scenario or kitchen-sink file changed. Twelve Sonnet subagents took a slice each in its own worktree at `41f3e6d7`: for each test they broke the rule it names in the production code, ran it, and restored the file. The result was rebased onto `aa421af8`.

| slice | checked | strengthened | guarded twice | not mutated |
|---|---|---|---|---|
| `specular-interact` `tests/it/`, all 75 files | 536 | 117 | 24 | 1 |
| `specular-scene` | 108 | 36 | 0 | 0 |
| `specular-app` | 177 | 44 | 2 | 0 whole, 5 in part |
| `specular-agent`, `specular-api` | 99 | 41 | 2 | 1 |
| `specular-bench`, `specular-cef` | 101 | 19 | 0 | 0 |
| `specular-compositor` `tests/` | 69 | 6 | 2 | 1 |

- 1,090 checked. 263 passed a break of the rule in their own name, one in four, and each now fails under it. The rest failed for the right reason as they were. CLEANUP-A recorded neither which 124 `tests/it/` tests nor which 9 readbacks it had checked, so all of both were done again; `specular-api`'s 60 were done again too and 22 of them were strengthened.
- The usual causes: a fixture on the default or on a grid line, one row where the rule has two sides, a count where the order mattered, `assert_undo_returns_to_start` standing in for "one step" (it undoes every step), and an expected value read from the code under test (`KAPPA`, `DEFAULT_TIMEOUT`, the measure's own stops).
- Two tests never reached the code they name. `comment_drafts::switching_tool_commits_the_draft_and_a_new_document_drops_it` clicked the pill of the comment it had just placed, so no draft was open when the document changed. `polish::a_drag_from_an_edge_where_it_crosses_an_entity_moves_the_entity` clicked after its drag had left the sticky selected. Both open the state first now.
- Replaced, 1: `auto_layout::an_item_dropped_into_a_row_takes_the_last_slot` is `..._takes_the_slot_its_stack_position_gives`, since an item sent to the back joins first (ADR 0015 D2). Renamed: `panels_tools::the_page_tool_offers_the_size_presets_and_hides_the_selections`, `hit_test::a_drawing_in_front_of_a_page_wins_only_where_its_ink_is`. Deleted: none.
- Added, 50 tests, each for a survivor no existing test could take a row for. The largest: `specular-api`'s `tasks.rs` (two tests pinned almost none of the breakpoint URL, label and option rules) and `specular-agent`'s `stream.rs` table (35 survivors in the tool labels and truncation limits, about 50 rows). The api and agent slice alone is 1,400 lines; cut it if the suite's size matters more than those rules.
- Product bug, 1, fixed: `host_of` kept the port of an IPv6 host, so the run bar read "Reading [::1]:3000". It cuts at `]`; a row in `stream_lines_become_notices` holds it.
- Guarded twice, 30, left as they are: one break passes and both together fail. Each is named with both lines in the slice's table. After the rebase `page_scroll::a_shape_scrolled_wholly_out_of_its_page_is_not_drawn` is one more: the clip hides what `out_of_page` would.
- Not mutated: `gpu_smoke::shared_frame_without_platform_import_is_released_immediately` (not compiled on macOS), `panel_builtin_cache::the_app_stays_send` (a compile-time check), `documents::a_file_that_does_not_exist_is_a_400_naming_the_path` (no arm to change that compiles). In part: `asset_run`'s no-space guard (the break writes into the crate folder), `repos_run`'s "never Electron's" half, `no_two_menu_items_share_a_shortcut`'s shell chords, `drop_run`'s relative path, the EXIF step in `images::decode`.
- Rules with no test, found on the way and left: an unchanged edge label records no step; `Pill::Dom`'s write target in `chat/prompt.rs`; a text's top or corner resize keeping its bottom edge; the sampler's mip filter; a GET body ignored in `api/server.rs:109`; `serve` draining every queued request (`api/mod.rs:78`); `Context::overlaps` itself under `bindings::no_two_rows_can_fire_for_the_same_key`; `placement.rs:27/:70/:150`. Likely dead code: `sidebar/rows.rs:135-139`, the `pressed` half of `repoint` in `panel/builtin/cache.rs`, `groups.rs:199`. `panels::the_context_menu_of_empty_canvas_is_drawn_as_a_menu_of_words` draws the key-hint preset surface, which is what the code means; the name is loose.
- The tables are the subagents' word. About 45 rows across all twelve slices were replayed here from the recorded file, line and text, and all matched. Two slices were sent back for a half of a test name they had called out of scope. The agents shared one scratch folder and overwrote each other's helper scripts twice, so one slice briefly mutated another's worktree; every commit was read for hunks outside a test module before it was picked.
- Rebase: test-module conflicts with THEMES and IMAGES in `images/mod.rs`, `icons/markup.rs` and `palette.rs`, both sides kept. Three snapshots the pass added were retaken on the themed toolbar, and the scrolled-out test scrolls past ANCHORING's fade.
- Gate on the rebased branch: fmt, clippy for the workspace, `cargo test --workspace` (1,403 pass, 0 fail). Not run: clippy with `specular-app/cef`, `fixtures/scenarios/run.sh`, and `specular-shell` and `specular-testkit` were not mutation-checked.
