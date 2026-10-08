# Rust native rebuild

**Status:** Built, and mostly unchecked by a person. Every wave below has landed on `claude/rust-chrome-spike`, along with most of what this plan deferred. The work was done in about a day by agents and verified by tests, headless scenario scripts and scripted window captures. Almost nothing has been used by hand. Start with the [handoff](./rust-native-rebuild-handoff.md).
**Builds on:** [`rust-cef-spike.md`](./rust-cef-spike.md).
**Scope as planned:** the core canvas. Pages, text, sticky notes, documents, shapes, drawings, edges, groups, comments, selection, undo, persistence. Agent chat, sync sets and auto-update were to come after.
**See also:** the [run log](./rust-native-rebuild-log.md), [`native/README.md`](../../native/README.md), and ADRs [0039](../adr/0039-rust-canvas-render-stack.md) to [0044](../adr/0044-ui-as-pure-models-with-replaceable-renderers.md).

## Status

As of commit `07285583` (2026-10-08). "Done" means built, gated and, where the log says so, run by script. It does not mean a person has tried it. "By hand" marks tasks whose feel nobody has judged. The run log entry of the same name has the detail.

| Task | Status | What is missing |
|---|---|---|
| **Wave 0, foundation** | | |
| F1 crate split | Done differently | Folded into F4 and F5. The crates are not the ones this plan named (see "Crates") |
| F2 typed document, undo | Done | |
| F3 `.canvas` reader and writer | Done | Electron files save byte for byte |
| F4 `Event`, `Effect`, `update` | Done | |
| F5 `Scene` and `view` | Done | |
| F6 testkit | Partial | No golden-image helper. GPU readback tests and headless PNGs do that job |
| F7 `native/CLAUDE.md` | Done | |
| **Wave 1, decisions and measurement** | | |
| M1 render bake-off | Done | ADR 0039, still Proposed. Its by-hand look was never done, and its egui choice was overtaken by ADR 0040 |
| M2 memory | Done | The extra memory was a 6 second sample. Settled, Rust is lighter (log, "Performance, part 2") |
| M3 results table in `rust-cef-spike.md` | Not done | The table is still empty. The numbers are in the log |
| **Wave 2, select tool** | | |
| S1 hit-test | Done | |
| S2 click, shift-click, marquee | Done | |
| S3 move | Done | By hand |
| S4 resize | Done | By hand |
| S5 delete, duplicate, option-drag, nudge | Done | |
| S6 clipboard | Partial | Not ported: copied file references, SVG, HTML and JSON text, long text becoming a Document, files copied in Finder. The two apps do not paste each other's entities |
| S7 stack order | Done | |
| S8 binding table | Done | |
| S9 autosave and file watch | Partial | No maximum wait on the debounce. An outside edit made while there are unsaved changes is overwritten with a logged warning |
| **Wave 3, kinds** | | |
| K1 shape | Done | A label that overflows a small shape is clipped to its middle line |
| K2 drawing | Done | No pressure. The highlighter has no gradient or grain |
| K3 text | Done | Hand and mono fonts fall back to system fonts |
| K4 group | Done | By hand |
| K5 edge | Done | The line runs through its label. By hand |
| K6 image file | Partial | png, jpeg, webp and gif only. No svg, no animated gif, no reload when the file changes |
| K7 page anchoring | Done, and past the plan | Scroll-follow is in. Element attachment (ADR 0032) is not. A resize does not fold the scroll shift |
| **Wave 4, text editing and documents** | | |
| T1 edit text in place | Done | By hand. No drag and drop of selected text, no caret affinity at a wrapped line's end |
| T2 IME | Done | Never typed with a real input method |
| T3 markdown, read only | Done | Links do not open. An image is the text `[image: alt]` |
| T4 markdown editing | Done | By hand. No rename of the file, no smart paste |
| T5 formatting shortcuts | Done | The built-in popup has the buttons. Not in a menu |
| **Wave 5, panels** | | |
| P1 toolbar | Done in both renderers | No hand or theme button |
| P2 item popup | Done | By hand |
| P3 page chrome | Done | No spin on reload. The address shows the old URL until the page reports the new one |
| P4 left sidebar | Done in the built-in renderer, partial in the GPUI shell | The GPUI shell lacks the toggle, folds and the newer row fields. No drag-reorder or favicons in either |
| P5 tabs and space folder | Done | Never run in a window when it landed. A `.canvas` another tool adds is not seen until the space reopens |
| P6 menu bar and context menu | Partial | Menu bar in both shells. The context menu exists only in the built-in renderer. GPUI menu items are never greyed |
| **Wave 6, comments and API** | | |
| C1 annotation model | Done | |
| C2 comment tool | Done | No way to edit a kept comment's text or to dismiss one from the canvas |
| C3 badges and regions | Done | A badge has no icon. Deleting a page leaves its comments bound to it |
| A1 HTTP server | Done | |
| A2 CLI verbs | Done | No second CLI. The Electron CLI runs against this app unchanged |
| A3 page snapshot and screenshot | Done | `print-pdf`, `record`, `component-states` and `design-system` answer 501 |
| **Built beyond this plan** | | |
| GPUI Kit shell (`specular`) | Done, behind parity | ADR 0040. Not at parity with the winit shell (see P4, P6). It never rests when idle |
| Performance pass | Done | Long frames not settled. The numbers were taken on the winit shell |
| Audit and test prune | Done | 1,778 tests cut to 1,074. Later work brought the suite to about 1,300 |
| Cleanup tasks 5, 6, 9, 10, 13 | Done | See "Cleanup tasks" |
| Agent chat (right panel) | Done in the GPUI shell | No thread routes in the HTTP API, no model chip |
| Sync sets, scroll and interaction sync | Done | No synced cursor. Followers jump where Electron eases. No text input sync |
| Alignment guides, auto-layout groups | Done | By hand |
| Inspect tool, repo bindings, auto-fix | Done | No inspect tree or box-model strips |
| First run, settings, `Specular Native.app` | Done | Ad hoc signed. Settings and the canvas after first run were never seen on a capture |
| **Not started** | | |
| Themes, presence cursors, auto-update, signing and notarization, element attachment, the focus session | Not done | Themes were in progress on another branch when this was written |

