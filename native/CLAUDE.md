# native/ — the Rust rebuild

The Rust version of Specular. The plan, the architecture and the task list
are in [`docs/plans/rust-native-rebuild.md`](../docs/plans/rust-native-rebuild.md).
Read its "Greenfield architecture" section before changing anything here.
Vocabulary follows [`CONTEXT.md`](../CONTEXT.md). The run log is
[`docs/plans/rust-native-rebuild-log.md`](../docs/plans/rust-native-rebuild-log.md).

## The shape

```
Event -> update(&mut App, Event) -> Vec<Effect>     pure, no I/O
         view(&App) -> Scene                        pure, no GPU
         render(&Scene)                             wgpu
```

- `specular-doc` owns the typed `Document`. Every mutation is a `Command`;
  `apply` returns the inverse, and undo is a stack of inverses.
- `specular-interact` owns `App`, `Session`, `Event`, `Effect`, `Tool`,
  `Gesture`, hit-test and `update`.
- `specular-agent` owns the canvas agent threads: the `Threads` store, the
  thread files' shape, the prompts and the `claude` stream. It is pure.
  Interact holds the state and `chat(&App)` is the right panel's model.
- `specular-scene` owns the `Scene` display list and `view`, one module per
  kind under `src/view/`.
- `specular-api` turns an HTTP request and `&App` into a read answer or an
  `Event::Api`, so an agent's write goes through `update` like a key press.
  The shell hosts the server (`specular-app/src/api/`).
- `App` owns one `Space`: every canvas of the open folder, each with its
  own `Document`, `History`, camera and selection. `app.document` is the
  active canvas's. Canvas operations are `Action::Canvas`, and the file
  work comes back as effects.
- Only the shell and the CEF crate do I/O. Everything else is testable with
  no window, GPU or CEF.
- There are two shells over one `specular_app::Runtime`, which runs every
  effect. `specular-app` is the winit one. `specular-shell` (binary
  `specular`) is the GPUI Kit one (ADR 0040): the Kit draws the toolbar and
  sidebar from the models, and the compositor draws the canvas under it. A
  new effect gets its runner in `specular-app/src/app/`, once, for both.

The Electron app under `../src` is the behavior spec, not the structure
spec. Read the TypeScript to learn what a feature does and to port pure
math and test cases. Do not copy its layering (IPC, broadcasts, stores,
registries). Never edit anything under `../src`.

## Adding a feature

A feature is one vertical slice. It touches the same places every time:

1. `specular-doc`: the `Kind` variant or fields, its `Command`s, its
   `.canvas` read and write.
2. `specular-interact`: the `Tool` or `Gesture` arm and the hit-test arm.
3. `specular-scene`: the `view` arm.
4. One behavior test: a scripted gesture through `specular-testkit` that
   asserts on the document and ends with the undo check. See "Tests" below
   for what else earns a test.

Adding an enum variant should make the compiler list every `match` that
needs a new arm. Do not add wildcard arms over `Kind`, `Tool` or `Gesture`.

### Tests

The suite stays small on purpose. A test has to clear the four-criterion
bar in [`tests/README.md`](../tests/README.md): it catches a regression you
can name, asserts on an observable outcome, survives a refactor, and reads
on its own.

- One behavior test per user-visible behavior. When the same gesture or
  verb runs on several kinds, handles or modifiers, that is one
  table-driven test with a row per case, not a test per case.
- A scene snapshot only for a new draw rule. A new state of a rule that
  is already drawn does not need one.
- A GPU readback only for a rendering rule a snapshot cannot see (a
  colour, coverage, a blend).
- A performance pin when a change exists to make something cheaper: a
  cache hit, a draw-call count, an idle frame that is not drawn.
- A `.canvas` round trip or byte test for anything read or written.
- An API contract case for a new route.
- No unit tests on trivial helpers, none that restate the type system,
  and none that pin how a private function is called. If a pure rule is
  hard enough to get wrong (resize math, text segmentation, markdown), it
  gets one or two tests on its inputs and outputs.
- A bug that only showed up in use goes into `fixtures/scenarios/`.

Before adding a test, look for the one that already covers the behavior
and extend its table.

### The gesture test

It goes in the `tests/` directory of the crate that owns the behavior
(`specular-interact/tests/it/gestures.rs` is the model), with
`specular-testkit` as a dev-dependency. A crate's `src/` unit tests cannot
use the testkit on that crate's own types.

