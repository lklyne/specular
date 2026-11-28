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
- `specular-sync` owns `CanvasSync`, a yrs mirror of one canvas's
  `Document` for editing it from several places at once (ADR 0048).
  Local commands go out as yrs updates and peers' updates come in as
  commands. Interact's `shared` module holds one per shared canvas in
  place of the plain `History`, and the shell carries `Effect::ShareSend`
  (no transport yet).
- `specular-scene` owns the `Scene` display list and `view`, one module per
  kind under `src/view/`.
- `specular-web` is the view-and-comment web viewer (ADR 0049): an `App`
  driven by a browser, answered from a published bundle, drawn by the
  compositor on WebGPU or WebGL2. Its rules (`Viewer`) are pure and tested
  natively; the browser half (`web.rs`) builds only for wasm32. Keep the
  crates it uses free of `std::time::Instant` (use `web_time`), threads
  and the file system. Its fonts are `specular-compositor/fonts/web`, made by
  `subset.sh` there: run it again when a bundled font changes.
- `specular-api` turns an HTTP request and `&App` into a read answer or an
  `Event::Api`, so an agent's write goes through `update` like a key press.
  The shell hosts the server (`specular-shell/src/api/`).
- `App` owns one `Space`: every canvas of the open folder, each with its
  own `Document`, `History`, camera and selection. `app.document` is the
  active canvas's. Canvas operations are `Action::Canvas`, and the file
  work comes back as effects.
- Only the shell and the CEF crate do I/O. Everything else is testable with
  no window, GPU or CEF.
- `specular-cef-helper` is the executable in the bundle's five
  `Helper*.app`s, which Chromium's child processes run. It depends on `cef`
  alone. Keep the app's crates out of it: each one linked in is paid for
  five times.
- `specular-shell` is the shell. Its library is the `Runtime`, which runs
  every effect, the command line and the headless modes; its binary
  `specular` is the GPUI Kit window (ADR 0040, `src/kit/`): the Kit draws
  the chrome and the sidebar from the models, and the compositor draws the
  canvas under it. A headless run is a `Runtime` over an offscreen window
  (`src/headless/`), so a new effect gets its runner in
  `specular-shell/src/app/` once, for both.

The Electron app was deleted in the big run. Its source is in git history
at `bc5253fa` (the last `main` commit before the run): read a file with
`git show bc5253fa:src/main/<path>`, or list them with
`git ls-tree -r --name-only bc5253fa src`. It is a behavior spec, not a
structure spec: read it to learn what a feature did, and do not copy its
layering (IPC, broadcasts, stores, registries).

## Adding a feature

A feature is one vertical slice. It touches the same places every time:

1. `specular-doc`: the `Kind` variant or fields, its `Command`s, its
   `.canvas` read and write.
2. `specular-interact`: the `Tool` or `Gesture` arm and the hit-test arm.
3. `specular-scene`: the `view` arm.
4. One behavior check: a scenario in `fixtures/scenarios/` that drives the
   gesture, checks the state with `expect` lines and ends with
   `undo-to-start`. See "Tests" below for what else earns a test.

Adding an enum variant should make the compiler list every `match` that
needs a new arm. Do not add wildcard arms over `Kind`, `Tool` or `Gesture`.

### Tests

The suite stays small on purpose. A test has to clear four bars: it
catches a regression you can name, asserts on an observable outcome,
survives a refactor, and reads on its own.

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
- A case in `fixtures/scenarios/cli/cases.py` for a new route or verb.
- No unit tests on trivial helpers, none that restate the type system,
  and none that pin how a private function is called. If a pure rule is
  hard enough to get wrong (resize math, text segmentation, markdown), it
  gets one or two tests on its inputs and outputs.
- A behavior across features, or a bug that only showed up in use, is a
  scenario in `fixtures/scenarios/`: a script that checks state with
  `expect` lines and ends with `undo-to-start`. `cargo test --workspace`
  runs them all headless; `fixtures/scenarios/run.sh --kit` runs them in
  the window.