## Where the spike stood

This section and the next describe the starting point and are kept as written. When this plan was drafted, `native/` was 15.6k lines of Rust in five crates. It had:

- a winit window, a wgpu compositor, and CEF offscreen pages imported as IOSurfaces with no copy
- camera math that matches the Electron app
- input, wheel, key and IME forwarding into pages, plus `<select>` popups
- a yrs-backed document that round-trips `.canvas` files losslessly, with undo for page move, resize, add and remove
- one SDF shape layer that draws page borders, the selection outline, resize handles and comment regions
- page move, page resize, and a comment-region drag
- a bench that compares against Electron

It had no text rendering at all. No toolbar, no panels, no entity kinds other than pages, no saving from the UI, no HTTP API. At `07285583` the crates under `native/crates` are about 112k lines of Rust with their tests, in twelve crates.

### What the measurements say

The runs in `native/runs/` are in, though the results table in the spike plan is still empty. Reading the `compare-*.md` files:

- Frame times are at parity. Both shells hold 120 fps on 9, 20 and 40 static pages.
- The Rust shell has a better tail. On `static-20` slow-zoom, Electron had 10 long frames and a 106 ms max. Rust had 0 and 9.7 ms.
- Idle memory is higher in Rust, by 16 to 20 percent (`static-20`: 4333 MB vs 3623 MB). The spike predicted the opposite.

By the spike's own criteria that is a "neutral" result. Performance alone does not pay for a rewrite. The case for continuing rests on the other things: one process instead of nine renderers and an IPC layer, working `<select>` and IME, and a codebase an agent can change without tracing a broadcast through four files. I think that case is real, but the memory number deserves an hour of attention before the rebuild gets big (task M2 below).

**Corrected since.** The memory finding was wrong. The idle sample was taken at 6 seconds, while the process tree still held about 500 MB it lets go of by 10. Sampled at 12 seconds after three fixes, the Rust shell's idle footprint for `static-20` was 2,546 MB against Electron's recorded 3,623 MB. Frame times stayed at parity. Long frames are not settled: the machine was busy during the reruns (log, "Performance, part 2").

## What the Electron app spends its code on

About 97k lines of TypeScript in `src/`, plus 31k of tests.

| Area | Lines | Needed in a single-process Rust app? |
|---|---|---|
| `src/main` (runtime 22.9k of it) | 46.4k | Partly. Much of it is sync between the Y.Doc, runtime arrays and renderers. |
| `src/renderer` | 32.6k | The behavior, yes. The React and nine-app split, no. |
| `src/shared` | 13.4k | The pure math ports almost line for line. |
| `src/preload` | 5.0k | No. There is no bridge. |