```rust
use specular_interact::Key;
use specular_testkit::{SHIFT, TestApp, assert_doc_snapshot};

#[test]
fn shift_drag_moves_the_page_along_one_axis() {
    let mut app = TestApp::with_pages(2); // p1 at (100, 100), p2 at (700, 100), 400x300
    app.press((200.0, 150.0)).hold(SHIFT).drag_to((260.0, 130.0)).release().let_go();
    assert_doc_snapshot!(app, @"");       // the failure prints the text to put here
    app.assert_undo_returns_to_start();   // every test that changes the document ends with this
}
```

- Start from `TestApp::with_pages(n)`, `TestApp::with_entities([..])` or
  `TestApp::from_canvas(json)`. Entities come from `page`, `text`, `shape`,
  `file`, `drawing` and `group`, each taking an id and a rect. `sticky`,
  `plain_text` and `labelled` also take the text.
  `note(id, rect, "plan.md")` is a Document; its text arrives with
  `note_text("plan.md", "..")`, and a double click then edits its source.
  `inside("g", entity)` puts one in a group, and
  `connected(document(entities), "e1", "a", "b")` adds an edge, and
  `with_edge(document, Edge { from_side, label, ..Edge::new(..) })` one that
  names sides, a label or a style.
- Input chains: `pointer_move`, `press`, `drag_to`, `release`, `drag`,
  `click`, `double_click`, `key`, `chord(CMD, Key::Char('z'))`,
  `type_text("hi")`, `wheel`, `pinch`, `tick`. `hold(mods)` keeps modifiers
  down until `let_go()`. `select`, `tool`, `zoom`, `undo`, `redo` and `act`
  run `Action`s. Anything else goes through `send(Event)`.
- Text editing: `double_click` a text, sticky or shape to edit it, then
  `type_text`, `key`, `chord`, `triple_click`, `compose("に")`, `commit("日")`
  and `paste("..")`. Read back with `editing_text()` and `caret()`, which is
  `(caret, anchor)` in bytes. Text is measured by `FixedAdvance`: 10 units a
  character and 20 a line, so `App::caret_rect()` is in round numbers.
  `measure_with(Arc::new(..))` swaps the measure, for a test on real fonts.
- Comments: `with_comment(document, comment(id, anchor, "text"))` starts
  with one in the document. A click or a drag with `tool(Tool::Comment)`
  opens a draft, read back with `comment_draft()`; `type_text` and
  `key(Key::Enter)` commit it. `answer_element(..)` and `answer_grab(&[..])`
  answer the latest `QueryElement` and `QueryRegionGrab` as the shell would.
- Canvases: `TestApp::with_space([("Home", document), ("Notes", document)])`
  starts with several, the first active. `switch_to("Notes")` and
  `act(Action::Canvas(..))` change them; read back with `canvas_names()`,
  `active_canvas()` and `app().canvas_document(&id)`.
- The right panel: `with_chat_panel()` tells the app the shell draws one,
  so a comment draft is finished there and the canvas keeps only its marker.
  `chat()` is the `ChatModel`, `chat_snapshot()` / `assert_chat_snapshot!`
  hold it as text, `send_chat("..")` presses Send (`send_chat_with` adds
  pasted images), and `agent_says(Notice::..)` plays the `claude` run of the
  open thread (`agent_says_in(&id, ..)` another). Writes and runs come back
  as `Effect::WriteThread`, `WriteThreadIndex` and `RunAgent`; read the
  store with `app().threads()` and the open thread with `chat_thread_id()`.
- Pages: `page_reports("p1", PageNotice::Scrolled { x: 0.0, y: 40.0 })` (or `Title`,
  `Url`, `Loading`, `DevtoolsUrl`) is a page saying something about itself, as
  the shell sends it; read back with `app().page_state(..)` and `page_scroll(..)`.
  A sync set is made with `select(&["p1", "p2"]).act(Action::ToggleSync)`; its
  following is asserted on the effects (`Navigate`, `ScrollPage`,
  `AskCandidates`, `ReplayPointer`) after `page_reports` of `Url`,
  `ScrollProgress`, `Pointed` and `Candidates`. See `tests/it/sync.rs`.
- Read back with `document()`, `session()`, `selection()`, `selected()`,
  `selected_ids()`, `rect("p1")` and `entity("p1")`. `take_effects()` drains the effects
  returned since the last drain; call it before the step whose effects the
  test asserts on.