Before adding a test, look for the one that already covers the behavior
and extend its table.

### Rust tests and the testkit

Behavior is checked by scenarios. The Rust tests that stay (round trips,
pure rules, readbacks, pins, protocol formats) live in the `tests/`
directory of the crate that owns the rule; a crate's `src/` unit tests
cannot use the testkit on that crate's own types.

`specular-testkit` holds what those tests use and nothing more:

- `TestApp::with_pages(n)` (p1 at (100, 100), p2 at (700, 100), 400x300),
  `with_entities([..])`, `from_document`, `from_canvas(json)`, `empty()`
  and `with_space([("Home", document), ..])`. Entities come from `page`,
  `text`, `shape`, `file`, `drawing` and `group`, each taking an id and a
  rect; `sticky`, `plain_text` and `labelled` also take the text.
  `inside("g", entity)` puts one in a group, `connected(document(..), "e1",
  "a", "b")` adds an edge, and `with_comment(document, comment(..))` a
  comment.
- Input: `press`, `drag_to`, `release`, `drag`, `click`, `double_click`,
  `key`, `chord(CMD, Key::Char('z'))`, `type_text`, `hold(SHIFT)` and
  `let_go`, `tick`, `viewport`, `zoom`, `select`, `act(Action)` and
  `send(Event)`. Text is measured by `FixedAdvance`: 10 units a character
  and 20 a line.
- Reading back: `app()`, `document()`, `session()`, `rect("p1")`,
  `selected_ids()`, `editing_text()`, `caret()`, `take_effects()`,
  `page_reports(..)`, `bind(..)` for a repo, and
  `assert_undo_returns_to_start()`.
- `assert_scene_snapshot!(app)` (`specular-scene/tests/`) prints one line
  per display-list item in paint order, for a new draw rule.

### Scenario vocabulary

What a script can name, beyond the steps in "Looking at what it draws":

- Controls are pressed by the models' names (`control text.color`); no
  panel is drawn headless, and the Kit window clicks the control where it
  draws it. A name that is not there fails the run with the ones that are.
- The tab row's controls are `view.canvas` and `view.item.<entity id>`
  (`Action::Show`, ADR 0045); `view.add` (Command+T) makes a page and shows
  its tab with `page.url` being typed in. A tab starts in the Fill lens, or
  in Device from the toolbar's `view.open`; `view.lens.<name>`,
  `view.minimize` and `view.others` sit beside them. In an item view the
  page shown is entered, so send Escape before a canvas key, and a
  Document shown is a reading column that one click edits. See `tabs.txt`,
  `item-view.txt` and `lens.txt`.
- The sidebar starts hidden (`sidebar on`, Command+B, or `sidebar.toggle`,
  in the tab row or, while the sidebar reaches the window's top, in the
  sidebar beside the traffic lights). Its controls are
  `sidebar.canvas.<n>` (by place, from 1; `.name` is the rename field,
  `.menu.rename` and `.menu.delete` its menu), `sidebar.add`,
  `sidebar.open` (Open project), `sidebar.project.<n>` (a project's head,
  by place; `.add`, `.details`, `.reveal`, `.close`, `.git.commit`,
  `.git.ignore`, `.origin.<k>.auto|remove` beside it),
  `sidebar.head.<canvases|notes|pages>`, `sidebar.<notes|pages>.<entity id>`
  (`.toggle` its chevron) and `sidebar.pages.comment.<id>`. The Notes and
  Pages rows are drawn by the `notes` and `pages` section mods, off until
  turned on in Settings > Mods. Shift- and
  Command-clicks on rows widen the selection, from the row picked last.
- A right click opens the context menu for what is under it; its items
  are `menu.<label in lower case, dashes>`.
- A page is select-first (ADR 0022): a click selects it, a second click or
  a double click enters it, and only an entered page gets input.
- A text field is pressed like a control and typed into; Enter keeps the
  text and Escape puts the old value back.