Whole categories go away when state, input and drawing live in one process: the preload bridges, `ipc-contract.ts`, the diffed runtime store and patch broadcast, forward and reverse sync, overlay window management, the above-view input authority rules, page-host texture transfer and its timeout handling. My estimate for the core canvas in Rust was 25k to 35k lines including tests. The audit measured 58k of source and 31k of tests before the prune, with more built than the core.

## Greenfield architecture

Corrected to match the code at `07285583`. Where the plan said one thing and the build did another, the text says what was built.

### One loop, three pure functions

```
winit / CEF / HTTP ──> Event
                         │
        update(&mut App, Event) -> Vec<Effect>      pure, no I/O
                         │
        view(&App, viewport, &ViewCache) -> Scene   pure, no GPU
                         │
        render(&Scene)                              wgpu
```

- **`App`** is one struct. It holds the active canvas's `Document` and `History` (persisted, undoable) and `Session` (camera, selection, active tool, in-flight gesture, hover, focus), plus the `Space` with every other canvas of the folder, the agent threads, repo bindings, tool defaults and settings ([ADR 0043](../adr/0043-per-canvas-state-in-a-space.md)).
- **`Event`** is one enum, 31 variants today. Pointer, key, wheel, IME, page events from CEF, a clock tick each loop turn, file and clipboard answers, API requests.
- **`update`** changes `App` and returns `Effect`s. Effects are the only way I/O happens: save the file, create a page host, forward input to a page, set the cursor, write the clipboard, run an agent. There are 46 ([ADR 0042](../adr/0042-update-view-render-loop-effects-only-io.md)).
- **`view`** turns `App` into a `Scene`, a flat display list in canvas and screen coordinates. Rects, ellipses, polygons, paths, text runs, columns of text rows, shadows, page quads, images. Its caller owns a `ViewCache` for parsed markdown and stroke outlines.
- **`render`** draws the `Scene`. It knows nothing about entities or tools.

The consequence that matters for agents: every feature is testable with no window, no GPU and no CEF. A test builds an `App`, feeds events, and asserts on the document, the effects and the scene. The spike's `chrome_state.rs` already worked this way. This plan made it the rule, and the audit found no file, network, process, environment or clock read in `specular-doc`, `specular-interact`, `specular-scene` or `specular-api`.

### Document

Typed all the way down.

```rust
pub struct Document { entities, edges, annotations, order: Vec<ItemId>, notes, extra: JsonMap }

pub struct Entity { id: EntityId, rect: Rect, label: Option<String>, anchor: Option<PageAnchor>, parent: Option<EntityId>, kind: Kind, extra: JsonMap }

pub enum Kind { Page(Page), Text(Text), File(FileRef), Group(Group), Drawing(Drawing), Shape(Shape) }
```

- Every mutation is a `Command` value passed to `Document::apply`. `apply` returns the inverse command. Undo is a stack of inverses, kept in a separate `History`. One user action is one command (or one `Command::Batch`), so one undo step.
- Ids are the `.canvas` id strings, not slot-map keys as first sketched. Undo has to restore the same identity. The stack order is a `Vec<ItemId>` because edges interleave with entities. Rects are `f64`, as the file's numbers are.
- Adding a kind means adding an enum variant. The compiler then lists every `match` that needs a new arm: serialize, hit-test, view, popup. That replaces the entity-kind registry and the capability table.
- `extra` keeps unknown JSON fields so a load and save never drops another tool's data.
- `.canvas` stays JSON Canvas v1.0 with the `specular: {}` extension object. No format change.

**Decided: yrs is dropped.** The spike kept the document in a yrs doc, as the Electron app does. In Rust that meant string-keyed map access with runtime errors, and typed structs as a second copy to keep in sync. Inverse commands do the undo job. If cloud sync (ADR 0018) comes back, the command log is the place to attach it. See [ADR 0041](../adr/0041-typed-document-inverse-command-undo.md), Proposed.

### Interaction

