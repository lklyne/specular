# Rust native rebuild

**Status:** Draft plan, nothing built yet.
**Builds on:** [`rust-cef-spike.md`](./rust-cef-spike.md) and the five crates in [`native/`](../../native/README.md).
**Scope:** the core canvas. Pages, text, sticky notes, documents, shapes, drawings, edges, groups, comments, selection, undo, persistence. Agent chat, sync sets, MCP and auto-update come after.

## Where the spike stands

`native/` is 15.6k lines of Rust in five crates. It already has:

- a winit window, a wgpu compositor, and CEF offscreen pages imported as IOSurfaces with no copy
- camera math that matches the Electron app
- input, wheel, key and IME forwarding into pages, plus `<select>` popups
- a yrs-backed document that round-trips `.canvas` files losslessly, with undo for page move, resize, add and remove
- one SDF shape layer that draws page borders, the selection outline, resize handles and comment regions
- page move, page resize, and a comment-region drag
- a bench that compares against Electron

It has no text rendering at all. No toolbar, no panels, no entity kinds other than pages, no saving from the UI, no HTTP API.

### What the measurements say

The runs in `native/runs/` are in, though the results table in the spike plan is still empty. Reading the `compare-*.md` files:

- Frame times are at parity. Both shells hold 120 fps on 9, 20 and 40 static pages.
- The Rust shell has a better tail. On `static-20` slow-zoom, Electron had 10 long frames and a 106 ms max. Rust had 0 and 9.7 ms.
- Idle memory is higher in Rust, by 16 to 20 percent (`static-20`: 4333 MB vs 3623 MB). The spike predicted the opposite.

By the spike's own criteria that is a "neutral" result. Performance alone does not pay for a rewrite. The case for continuing rests on the other things: one process instead of nine renderers and an IPC layer, working `<select>` and IME, and a codebase an agent can change without tracing a broadcast through four files. I think that case is real, but the memory number deserves an hour of attention before the rebuild gets big (task M2 below).

## What the Electron app spends its code on

About 97k lines of TypeScript in `src/`, plus 31k of tests.

| Area | Lines | Needed in a single-process Rust app? |
|---|---|---|
| `src/main` (runtime 22.9k of it) | 46.4k | Partly. Much of it is sync between the Y.Doc, runtime arrays and renderers. |
| `src/renderer` | 32.6k | The behavior, yes. The React and nine-app split, no. |
| `src/shared` | 13.4k | The pure math ports almost line for line. |
| `src/preload` | 5.0k | No. There is no bridge. |

Whole categories go away when state, input and drawing live in one process: the preload bridges, `ipc-contract.ts`, the diffed runtime store and patch broadcast, forward and reverse sync, overlay window management, the above-view input authority rules, page-host texture transfer and its timeout handling. My estimate for the core canvas in Rust is 25k to 35k lines including tests.

## Greenfield architecture

### One loop, three pure functions

```
winit / CEF / HTTP ──> Event
                         │
        update(&mut App, Event) -> Vec<Effect>      pure, no I/O
                         │
        view(&App) -> Scene                         pure, no GPU
                         │
        render(&Scene)                              wgpu
```

- **`App`** is one struct. It holds the `Document` (persisted, undoable) and `Session` (camera, selection, active tool, in-flight gesture, hover, focus).
- **`Event`** is one enum. Pointer, key, wheel, IME, page events from CEF, timer ticks, API requests.
- **`update`** changes `App` and returns `Effect`s. Effects are the only way I/O happens: save the file, create a page host, forward input to a page, set the cursor, write the clipboard.
- **`view`** turns `App` into a `Scene`, a flat display list in canvas and screen coordinates. Rects, paths, text runs, page quads, images.
- **`render`** draws the `Scene`. It knows nothing about entities or tools.

The consequence that matters for agents: every feature is testable with no window, no GPU and no CEF. A test builds an `App`, feeds events, and asserts on the document, the effects and the scene. The spike's `chrome_state.rs` already works this way. This plan makes it the rule.

### Document

Typed all the way down.