- The window's regions are `tabs`, `toolbar`, `dock`, `sidebar` and
  `chat` (ADR 0055). `run ui.toggle {"id":"toolbar"}` (and `ui.show`,
  `ui.hide`, by a region or a control id) hides one over the layout file,
  and the canvas takes its room; `key ctrl+alt+shift+escape` or `run
  ui.reset` goes back to the shipped layout. `expect layout is region
  toolbar hidden`, `expect layout is toggled toolbar hidden` and `expect
  layout is insets LEFT TOP RIGHT BOTTOM` (what the canvas is seen
  between, `App::insets`) check it. See `regions.txt`.

## Looking at what it draws

`specular --snapshot` draws a canvas into a PNG with no window, on the real GPU
with the synthetic page source. Nothing else is written.

```sh
cargo run -p specular-shell -- --snapshot out.png fixtures/kitchen-sink.canvas
cargo run -p specular-shell -- --snapshot out.png --snapshot-size 1200x800 \
    --snapshot-camera -400,-900,2 --snapshot-scale 2 FILE.canvas
cargo run -p specular-shell -- --script steps.txt FILE.canvas
```

- Pages are synthetic stand-ins. To see the real ones, build with
  `--features cef`, bundle (`crates/specular-cef/scripts/bundle-macos.sh
  debug`, with `CEF_PATH` set) and run the bundle's inner binary with
  `--source cef` before `--snapshot` or `--script`. The run waits for every
  page to load and paint, a script's input reaches an entered page, and
  `wait ms` lets pages run for that long. See `README.md`.
- The camera is `fit` (the default) or `x,y,zoom`: the pan in screen
  pixels, so canvas point `(cx, cy)` at the top-left is `-cx*zoom,-cy*zoom,zoom`.
- A script is one step a line, read by `specular_shell::script`, the one
  module both runners share: this headless one and the Kit window
  (`SPECULAR_SHELL_SCRIPT_FILE`). Its first line may name the canvas
  (`# canvas: ../kitchen-sink.canvas`), and `given` lines may describe the
  starting document instead. Input steps: `click x y`, `double-click`,
  `triple-click`, `right-click`, `move`, `press`, `drag-to`, `release`
  (`release x y`), `drag x1 y1 x2 y2`, `hold shift+cmd` (`hold none`),
  `key cmd+z`, `type some text`, `compose にほ`, `commit 日本`,
  `wheel dx dy`, `scroll x y dx dy`, `pinch 0.2`, `tool shape`,
  `select id ..`, `run NAME [json]` (a command of the registry,
  `specular_interact::commands()`: `run edit.undo`, `run arrange.lay-out
  {"mode":"row"}`), `sidebar on|off`, `camera ..`, `wait ms`,
  `motion on|off`, `reduce-motion on|off`, `page-opens-tab p1 URL
  [background]`, `control NAME`, `hover-control`, `press-control`. Positions
  are screen pixels.
- Checks: `expect ..` states a fact about the app (tool, selection, focus,
  editing, showing, counts, an entity's kind, rect, text or parent, the
  camera) or about the canvas as a save would write it (`expect saved
  /nodes/s1/color = "4"`, `same-as start`, `changed-from start at PATH`).
  `undo-to-start` undoes until nothing is left and checks the canvas is
  the one the run opened. `mark NAME`, `reopen` and `state out.txt` (every
  fact, in `expect` words) help write them. The vocabulary is in
  `fixtures/scenarios/README.md` and `script/expect.rs`.
- Output steps: `snapshot out.png`, `save out.canvas`, `state out.txt`.
  Steps only a window can do (`resize`, `resize-drag`, `keycode`, `swipe`,
  `pan`, `drop`, `menu-choose` ..., `script/window.rs`) are skipped headless
  and listed when the run ends.
- No panels are drawn headless: `control NAME` does what the control does
  from the models (`specular_interact::named_controls`, `Event::Control`),
  an option whether or not its list is open, a sidebar row whether or not
  it is in view. Canvas rows are named by place, `sidebar.canvas.1`.