One `Tool` enum, as ADR 0005 has it, with nine variants (`Select`, `AddPage`, `AddText`, `AddSticky`, `AddDocument`, `AddShape`, `Draw`, `Comment`, `Inspect`). One `Gesture` enum for the drag in flight (`Move`, `Resize`, `Marquee`, `Comment`, `Place`, `Draw`, `TextSelect`, `EdgeDrag`, `Line`). There is no pan gesture: a pan is a wheel event. Pointer routing is one function, in this spirit:

```rust
fn on_pointer(app: &mut App, ev: PointerEvent) -> Vec<Effect> {
    match (&app.session.gesture, hit_test(app, ev.pos), &app.session.tool) { ... }
}
```

Hit-test returns a typed `Hit` (`Comment`, `GroupLabel`, `Handle`, `Anchor`, `Layout`, `PageContent`, `EntityBody`, `GroupBorder`, `Edge`, `Empty`, `Panel`). There is no "input authority" question because nothing else receives input. Select-first, interact-second (ADR 0022) is `Focus::Page`, the entered page, checked once after every event.

Keys, menu items, panel controls and API act routes all ask for an `Action`. Key bindings are one const table of chord, context and action.

### Rendering and text

The bake-off (task M1) settled the canvas renderer, and a later spike changed the panel choice.

- **Canvas items** draw in the compositor's own wgpu pass: rounded rects and ellipses on the SDF shape layer, text through glyphon on cosmic-text, strokes and paths tessellated by lyon, in one 4x multisampled pass with the page quads. Vello was turned down: it was 2 to 3 times slower with dense text and copies page textures into its atlas ([ADR 0039](../adr/0039-rust-canvas-render-stack.md), Proposed).
- **Panels** are not egui. ADR 0039 chose egui and it was never built. Panels are pure models with two renderers: a built-in one that paints scene items, and GPUI Kit in the `specular` shell ([ADR 0044](../adr/0044-ui-as-pure-models-with-replaceable-renderers.md) and [ADR 0040](../adr/0040-gpui-kit-hybrid-shell.md), both Proposed). The CEF-HTML fallback was not needed.
- **Text editing** is the app's own editor in `specular-interact/src/edit/`, not a layout crate's. It asks a `TextMeasure` for a layout as plain data and computes carets, selections and motion from that. The compositor implements the measure on the font system glyphon draws with, so the caret sits where the glyphs are. IME events come from the shell.
- **Markdown documents** are read through pulldown-cmark into rows of styled text. An edit shows the source, one row a line, styled by a small hand-written styler. That replaces CodeMirror and react-markdown, and it is plainer than CodeMirror.

### Crates

```
specular-core        Camera, f32 geometry, the page and input model, PageSource, the text-measure trait.
specular-doc         Document, Entity, Command, undo, .canvas read/write.
specular-agent       The agent thread model, prompts, the claude stream parser, repo bindings. Pure.
specular-interact    App, Session, Space, Event, Effect, Action, Tool, Gesture, hit-test, update(), the text editor, the panel models.
specular-scene       Scene display list types, view(), the markdown parser, the built-in panel painter.
specular-api         HTTP routes as Events in and JSON out. No socket: the shell hosts the server. The CLI is the Electron one.
specular-compositor  wgpu: draws a Scene. Kept its spike name; the plan called it specular-render.
specular-cef         CEF page hosts. The plan called it specular-pages.
specular-app         Runtime (every effect runner), the command line, headless modes, --bench, and the winit window.
specular-shell       The GPUI Kit window, binary `specular`. Uses specular-app's Runtime.
specular-testkit     TestApp, snapshot macros, entity builders. Dev-dependency only.
specular-bench       Profiles, frame stats, memory, the Electron comparison.
```

Dependencies point one way: `core`, `doc`, `agent` <- `interact` <- `scene` <- `compositor`, with `api` beside `scene`, then `app`, then `shell`. Only `app`, `shell` and `cef` do I/O. There is no `specular-ui`: panel models are in `interact` and panel painting in `scene`. `native/README.md` has the full table.

### Rules that keep it easy for agents

- One feature is one vertical slice: a `Kind` variant or `Tool` variant, its commands, its hit-test arm, its `view` arm, its test. A slice touches the same places every time, and `native/CLAUDE.md` lists them.
- Every slice ships one behavior test: a scripted gesture through `testkit` asserting on the document. A scene snapshot (`insta`) only for a new draw rule, and no unit tests on trivial helpers. `native/CLAUDE.md` has the full rule. (This replaced "three tests per slice", which produced 1,778 tests in a day.)
- The gate is `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`. It needs no window or CEF, and the GPU tests skip without an adapter. A change that crosses features also runs `fixtures/scenarios/run.sh`.
- Keep the spike's lints: no `unwrap` outside tests, unsafe only where it is needed (the IOSurface import, CEF callbacks, AppKit calls in the shells).
- Files stay under about 400 lines. One enum arm growing past 80 lines moves to its own module. At the audit, six files and 18 functions were over (cleanup tasks 11 and 12).