```rust
pub struct Document { entities: SlotMap<EntityId, Entity>, order: Vec<EntityId>, edges: Vec<Edge>, annotations: Vec<Annotation>, extra: JsonMap }

pub struct Entity { id: EntityId, rect: CanvasRect, anchor: Option<PageAnchor>, parent: Option<EntityId>, kind: Kind, extra: JsonMap }

pub enum Kind { Page(Page), Text(Text), File(FileRef), Group(Group), Drawing(Drawing), Shape(Shape) }
```

- Every mutation is a `Command` value passed to `Document::apply`. `apply` returns the inverse command. Undo is a stack of inverses. One user action is one command (or one `Command::Batch`), so one undo step.
- Adding a kind means adding an enum variant. The compiler then lists every `match` that needs a new arm: serialize, hit-test, view, popup. That replaces the entity-kind registry and the capability table.
- `extra` keeps unknown JSON fields so a load and save never drops another tool's data. The spike already proves this approach.
- `.canvas` stays JSON Canvas v1.0 with the `specular: {}` extension object. No format change.

**Decision to confirm: drop yrs.** The spike keeps the document in a yrs doc, as the Electron app does. In Rust that means string-keyed map access with runtime errors, and the typed structs become a second copy that must stay in sync. That is the two-layer model again. The Electron app uses Yjs for undo, and inverse commands do that job in a tenth of the code. If cloud sync (ADR 0018) comes back, the command log is the place to attach it. I recommend dropping yrs. The overnight list assumes it. If you want to keep it, D1 and D2 change and nothing else does.

### Interaction

One `Tool` enum, same as ADR 0005. One `Gesture` enum for the drag in flight (`Move`, `Resize`, `Marquee`, `DrawStroke`, `PlaceShape`, `EdgeDrag`, `CommentRegion`, `PanCanvas`). Pointer routing is one function.

```rust
fn on_pointer(app: &mut App, ev: PointerEvent) -> Vec<Effect> {
    match (&app.session.gesture, hit_test(app, ev.pos), &app.session.tool) { ... }
}
```

Hit-test returns a typed `Hit` (`Handle`, `EntityBody`, `PageContent`, `EdgeAnchor`, `Popup`, `Panel`, `Empty`). There is no "input authority" question because nothing else receives input. Select-first, interact-second (ADR 0022) is one match arm.

### Rendering and text

This is the part with a real choice in it, and the first overnight task is a bake-off to settle it.

Canvas items need text and vector paths that stay sharp from zoom 0.02 to 3. Two candidates:

- **Vello.** A wgpu 2D renderer. Paths, strokes, gradients, images and text (through parley) at any zoom, in one draw. Drawings, shapes, edges and text all become "push a path". Risk: its wgpu version may not match the workspace's wgpu 30.
- **Keep the SDF shape layer, add glyph atlas text** (cosmic-text plus a wgpu atlas such as glyphon) and tessellate strokes with lyon. More pieces, each small, and no dependency on vello's release schedule.

For panels (toolbar, sidebar, popups, later the chat pane) I recommend **egui**. Immediate mode suits this architecture: the panel code reads `&App` and returns `Event`s, with no widget state to keep in sync. It is also the UI library agents write most reliably. The same version risk applies.

Fallback if the Rust UI crates fight wgpu 30: draw panels as local HTML in CEF offscreen browsers. The shell already hosts those. It works, but it brings back a JS bridge, so it is the fallback and not the plan.

Text editing uses the layout crate's editor (parley `PlainEditor` or cosmic-text `Editor`) with IME events the shell already receives from winit. Markdown documents are edited as plain text with syntax-styled spans, parsed by pulldown-cmark. That replaces CodeMirror and react-markdown. It will be plainer than CodeMirror on day one.

### Crates

```
specular-doc        Document, Entity, Command, undo, .canvas read/write. No deps on anything below.
specular-interact   App, Session, Event, Effect, Tool, Gesture, hit-test, update().
specular-scene      Scene display list types and view(&App) -> Scene.
specular-render     wgpu: draws a Scene. Absorbs specular-compositor.
specular-pages      CEF page hosts. The current specular-cef.
specular-ui         Panels and popups.
specular-api        HTTP routes and the CLI verbs, as Events in and JSON out.
specular-shell      winit, effect runner, autosave, file watch. The only binary.
specular-testkit    Headless App driver, scene snapshot helpers, golden images.
specular-bench      Unchanged.
```