- The clipboard and the Documents a run makes are kept in memory:
  `clipboard some\ntext` is another app copying, `key cmd+v` pastes it,
  and `tool document` then a click makes `Untitled Note.md` with no file.
  `save` writes what an autosave would.
- `fixtures/scenarios/` holds whole sessions as scripts, one flow a file,
  and `fixtures/scenarios/run.sh` runs them (`--kit` in the window, `--cef`
  on real pages, `--cli` the CLI end to end) into
  `runs/scenarios/<mode>/`. `cargo test --workspace` runs every headless
  scenario too (`specular-shell/tests/scenarios.rs`; `SCENARIO=name` for
  one). Add to them when a bug only showed up in use.
- `fixtures/kitchen-sink.canvas` has every kind in every style. Snapshot it
  after changing `specular-scene` or `specular-compositor` and open the PNG
  with the Read tool. A scene snapshot test cannot see a wrong colour or a
  hole in a fill.

## Driving it from outside

The running app answers the Electron app's HTTP routes, and the `specular`
CLI (`crates/specular-cli`, binary `specular-cli`, linked onto the `PATH` as
`specular` by the setup dialog) drives it. It listens on 29979 and writes
`~/.specular/specular-mcp.json` unless the Electron app has them. Then it
logs the port it took and the `SPECULAR_DISCOVERY_FILE=` to pass the CLI.
`cargo run -p specular-cli -- canvas` runs a verb from the checkout.

- A route is an arm in `specular_api::Api::route` and a handler returning
  `Step`. A read answers from `&App`. A write returns an `ApiRun`: an
  `Action` the window already has, or one `Command` for one undo step.
  JSON the CLI decodes is typed once in `specular-api/src/wire.rs`.
- A verb is tested end to end: `fixtures/scenarios/cli/run.sh` builds the
  CLI and the Kit app, starts the app on a scratch space with a discovery
  file of its own, and runs `cases.py`, which asserts on each verb's output,
  exit code and the saved canvas. Add a case there for a new verb or route.
  `specular-api/tests/api/` keeps what the CLI cannot reach (the page
  proxy, `Api::plan`, a write during a drag); `specular-shell/src/api/tests.rs`
  is the transport over a real socket.
- What the app can do by name is one table, the command registry
  (`specular-interact/src/commands/`, ADR 0055): the menus, the palette,
  the context menu, the scripts' `run` step, `GET /commands`, `POST /run`
  and the CLI's `commands` and `run` read it. A new command is a row
  there; its label, if a menu shows it, is written there once.
- What a mod reads is the keyed state (`specular-interact/src/state.rs`):
  `panels/toolbar`, `panels/dock`, `theme`, `keys`, `layout` and the rest,
  as JSON from `&App`. `GET /state` (`?key=` for one) and `specular state
  [key]` answer it; the Kit window publishes it into a `Revisions` whose
  per-key revision moves only when the value does. The camera is in no
  key. Layouts are `<app home>/layouts/<name>.json` (ADR 0059): trees of
  rows, columns, stacks and blocks with the look at the root, solved by
  `specular_interact::chrome::solve`. The shipped ones (`default`,
  `classic`, `studio`) are rewritten on every start; any other file is the
  person's, and `layout.switch` moves between them. `<app home>/keys.json`
  rebinds keys (`bindings/keymap.rs`, read in `app/keys_run.rs`, a mistake
  in `keys.error`). The files are watched by
  `specular-shell/src/app/layout_run.rs`.
- A page whose address is a file inside the space folder gets the page
  bridge, `window.specular` (`state`, `run`, `commands`, `subscribe`):
  the check is `specular_interact::bridge_allowed`, the messages are
  `specular_api::bridge` (the `/state`, `/commands` and `/run` routes
  planned without a socket), the transport is `specular-cef/src/bridge_*`
  and `specular-shell/src/app/bridge_run.rs`. Sample:
  `fixtures/bridge/space/widget.html`, run by `fixtures/scenarios/cef/bridge.txt`.