### What to leave out

Cut, as planned. These exist in the Electron app mostly because of its architecture or as experiments, and none was built:

- the diffed runtime store, patch broadcast, preload bridges, overlay manager, page-host transfer pool
- lifecycle freeze, idle throttle and focus emulation (workarounds for Electron OSR)
- the component renderer, design-system store, video recorder and trimmer
- the debug window (the HTTP API and the headless modes cover it)
- MCP server (the CLI replaced it)

This plan deferred the following "until the core is solid". The run did not wait, and built them:

- the agent chat pane and auto-fix (spawning the `claude` CLI with stream-json output, no SDK)
- sync sets, scroll sync, interaction sync
- scroll-follow for anchored items
- auto-layout groups, gap strips, reorder dots
- alignment and distribution guides
- first run, the settings dialog, and an ad hoc signed app bundle

Still not built: element attachment (ADR 0032), presence cursors, the focus session, themes, auto-update, Developer ID signing and notarization.

## What the full core takes

The estimate as drafted, kept for the record. All six phases have landed. The "needs a human" column turned out right, and that checking has not happened yet.

Six phases. Sizes are my estimate of Rust lines including tests.

| Phase | Contents | Size | Needs a human at a Mac? |
|---|---|---|---|
| 0 | Crate split, `update`/`view` loop, typed document, testkit, render stack bake-off | 4k | Only to look at the bake-off result |
| 1 | Select tool: hit-test, select, marquee, move, resize, delete, duplicate, copy/paste, z-order, undo, autosave | 4k | Smoke |
| 2 | Kinds: text, sticky, shape, drawing (pen, highlight), group, edge, image file | 7k | Smoke |
| 3 | Text editing, markdown document, IME in editors | 4k | Yes. Editing feel cannot be judged headless. |
| 4 | Panels: toolbar, item popups, left sidebar, tabs, space folder | 5k | Yes |
| 5 | **Done.** Comments (three anchor types), page chrome (URL bar, back/forward, viewport presets), HTTP API and CLI verbs | 5k | Smoke |

Phases 0 to 2 and most of 5 are headless-verifiable, so agents can run them unattended. Phases 3 and 4 produce code overnight but need your eyes before anyone calls them done.

## Overnight list

The task list as drafted. The status of each task is in the table at the top.

Ordered in waves. Tasks within a wave touch different files and can run in parallel. Each task is one PR into a `rust-rebuild` feature branch and passes the gate above. Tasks marked (Mac) also need the morning check.

### Wave 0. Foundation, serial

- **F1.** Rename and split crates to the layout above. Move `chrome_state.rs`, `annotation.rs`, `placement.rs`, `input_map.rs` out of `specular-app` into `specular-interact`. No behavior change. Bench still runs.
- **F2.** `specular-doc`: `Document`, `Entity`, `Kind::Page` only, `Command` with inverse, undo and redo stacks. Port the spike's fixture round-trip tests onto it. Delete the yrs document.
- **F3.** `.canvas` reader and writer for all six kinds plus edges and annotations, typed, with `extra` passthrough. Test against every canvas in `tests/integration/` snapshots and the starter space: load, save, compare JSON values.
- **F4.** `Event`, `Effect`, `App`, `Session`, `update()`. Port the existing page move, resize and comment-region gestures onto it. `specular-shell` becomes a thin event translator plus effect runner.
- **F5.** `Scene` display list and `view()`. The compositor draws a `Scene` instead of `SceneView` plus `ShapeDraw`s.
- **F6.** `specular-testkit`: `TestApp::new(canvas_json).pointer_down(..).drag_to(..).key(..)`, `assert_doc_snapshot!`, `assert_scene_snapshot!`, and a golden-image helper on the software adapter the GPU tests already use.
- **F7.** `native/CLAUDE.md`: the five-file slice recipe, the gate, the test trio, vocabulary pointers to `CONTEXT.md`.