- `assert_doc_snapshot!(app)` keeps its snapshot in
  `tests/snapshots/<test>.snap`; `INSTA_UPDATE=always cargo test` writes
  it. Prefer the inline form for small documents: a failing run prints the
  new text, and `cargo insta accept` (from `cargo install cargo-insta`)
  writes it into the source. Read the snapshot before accepting it.
- The built-in toolbar and popup are off in a test until
  `app.with_panels()`. Then `click_control("text.color")` clicks a control
  by its name, looking its rect up in the layout (`hover_control`,
  `press_control`, `control_rect` likewise), `assert_panel_snapshot!(app)`
  holds the layout as text, and `panel_scene_snapshot()` what the panels
  draw. `view` never draws them, so `scene_snapshot()` is the same either
  way.
  A text field is clicked like any control and then typed into:
  `enter_in_field("page.url", "example.org")` replaces its text and presses
  Enter, `field_edit()` reads what is typed so far, and Escape puts the
  old value back.
  The sidebar starts hidden, as in Electron: `show_sidebar(true)` (or the
  `sidebar.toggle` button) shows it. Its controls are `sidebar.canvas.<id>`
  (`.name` is the rename field, `.menu.rename` and `.menu.delete` the
  right-click menu), `sidebar.add`, `sidebar.head.<canvases|notes|pages>`,
  `sidebar.<notes|pages>.<entity id>` (`.toggle` its chevron) and
  `sidebar.pages.comment.<id>`. Only rows in the window are laid out, so
  scroll with `wheel` over the sidebar before clicking one below it.
  `app.covered_left()` is the width it covers, which zoom to fit, a reveal,
  zoom steps and popups read.
  `right_click(at)` opens the context menu, which is a `PopupModel` of
  choices at a point (`context_menu(app, &target, at)`, drawn in the
  dropdown slot). `assert_menu_snapshot!(app)` holds its model,
  `menu_open()` says whether one is open, and `click_control("menu.duplicate")`
  picks an item (`menu.<label in lower case, dashes>`; a canvas row's menu is
  `sidebar.canvas.<id>.menu.rename|delete`).
- A page is select-first (ADR 0022). `click` selects it, a second `click`
  or a `double_click` enters it, and only an entered page gets input.
- The scene snapshot is `assert_scene_snapshot!(app)`, in
  `specular-scene/tests/view.rs`. It prints one line per display-list item
  in paint order. `TestApp::scene_snapshot()` returns the same text, and
  `specular_scene::view(app.app(), viewport)` the `Scene` itself.

## Looking at what it draws

`specular-app` draws a canvas into a PNG with no window, on the real GPU
with the synthetic page source. Nothing else is written.

```sh
cargo run -p specular-app -- --snapshot out.png fixtures/kitchen-sink.canvas
cargo run -p specular-app -- --snapshot out.png --snapshot-size 1200x800 \
    --snapshot-camera -400,-900,2 --snapshot-scale 2 FILE.canvas
cargo run -p specular-app -- --script steps.txt FILE.canvas
```

- Pages are synthetic stand-ins. To see the real ones, build with
  `--features cef`, bundle (`crates/specular-cef/scripts/bundle-macos.sh
  debug`, with `CEF_PATH` set) and run the bundle's inner binary with
  `--source cef` before `--snapshot` or `--script`. The run waits for every
  page to load and paint, a script's input reaches an entered page, and
  `wait ms` lets pages run for that long. See `README.md`.
- The camera is `fit` (the default) or `x,y,zoom`: the pan in screen
  pixels, so canvas point `(cx, cy)` at the top-left is `-cx*zoom,-cy*zoom,zoom`.
- A script is one step a line, in the testkit's words: `click x y`,
  `double-click x y`, `triple-click x y`, `move`, `press`, `drag-to`,
  `release`, `drag x1 y1 x2 y2`, `hold shift+cmd` (`hold none`),
  `key cmd+z`, `type some text`, `compose にほ`, `commit 日本`,
  `wheel dx dy`, `pinch 0.2`, `tool shape`, `select id ..`,
  `act annotate-selection`, `act resolve-comment`, `act page-back`
  (`-forward`, `-reload`, `-stop`), `act zoom-to-fit`, `act arrange-row`
  (`-column`, `-grid`), `act focus-selection`, `right-click x y`,
  `sidebar on|off`, `camera ..`, `wait ms`,
  `snapshot out.png`, `save out.canvas`. Positions are screen pixels. A snapshot between `press` and `release` shows a gesture in
  flight.