- A route the CLI can call and this app lacks gets a row in `unported.rs`,
  so the caller reads which verb is missing instead of a 404.
- The browse verbs (`snapshot`, `click`, ...) reach a page through its own
  CDP websocket, which `GET /pages/<id>/cdp-target` names. The routing is
  `specular_api::cdp::PageProxy`, the sockets are `specular-shell/src/cdp/`,
  and `fixtures/scenarios/cef/cli-pages.sh` runs every verb against a
  bundled debug CEF app on a scratch space (`CLI=` names the CLI binary).
  `fixtures/scenarios/run.sh --cef` runs it with `cef/sync.txt`, a sync
  set of two widths, on the same bundle.
- An agent cursor is drawn when the CLI runs the patched agent-browser:
  `export AGENT_BROWSER_PATH="$(scripts/build-agent-browser.sh)"`. It sends
  `Specular.willAct` before each targeted verb, `PageProxy` holds the
  answer, and `specular-interact/src/presence.rs` releases it when the
  cursor is there (ADR 0046). `RUST_LOG=specular_shell::app=debug` logs each
  act and its go.
- The canvas verbs move the same cursor: `specular-interact/src/api.rs`
  compares the document before and after a write whose caller sent a
  session id (`x-specular-session-id`) and `presence/canvas.rs` sends the
  cursor to what changed. A route needs no presence code of its own.
  `POST /session/presence` with a `labelKey` (`scan_workspace`, `thinking`,
  `departing`, ...) is `ApiRun::Report`.
- Who is on the canvas is `App::agents()` and the `agents` state key
  (`presence/roster.rs`); `view.follow` is `Action::Follow`
  (`presence/follow.rs`). A camera move the person makes, or Escape, ends a
  follow.

### Mods

A mod (ADR 0055) is a folder of `mod.json` and `main.js`, run by
`gpui-shell` in `crates/specular-mods`. The run log is
`docs/plans/mods-run.md`.

- **Where they live.** Mods are in `<app home>/mods/<name>/`. The shipped
  ones live in `crates/specular-mods/mods/`, with the guide
  (`mods/README.md`) and `specular.d.ts`. All are written there on every
  start, except a folder whose `mod.json` has `cloned_from`. Each gets a
  `.shipped` marker of what was written, so a folder edited in place is
  moved to `<name>-edited` as a clone first (`Event::ModsSetAside`, a
  notice), and a mod no longer shipped is removed only while its folder
  is untouched (`src/retired.rs`; mods
  retired before the marker are listed there by hash). A new command
  or state key must be named in both the guide and the d.ts; the test in
  `tests/shipped.rs` fails otherwise.
- **How a mod is placed.** A layout block's `content` names a mod
  (`{"mod": "sidebar", "props": {..}}`) or one of the Kit's own panels
  (`{"panel": "sidebar"}`), and the root names the overlays (`dock`,
  `menu`, `share`, `toast`, `palette`, `welcome`) the same way; every
  native panel stays as the fallback. `overlay.open` opens a copy of a mod
  over the window (`specular-interact/src/overlays.rs`, drawn by
  `kit/view/overlay_layer.rs`). `ModSet` loads the mods, publishes
  `mods/<name>/props`, writes `<name>.error` and renders a mod only when a
  key it read moved. The window's side is `src/kit/mods.rs`,
  `src/kit/mods/host.rs` and `src/kit/view/mod_bar.rs` (where each mod is
  drawn); the menu mod is placed in `view/canvas_menu.rs`.
- **The shipped mods.** `default` draws every panel with a mod:
  `toolbar`, `tabs`, `share`, `toast`, `sidebar`, `dock`, `palette`,
  `chat` and `menu`; `welcome` is opt-in; `confirm` and `prompt` are dialogs any mod
  opens, `status` is `studio`'s status bar, and `star` is the example
  mod shape. A text field in a mod is the `Field` host component
  (`kit/mods/text_field.rs`), its text on `shared/field/<id>`.