### Wave 1. Decisions and measurement, parallel with wave 0

- **M1.** Render bake-off. Two throwaway binaries on wgpu 30. Each draws 500 sticky notes with wrapped text, 200 freehand strokes and 100 arrows, panning and zooming from 0.02 to 3. One uses vello, one uses SDF shapes plus glyphon plus lyon. Report: does it build against wgpu 30, frame time, text sharpness at each zoom (golden images), lines of glue. Same for egui: does egui-wgpu build against wgpu 30 and draw a toolbar over the compositor's pass. Output is a one-page ADR draft with a recommendation. (Mac for the final look.)
- **M2.** Memory. Break the idle footprint down per process for `static-20` in both shells using the existing `rss` sampler. Find where the extra 700 MB lives. Suspects: 120 fps `windowless_frame_rate` holding more surfaces per page, the import cache keeping IOSurfaces alive, CEF flags that Electron sets and the spike does not. (Mac.)
- **M3.** Fill in the results table and verdict in `rust-cef-spike.md` from `native/runs/`.

### Wave 2. Select tool, after F4 to F6

- **S1.** `hit_test` over all entities in stack order, returning typed `Hit`. Port the cases from `tests/unit/hit-test.test.ts`.
- **S2.** Click select, shift-click toggle, marquee select. Port `marquee-selection.ts`.
- **S3.** Move for any selection, with axis lock on shift. Group children move with their group.
- **S4.** Resize for single and multi selection, with per-kind min size and aspect rules as a `match` on `Kind`. Port `resize-accumulator.ts` and `multi-resize-accumulator.ts`.
- **S5.** Delete, duplicate, option-drag copy, nudge with arrow keys.
- **S6.** Copy, cut, paste through a clipboard `Effect`. Paste a URL to create a page. Paste an image to create a file entity.
- **S7.** Stack order: bring forward, send back, to front, to back (ADR 0014).
- **S8.** Keyboard binding table: one `const` array of `(chord, context, Action)`. Port `bindings.ts`. Cmd+Z and Cmd+Shift+Z wired to the document.
- **S9.** Autosave with a 350 ms debounce as an `Effect::Save`. File watch that reloads on external edits when there are no unsaved changes.

### Wave 3. Kinds, after M1 picks a renderer. One task each, parallel

- **K1.** Shape. All kinds in `src/shared/shapes.ts`, with color and stroke width. `add-shape` one-shot tool with drag to size.
- **K2.** Drawing. Pen and highlight brushes. Port the perfect-freehand outline math (it is about 300 lines of pure geometry). `draw` persistent tool. Stroke scaling on resize (`scale-strokes.ts`).
- **K3.** Text, display only. Plain and sticky styles, three fonts (sans, mono, hand), size, alignment, color, wrap, auto height.
- **K4.** Group. Bounds from children, label, background, enter and exit, drag in and out (`group-drop-target.ts`).
- **K5.** Edge. Anchors on four sides, routing from `edge-geometry.ts`, arrowheads, stroke styles, label. Drag from an anchor to create. Cancel during a re-route deletes the edge, which is intended.
- **K6.** File, image. Decode, upload to a texture, draw. Drop a file on the canvas to create one. Copy into `assets/`.
- **K7.** Page anchoring, basic. An entity dropped with its center on a page anchors to it and moves with it. No scroll-follow, no element attachment yet.

### Wave 4. Text editing and documents, after K3

- **T1.** Edit a text or sticky in place. Caret, selection, word and line movement, clipboard, undo coalescing into one document command per edit session. (Mac.)
- **T2.** IME composition in the editor, using the winit IME events the shell already translates for pages. (Mac.)
- **T3.** Markdown file entity, read only. pulldown-cmark to styled text runs: headings, lists, code, links, emphasis, GFM tables and task lists.
- **T4.** Markdown editing as plain text with syntax styling. The `.md` file stays the source of truth. `add-document` tool creates the file. (Mac.)
- **T5.** Bold, italic, list and heading shortcuts as text transforms on the selection.

### Wave 5. Panels, after M1 confirms egui. Parallel