Dependencies point one way: `doc` <- `interact` <- `scene` <- `render`/`ui` <- `shell`. Only `shell` and `pages` do I/O.

### Rules that keep it easy for agents

- One feature is one vertical slice: a `Kind` variant or `Tool` variant, its commands, its hit-test arm, its `view` arm, its tests. A slice touches the same five files every time, and `native/CLAUDE.md` lists them.
- Every slice ships three tests. A command round trip (apply, undo, compare). A scripted gesture through `testkit` asserting on the document. A scene snapshot (`insta`) of the resulting display list.
- The gate is `cargo fmt --check && cargo clippy -- -D warnings && cargo test`. It runs in seconds, with no Mac, GPU or CEF needed.
- Keep the spike's lints: no `unwrap` outside tests, unsafe only in the IOSurface import and CEF callbacks.
- Files stay under about 400 lines. One enum arm growing past 80 lines moves to its own module.

### What to leave out

Cut for good, I think. These exist in the Electron app mostly because of its architecture or as experiments:

- the diffed runtime store, patch broadcast, preload bridges, overlay manager, page-host transfer pool
- lifecycle freeze, idle throttle and focus emulation (workarounds for Electron OSR)
- the component renderer, design-system store, video recorder and trimmer
- the debug window (a `--dump-state` flag and the HTTP API cover it)
- MCP server (the CLI replaced it)

Defer until the core is solid, then decide each one fresh:

- agent chat pane and agent-fix (spawn the `claude` CLI with stream-json output, no SDK)
- sync sets, scroll sync, interaction sync
- element attachment and scroll-follow for anchored items
- auto-layout groups, gap handles, reorder dots
- alignment and distribution guides
- presence cursors, onboarding, settings window, auto-update, packaging and notarization

## What the full core takes

Six phases. Sizes are my estimate of Rust lines including tests.

| Phase | Contents | Size | Needs a human at a Mac? |
|---|---|---|---|
| 0 | Crate split, `update`/`view` loop, typed document, testkit, render stack bake-off | 4k | Only to look at the bake-off result |
| 1 | Select tool: hit-test, select, marquee, move, resize, delete, duplicate, copy/paste, z-order, undo, autosave | 4k | Smoke |
| 2 | Kinds: text, sticky, shape, drawing (pen, highlight), group, edge, image file | 7k | Smoke |
| 3 | Text editing, markdown document, IME in editors | 4k | Yes. Editing feel cannot be judged headless. |
| 4 | Panels: toolbar, item popups, left sidebar, tabs, space folder | 5k | Yes |
| 5 | Comments (three anchor types), page chrome (URL bar, back/forward, viewport presets), HTTP API and CLI verbs | 5k | Smoke |

Phases 0 to 2 and most of 5 are headless-verifiable, so agents can run them unattended. Phases 3 and 4 produce code overnight but need your eyes before anyone calls them done.

## Overnight list

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

### Morning checklist

1. Read the M1 bake-off note and pick the renderer. Wave 3 onward depends on it.
2. Read the M2 memory breakdown.
3. Build on the Mac and run `specular-app fixtures/input.canvas`. Check that pages still load and take input.
4. Open a real canvas from your space. Check it draws every kind and saves without a diff in `git`.
5. Try each (Mac) task by hand. Text editing first, since it is the one most likely to feel wrong.

### What to expect by morning

Waves 0, 1 and 2 should land, with M1 and M2 written up. Those are well-specified ports with fast tests. Wave 3 and wave 6 can start only if M1 gives a clear answer without you, so plan for some of K1 to K6 and all of C1, A1, A2. Waves 4 and 5 are a second night. One night will not reach parity. It should reach a Rust app that opens your real canvases, selects, moves, resizes, undoes and saves, and draws most kinds.