- **What a mod can call.** `run` resolves through `run_named` and the work
  is deferred out of the script call. Rust-drawn pieces are host
  components (`Components`): `Icon`, `Zoom`, `ToolOptions`, `Favicon`,
  `WindowDrag`, `Share`, `Prism`, `Field`, `Composer`, `Material`. A layout node's
  `material` and `Material` are system views under GPUI
  (`kit/view/material.rs`, ADR 0061). Headless, the window-only ones are
  placeholders. A new one goes in `HOST_COMPONENTS`, the d.ts and the
  guide; the window's version in `kit/mods/host.rs`.
- **Canvas items, tools and commands (ADR 0057, API 1.1).** The app's
  side is `specular-interact/src/mods/`: the catalog the shell scans
  (`Event::ModCatalog`), `ModAction`, the `canvas/mod_items`, `tool` and
  `call` keys, the install notice. A mod item is `Kind::Mod`
  (`specular-doc/src/mod_item.rs`, `mod_draw.rs`) drawn by
  `specular-scene/src/view/mod_item.rs`; the shell's side is
  `app/mods_run.rs`. Mods with items, tools or commands run out of sight
  (`ModSet::sync`'s `background`, `Mods::background`). The shipped
  examples are `feed` and `watercolor`. A tool's `stroke` (API 1.6) is
  drawn by the scene from `App::mod_trail`, and `publish_live` sends the
  `tool` and `call` keys each turn, outside the models gate.
- **The whole window (ADR 0059, API 1.2).** The state keys
  `panels/sidebar`, `panels/chat`, `selection`, `canvas/items`,
  `canvas/info`, `layouts`, `look` and `overlays`
  (`specular-interact/src/state/panels.rs`), the `chat.*` and
  `overlay.*` commands, and a mod tool with `"group": "shape"` listed in
  the shape tool's options (`panel/dock/shape.rs`).
- **Mod shapes (ADR 0060, API 1.4).** `shapes` in `mod.json` give
  `ShapeKind::Custom` shapes (`specular-doc/src/custom_shape.rs`) that the
  shape tool, the dock (`panel/dock/shape.rs`) and `view` treat as any
  shape; `shape.set` (`specular-interact/src/mods/shape.rs`) reshapes them.
- **Threads anywhere (API 1.3).** `chat/threads` and a `chat/thread/<id>`
  key for each thread of the active canvas
  (`specular-interact/src/state/chat.rs`), published beside the fixed
  keys (`is_state_key` names both); `chat.new-thread {id, open}`,
  `chat.send {thread, images}` (starting a thread not made yet) and `chat.stop {thread}`. The panel's composer is the `Composer` host component (`kit/mods/composer.rs`, on `view/chat/composer_box.rs`), and web pages load `mods/lib/chat.js` (`<specular-chat>`, `<specular-thread>`, `<specular-composer>`). A mod command with
  `"chat": true` is a button in the agent panel's header (`HeaderAction`,
  `mods::chat_buttons`), and the `thread` mod makes a thread a web canvas
  item.
- **Checking and measuring.** `specular mod check <dir> [--state
  fixture.json]` (the `specular-mod-check` crate) renders a mod headless.
  `specular mod stats` is `GET /mods`. `specular mod clone <name>
  [new-name] [--use]` copies a mod to one the person owns. `specular shot
  [file.png]` is `POST /window/screenshot`, the canvas as a PNG.
  `specular --skill` prints `crates/specular-cli/src/SKILL.md`, the manual
  the short `specular-mods` skill points agents to. The gates are
  `cargo test -p specular-mods --release --test toolbar` and `--test
  panels` (`-- --nocapture` for the numbers): median under 2 ms, none
  over 8 ms of thread CPU, no render while idle or on a camera move, every id the
  native model lists, a click reaching the host. A new panel's gate goes
  in `tests/panels/`, on `tests/support/`.

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
  kill any `specular` process. A benchmark may be running.
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