- **P1.** Toolbar. Eight tools, active state, shortcuts, tool-mode popup that writes tool defaults to app settings. (Mac.)
- **P2.** Item popup. One function, `popup_for(&Selection) -> PopupSpec`, with a `match` on kind. Color, stroke, font, size, alignment, shape kind, delete. (Mac.)
- **P3.** Page chrome. URL field, back, forward, reload, viewport preset menu from `device-catalog.ts`, loading state, title. (Mac.)
- **P4.** Left sidebar. Canvases list, Notes and Pages sections, click to select and focus, rename, add, delete. (Mac.)
- **P5.** Tabs and space folder. Open a folder, list `.canvas` files, switch the active one. Read `spacePath` from the same settings file the Electron app uses, so both apps open the same space.
- **P6.** Native menu bar and context menu with the app's shortcuts.

### Wave 6. Comments and API, after wave 2

- **C1.** Annotation model with the three anchor types and `pageAnchor`. Persisted in `.canvas` in the Electron app's shape.
- **C2.** Comment tool. Click on canvas for a point, drag for a region (the spike has this), click on a page element for an element anchor using a CEF devtools message for the selector and bbox.
- **C3.** Badges and region rects in the scene. Hide page-bound ones when the page URL no longer matches.
- **A1.** HTTP server on `localhost:29979` with the secret header. `GET /canvas`, patch, and act routes. Each request becomes an `Event` and gets a reply channel. Port the route contract tests.
- **A2.** `specular` CLI verbs against A1, so the existing skill file works unchanged against the Rust app.
- **A3.** Page snapshot and screenshot routes through CEF.

### What a person still has to do

The morning checklist this section held is done or overtaken. The renderer was picked without a person (ADR 0039), the memory breakdown is in the log, and pages load and take input under script.

What is left is all by hand, and it is the merged checklist in the [handoff](./rust-native-rebuild-handoff.md): open a real canvas and check it saves without a diff, type with a real input method, use a trackpad, resize the window, and try each interaction once. Text editing first, since it is the one most likely to feel wrong.

### Where it stands

One day of agent work reached further than this plan expected of two nights: every wave, a second shell, and most of the deferred list. The cost is in verification. The agents checked their work with tests, headless scripts and scripted captures, and the log says in each entry what was not run or not seen. Treat the app as a working draft that has never had a user.

The next steps, in order:

1. The by-hand checklist in the handoff. It will find bugs that scripts cannot.
2. The decisions only the user can make: ADR sign-offs, the four cleanup cuts below, distribution.
3. GPUI shell parity (context menu, sidebar folds, greyed menu items, resting when idle), then the cleanup cuts that depend on it.

## Cleanup tasks

From the audit in the run log ("AUDIT, part 1"). The audit cut the tests from 1,778 to about half, removed dead public items, put the f32 rects on one type and split the files it could. These are the larger cuts it left, ranked by lines removed for the risk taken. Line counts are source lines without tests, measured after the chrome branch landed.

**Status.** Tasks 5, 6, 9, 10 and 13 are done (log, "CLEANUP-A"). Tasks 1 to 4 wait on a decision from the user. Tasks 7, 8, 11 and 12 are open and need only a quiet tree.