- The toolbar and the item popup are drawn and take clicks, so the top 44
  pixels are the toolbar. `control shape.color` clicks a control by its
  name wherever it is (`hover-control`, `press-control` likewise), and a
  wrong name fails the run with the names that are shown. A field takes
  `control page.url`, `key cmd+a`, `type ..`, `key enter`. A dropdown's
  options have names once it is open. `panels off` runs without them.
- The clipboard and the Documents a run makes are kept in memory:
  `clipboard some\ntext` is another app copying, `key cmd+v` pastes it,
  and `tool document` then a click makes `Untitled Note.md` with no file.
  `save` writes what an autosave would.
- `fixtures/scenarios/` holds whole sessions as scripts, with a `run.sh`
  that runs them all into `runs/qa/` and checks the canvases they save.
  Run it after a change that crosses features, and add to it when a bug
  only showed up in use.
- `fixtures/kitchen-sink.canvas` has every kind in every style. Snapshot it
  after changing `specular-scene` or `specular-compositor` and open the PNG
  with the Read tool. A scene snapshot test cannot see a wrong colour or a
  hole in a fill.

## Driving it from outside

The running app answers the Electron app's HTTP routes, so the `specular`
CLI and the skill file work against it. It listens on 29979 and writes
`~/.specular/specular-mcp.json` unless the Electron app has them. Then it
logs the port it took and the `SPECULAR_DISCOVERY_FILE=` to pass the CLI.

- A route is an arm in `specular_api::Api::route` and a handler returning
  `Step`. A read answers from `&App`. A write returns an `ApiRun`: an
  `Action` the window already has, or one `Command` for one undo step.
- Test a route in `specular-api/tests/api/` through `Scripted`, which runs
  the planned event through `update`. `specular-app/src/api/tests.rs` is the
  same over a real socket.
- A route the CLI can call and this app lacks gets a row in `unported.rs`,
  so the caller reads which verb is missing instead of a 404.
- The browse verbs (`snapshot`, `click`, ...) reach a page through its own
  CDP websocket, which `GET /pages/<id>/cdp-target` names. The routing is
  `specular_api::cdp::PageProxy`, the sockets are `specular-app/src/cdp/`,
  and `fixtures/scenarios/cef/cli-pages.sh` runs every verb against a
  bundled debug CEF app on a scratch space. `fixtures/scenarios/cef/sync.sh`
  runs a sync set of two widths on the same bundle.

## Gate

Run from `native/` before every commit:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The workspace lints are the contract: no `unwrap`/`expect` outside tests,
`#[expect(lint, reason = "...")]` instead of `#[allow]`, docs on public
items, `// SAFETY:` on every `unsafe` block. `thiserror` in libraries,
`anyhow` only in binaries. Declare each dependency once in
`[workspace.dependencies]`. Keep files under about 400 lines.

## Working a task from the plan

You are one of several agents working through the task list, each with a
fresh context. Another agent may be working in this checkout at the same
time on different files.

- Do exactly one task, the one you were given. Read its entry in the plan,
  the run log's "Decisions" section, and the code it builds on.
- Make design choices that serve the end-state architecture, not the
  shortest diff. If the plan is silent or wrong about something, pick the
  simplest option consistent with the architecture, record it under
  "Decisions" in the run log, and keep going.
- Do not use `--release`, do not run `bundle-macos.sh`, and do not start or
  kill any `specular-app` process. A benchmark may be running.
- Commit only your own files, staged by explicit path (never `git add -A`
  or `git add .`). Conventional message, scope `native`. Do not push, open
  PRs, or switch branches.
- If the gate fails in a crate you did not touch, wait a minute and retry
  once; the other agent may be mid-edit. If it still fails, note it in the
  log and gate your own crates with `-p`.
- When done, append a log entry: task id, commit sha, what exists now, what
  the next agent must know, anything that needs a human at a Mac. Keep it
  under 15 lines. Commit the log with your work.
- If you cannot finish, commit what compiles, write `BLOCKED` and the
  reason in the log, and stop.
- End your final message with one line: `TASK <id> DONE <sha>` or
  `TASK <id> BLOCKED <reason>`.