| # | Task | Removes | Risk and what it needs |
|---|---|---|---|
| 1 | **Needs a decision.** Retire the built-in toolbar, popup and sidebar renderer once the GPUI Kit shell draws everything it does. `interact/panel/builtin` (3,524), most of `scene/panel` (1,876; the Kit still uses its icon paths and colours), testkit's panel helpers (564) and eight `panel_builtin*` test files (1,660). The panel models in `interact/panel` stay: both renderers read them. | about 7,000 | The headless runner draws and clicks these panels. Scenario `j`, the chrome scenario, `control NAME` script steps and the bench's "after" rows all depend on them, and the Kit cannot run headless. Needs a decision: either scripts name controls through the models with no drawn panel, or the built-in renderer stays as the headless one and the duplication is accepted. |
| 2 | **Needs a decision.** Retire `specular-app`'s winit window once the Kit shell is at parity (ADR 0040's list). Nine files name `winit`: `translate.rs`, `app/menu_bar/`, `app/gpu_window.rs`, `app/mod.rs`, `launch.rs`, `app/input.rs`, `app/turn.rs`, `app/drop_run.rs`. Drops the `winit` and `muda` dependencies. | about 1,700 | `--bench` only runs in the winit window, so every performance number in the log comes from it. The key tables use `winit::keyboard::KeyCode`. Port `--bench` and the key tables first, then compare one bench run across both shells before deleting. |
| 3 | **Needs a decision.** One shell crate. After 2, `specular-app` is a `Runtime` library plus a headless runner, and `specular-shell` is the only window. Merge them, or rename `specular-app` to what it is. | under 200, and one crate | Mechanical, but it touches every import in both. Do it when nobody else has a branch open on either. |
| 4 | **Needs a decision.** Retire the Electron comparison half of `specular-bench`: `compare`, `electron_trace`, `profile` aliases and the commands that read Electron's output. | about 1,200 | The spike's question is answered (run log, "Performance"). Keep it if another Electron comparison is planned. Decision only. |
| 5 | Take `specular-testkit` out of `specular-app`'s normal dependencies. The headless runner is built on `TestApp`, so test support ships in the binary. Move the script driver's core (press, drag, key, type) into `specular-interact` as a small `Driver`, and let testkit and the runner both wrap it. | about 300 of duplication | Touches every test's import path if done carelessly. Keep `TestApp`'s API as it is and change what is under it. |
| 6 | **Done**, with an `OwnerId` alias in place of the opaque `u64`. Fix the renderer's dependency direction. `specular-compositor` depends on `specular-interact` for `TextMeasure`, `TextLayout` and `CaretStop`, and on `specular-doc` for `EntityId` and `TextAlign`. Move the text-measure trait and its layout types into `specular-scene` (or a small text crate under interact), and key the compositor's text areas by an opaque `u64`. | 0, but the plan's rule holds again | A wide rename across interact, scene, compositor and both shells. Collides with any text work in flight. |
| 7 | The remaining rect types. `interact::PanelRect` (x, y, width, height in f32) is `core::Rect` under another name; `interact::ScreenRect` (min and size) differs only in shape. `interact/geometry.rs` has free functions `union`, `intersection` and `contains` over `doc::Rect` that belong on the type. | about 150 | `PanelRect` is in 12 panel files and goes away with task 1, so do this after deciding 1. `ScreenRect`'s `contains` includes all four edges and `Rect`'s does not: check each hit-test caller. |
| 8 | Narrow the public API. 219 `pub` items are never named outside their crate (interact 92 of 359, cef 45 of 56, bench 42 of 90, scene 19, compositor 9, core 8). Make them `pub(crate)` and let the compiler report what is then dead. | unknown until done; the name scan found only 5 dead | Safe, but it edits a line in about 150 files, so it conflicts with every open branch. Run it in one sitting on a quiet tree. |
| 9 | **Done.** One test binary for `specular-interact`. Its 63 files in `tests/` are 63 binaries, each linking the crate. Move them under `tests/it/` with one `main.rs`. | 0 lines, most of the suite's link time | File moves conflict with any branch that adds a test. Snapshot names change (`it__file__test.snap`), so rename the `.snap` files in the same commit. |
| 10 | **Done**, with a mutation check on 386 tests. A second pass on the tests. The suite is at about half. What is left to cut: the blocks pasted together in `specular-doc`, `specular-core` and interact `src/` (make them real tables or drop the repeats), compositor helper tests next to GPU readbacks of the same rule, and the 15 near-duplicate cases in `api/contract.rs`. | 150 to 250 tests | Low. Mutation-check what stays: break the code a test names and see it fail. Nobody has done that for any test here. |
| 11 | Split the files still over 400 lines: `cef/source.rs` (497) and `cef/client.rs` (418) were left because `specular-cef` had a branch open. `shell/view/controls.rs` (413), `interact/event.rs` (408), `interact/space/ops.rs` (405) and `interact/panel/builtin/dropdown.rs` (402) are at the line and have no clean seam; `controls.rs` and its dropdown code call each other both ways. | 0 | None for the cef pair once that branch lands. |
| 12 | The 18 functions over 80 lines, largest first: `Runtime::render` (132), `Runtime::capture` (122), `scene_pass::build` (110), `update` (109), `run_action` (106). `render` and `capture` share most of a frame's setup. | about 100 | `Runtime` is shared by both shells and had a branch open. `update` and `run_action` are flat matches and read fine long. |
| 13 | **Done.** Caches in the wrong place. `App` holds `StackCache`, a text-layout cache behind a `Mutex`, inside state that `update` owns; and `view` parses markdown and rebuilds freehand outlines every frame (0.6 to 0.9 ms). Give `view` one cache argument owned by the shell, and move `StackCache` into it. | 0, and a faster frame | Changes `view`'s signature, which every scene test calls. |
