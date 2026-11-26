# Mods run log (ADR 0055)

The build of [ADR 0055](../adr/0055-chrome-as-files-then-scripted-panels.md)
(the ADR itself is in PR 457), one section a phase: what was built, what each
phase's end test measured, and the calls made without asking.

## Phase 0: the spike

**Result: go.** `gpui-shell` 0.11 runs inside our GPUI pin with no fork and
no patch, and a scripted toolbar meets phase 3's numbers by a wide margin.

What was built: `crates/specular-mods`, the mod host. It loads a mod folder
(`mod.json` + `main.js`) on a `gpui-shell` runtime of its own, gives it the
`specular` host module (`state`, `props`, `shared`, `setShared`, `run`,
`commands`, `notify`), and records which keys each mod read so it renders
again only when one of them moves (`Reads`, `Revisions`, a port of Herdr's
`publish` and `LiveReads`). `tests/toolbar.rs` loads a toolbar mod over a
`panels/toolbar` fixture in a headless GPUI window and measures it.

Numbers, all from `cargo test -p specular-mods --release --test toolbar --
--nocapture` (Linux x86_64, 4 cores, GPUI's headless test platform; CI runs
the same test on macOS):

| | Release | Debug |
|---|---|---|
| Load and first render | 11 ms | 36 ms |
| Script render, median of 300 | 0.08 ms | 1.1 ms |
| Script render, p95 | 0.13 ms | 1.2 ms |
| Script render, slowest | 0.40 ms | 1.7 ms |
| Materialize, median | 0.03 ms | 0.19 ms |
| Script renders in 100 idle frames | 0 | 0 |
| Render when an unread key moves | none | none |

Phase 3's gate is a median under 2 ms and none over 8 ms; the release build
is 25x and 20x inside it.

Build cost, measured on Linux with a bare GPUI Kit window against the same
window with `gpui-shell` (release, 4 cores):

- Build time: about 5.5 minutes added to a clean release build. Nothing is
  added to an incremental build of our crates.
- Binary size: 28 MB to 64 MB stripped (+35 MB). Most of it is the runtime
  itself (110,000 lines, with GPUI's inspector reflection that it turns on
  for the script's style surface), the QuickJS JIT, and the `fetch` and
  `websocket` modules' HTTP and TLS stack. The macOS figure will differ; it
  is worth reading off the first release build.

Patches we would need upstream: none to run. Two would be nice to have,
neither blocking:

- The budgets (render 2 s, event 5 s) are private. Ours are measured instead
  (`ModStats`, `specular mod stats`), as the ADR planned.
- The engine tier is private. Debug builds stay on the interpreter; release
  builds tier hot functions up to the JIT. Upstream notes the JIT crashing on
  Linux and Windows under many short-lived runtimes in debug, which is why
  debug stays on the interpreter. Specular is macOS only and holds a handful
  of long-lived runtimes, so this is a watch item, not a stop.

Findings that change the ADR's text:

- `default-features = false` drops the JavaScript engine too: `quickjs` is a
  default feature. The pin is `default-features = false, features =
  ["quickjs"]`.
- `gpui-shell`'s manifest file is `gpui-shell.json`, not `mod.json`, so ours
  sits beside it with no clash. A mod has no `gpui-shell.json`; the runtime
  loads `main.js`.
- Host functions do not know which script called them. Each mod gets its own
  instance of the `specular` module (and its own runtime), which is what
  lets a read be recorded against the mod that made it and gives per-mod
  numbers.
- The host owns hot reload: `ShellRuntime::watch` reloads a mod on save and
  keeps the old build running when the new one fails, which is the ADR's
  "a mod that fails at load keeps its old build".

Call made: the mod host is its own crate, `specular-mods`, rather than a
module inside `specular-shell`. The Kit window builds only on macOS; this
crate builds and tests on Linux too, so the mod host is checked headless
everywhere. The ADR's rule still holds: the command registry and the keyed
state live in `specular-interact` with no runtime dependency, and a change of
runtime touches `specular-mods` and the shell alone.

## Phase 1a: the command registry

**Built.** One table of everything the app runs by name, in
`specular-interact/src/commands/` (pure, no runtime dependency):
`CommandSpec { name, label, args, about, enabled, build }`, `Run { Action,
Shell, Api }`, `RunError`, and `commands()`, `command(name)`,
`run_named(app, name, args)` (an unknown name is refused with up to three
nearest names, bad arguments with the command's `args` line, a command with
nothing to act on as disabled, and `control.press` of a control that is not
there with the names that are). `Run::into_event` turns a run into the
`Event` a runner sends; a `Shell` run is the window's.

Its readers, with their own name tables deleted:

- Menus, the context menu and the palette take label and action from the
  registry by name (`menu.rs` lists names, `palette.rs` `EXTRAS` lists names,
  the shell group's labels are found by `Run::Shell`). `ShellCommand::label`
  is gone. A dump of `palette()`, `menus()` and both context menus for the
  same app was byte-identical before and after.
- Scripts: `act WORD` is replaced by `run NAME [json]`, resolved at its
  turn through `run_named` in both runners (headless `Step::Run`, Kit
  `Do::Run`). `words.rs`'s `action_named` and `facts_canvas/step.rs`'s name
  table are deleted; the tool words are `Tool::name`/`Tool::named`, which
  the toolbar's control ids use too. Every scenario file was rewritten
  mechanically (`act undo` to `run edit.undo`, `act arrange-row` to
  `run arrange.lay-out {"mode":"row"}`, `act show:h1` to
  `run tab.show {"item":"h1"}`, ...). New fact: `expect command NAME
  enabled|disabled`. New scenario: `commands.txt`.
- HTTP: `GET /commands` and `POST /run {name, args}` (`specular-api/src/commands.rs`),
  a write through `Event::Api` like the others; statuses 404 unknown name or
  control, 400 bad args, 409 disabled or a window command.
  `specular-api/tests/api/commands.rs` drives both.
- CLI: `specular commands` and `specular run <name> [json-args]`, with a
  case in `cli/cases.py`.

The names (93): `app.quit open-space open save settings hide hide-others
minimize command-palette`; `canvas.new duplicate delete switch rename
begin-rename`; `edit.cancel undo redo cut copy paste duplicate delete
select-all select nudge`; `text.bold italic code strikethrough bullet-list
numbered-list task-list body heading-1..6`; `arrange.bring-forward
send-backward bring-to-front send-to-back group ungroup auto-layout lay-out
group-layout group-gap`; `comment.annotate-selection resolve delete focus
reveal`; `page.new-tab edit-address back forward reload stop navigate frame
frame-color rotate scheme orientation toggle-sync`; `tab.show open close
next previous lens others`; `tool.set ellipse diamond highlighter`;
`view.zoom-in zoom-out zoom-reset zoom-to-fit zoom camera reveal theme
toggle-sidebar`; `share.copy-link access join stop`; `control.press`.

Left out, and why:

- `PrepareShow` (a tab press before its click), `PageEdit` (what an Edit
  item becomes inside an entered page; the menus still make it), and
  `Sidebar(Section | Row)` (folding a sidebar row is reached by
  `control.press` on its id).
- `SetToolDefault`, `SetProperty` except orientation, and the
  `SetToolVariant` patches other than the palette's three: their argument
  shapes are the dock's whole property surface. A mod presses the dock's
  controls with `control.press` instead.
- `Chat`, `Repo`, `Space`, `Setting`: the right panel's, the repos pane's,
  the first-run view's and the settings' own actions, each already reached
  by `control.press` where a control shows them.

Calls made:

- `CommandSpec` has a `label` beside `about`: `about` is a sentence, and a
  menu needs a title. Commands no menu or palette shows have an empty label.
- `enabled` is "has something to act on" (`has_target` and no drag), not the
  menus' key context: a command run by name is not a key press, so an
  entered page does not disable `edit.copy` for an agent. The menus keep
  computing their own enablement with the context.
- Script `run` checks `enabled`, so five scenario lines that ran a command
  to show it did nothing (`page.reload` with no single page, a lone
  `arrange.lay-out`, `page.toggle-sync` over an HTML file, a fourth undo)
  became `expect command NAME disabled` or were dropped.
- `control.press` is a new `Action::PressControl(ControlId)`, run through
  `panel::activate_control`, so it goes through `update` from every door
  (script, palette, `POST /run`) without `Action` being serializable.
- `Run::Api` is used where no `Action` says the change: `share.join`,
  `share.stop`, and `page.navigate` with a `page` id. All three variants
  of the ADR's `Run` have users, so none was left out.
- Over HTTP a `Shell` command is refused (409): the server has no window to
  hand it to. A mod in the window can run it (`Run::Shell` goes to
  `menus::run_shell`, as the Kit's script runner does).
- Not changed: the settings' shortcut list still names unlisted bindings
  with its own few words (`unlisted` in `settings.rs`, e.g. "Bulleted list"
  where the palette says "Bullet list"). Moving it onto the registry changes
  visible labels, so it is left for a later pass.

Not verified here (Linux): the Kit window's `Do::Run` (`kit/debug_input/`
builds only on macOS); `fixtures/scenarios/cli/run.sh`, which needs the
Kit app (the Linux binary has no window and no server); `cef/sync.txt` and
`cef/pages.txt`, whose `run page.back` / `page.reload` / `page.toggle-sync`
now must be enabled when they run. Two compositor GPU readback tests
(`a_clip_cuts_an_item_to_its_rect`, `opacity_blends_an_item_with_what_is_under_it`)
fail on this box's software Vulkan driver with and without this change.

## Phase 1b: keyed state, the layout file, insets, the reset key

**Built.**

- **Keyed state** (`specular-interact/src/state.rs`, pure): `state(&App)`
  gives eleven keys of JSON, `state_key`, `STATE_KEYS`, and
  `publish_state(&mut Revisions, &App)` returns the keys whose revision
  moved. `Revisions` moved here from `specular-mods/src/reads.rs`
  (`specular-mods` depends on interact and re-exports it; `Reads` stays
  there). The panel models derive `Serialize`; every `Action` field is
  skipped, the `ControlId` written beside it is what `control.press` takes.
  Keys and shapes:
  - `panels/toolbar`: `ToolbarModel` without `share` (its own key):
    `{chat, theme, sections: [{tools: [..]}, {zoom: {id}}], tools, view}`.
  - `panels/tool_options`, `panels/dock`: `{controls: [{kind: button|toggle|
    swatches|dropdown|stepper|field|choices|separator, id, ..}]}` or `null`
    (the dock is `null` while a tool has options, as the window shows it).
  - `panels/strip`: `{tabs: [{id, label, icon, active}], add}`.
  - `panels/menu`: `{at: [x, y], menu: {controls}}` or `null`.
  - `panels/palette`: `[{label, group, command, chord, enabled, checked,
    keywords}]`, `command` the registry name.
  - `panels/share`: the Share button and its access dropdown.
  - `theme`: `{appearance: light|dark, choice: system|light|dark}`.
  - `settings`: `{showSidebar, showChat, tools, hideAgentCursors,
    hideSyncedCursors}`.
  - `keys`: `[{keys: "⇧⌘Z", context, command, args}]`, the binding table,
    then the window's keys, then `{keys: "⌃⌥⇧⎋", command: "ui.reset",
    fixed: true}`.
  - `layout`: `{file: <the layout>, toggles: {hidden: {id: bool}}, hidden:
    [regions hidden]}`.
- `GET /state` (`?key=` one, 404 with the keys for an unknown one),
  `specular state [key]`, a `cli/cases.py` case and an API test. The Kit's
  `Canvas` owns a `Revisions` and publishes into it each time it reads the
  models; nothing reads it until the mod host does.
- **The layout file** (`chrome/layout.rs`): `{tabs, toolbar, dock, sidebar,
  chat}`, each `{"mod": name|null, "hidden": bool, "props": {..}}`; a
  region left out is the app's own, unknown keys are errors, `props` is
  carried untouched and its `hidden` lists control ids. `Layout::parse`
  returns `LayoutError`; `LayoutFile` keeps the last good layout past a bad
  read. On disk: `<app home>/layouts/default.json` (the folder that holds
  `preferences.json`: `~/Library/Application Support/Specular` on macOS,
  `SPECULAR_NATIVE_CONFIG_DIR` when set), written with the shipped layout
  on every start, polled every 500 ms (`watch::Polled`), a bad save keeps
  the last good one and writes `default.error`; toggles are
  `layouts/.state/default.json` (`specular-shell/src/app/layout_run.rs`).
  Only a runtime with settings on (the window) touches these; headless runs
  start on the shipped layout.
- **Commands**: `ui.toggle {id}`, `ui.show {id}`, `ui.hide {id}` (a region
  name or a control id) and `ui.reset`, as `Action::Layout(LayoutAction)`.
  A toggle equal to what the layout says is dropped, so `.state` holds only
  differences. `Effect::SaveLayoutToggles` and `Effect::ResetLayout`.
- **Insets** (`chrome.rs`): `insets(Regions, Shown) -> Insets {left, top,
  right, bottom}` is the one function; `App::insets()` replaces
  `covered_left`/`covered_top`, and `viewport::area`, `named::open_menu`,
  `script/chrome.rs` and the Kit's `view.rs` read it. The region sizes
  (`TAB_ROW`, `DOCK_ROW`, `CHROME_HEIGHT`, `TOOLS_DOCK_WIDTH`,
  `SIDEBAR_WIDTH`) live in `chrome.rs` alone, re-exported at their old
  paths. The shipped layout gives exactly the old numbers (every existing
  scenario passes unchanged).
- **The reset key**: Control+Option+Shift+Escape (not Command), checked
  first in `bindings::on_key`, before the editor and an entered page; it
  runs `ui.reset`.
- **Tests**: `fixtures/scenarios/regions.txt` (toolbar, tabs, sidebar,
  dock toggled with the insets checked, a control id, the reset key,
  `ui.reset`, `undo-to-start`); new `expect` topic `layout` (`region NAME
  shown|hidden`, `toggled ID hidden|shown`, `insets L T R B`);
  `tests/it/layout.rs` (round trip with props, bad saves keep the last
  good); `tests/it/state.rs` (publishing twice moves nothing; a pan and a
  zoom move no key, a tool change moves `panels/toolbar`).

Calls made:

- **`Event::Insets` is not an event.** The insets depend on state `update`
  itself changes in the middle of an event (the sidebar opening, a tab
  showing its item, which the camera fits under), so numbers sent by the
  shell would be one event behind. Instead the shell sends the layout
  (`Event::LayoutLoaded`, `Event::LayoutToggles`), the app keeps it beside
  the toggles, and `App::insets()` computes them from it by the one pure
  function both runners use. `Event::ShellChrome`/`ChromeLayout` still say
  whether the chrome covers the viewport.
- The zoom readout is written as `{zoom: {id}}`: its face and its levels'
  checks follow the camera. Whatever shows the zoom will be a host element.
- Hiding `toolbar` hides the tool ribbon (docked or floating); the Share,
  chat and view controls drawn in the tab row go with `tabs`. Hiding
  `sidebar` keeps the sidebar's own open flag (Command+B still flips it),
  and it is drawn only when both say so. `chat` is beside the slot, so
  hiding it widens the viewport the window reports; its inset is 0.
- A control id toggled is kept and published (`layout.toggles`) but the
  app's own panels do not filter by it yet: `props.hidden` is for mods.
- Keys are not a file yet, so the reset key and `ui.reset` reset the layout
  and its toggles only.
- The Kit places the regions under the tab row at `theme::chrome_top()`, a
  thread-local each render sets from the models, the way the theme's
  colours are functions of the theme in force, rather than a new argument
  through five views.

Not verified here (Linux): every Kit edit (`kit/canvas.rs`,
`kit/canvas/models.rs`, `kit/theme.rs`, `kit/view.rs`, `kit/view/toolbar.rs`,
`kit/view/sidebar.rs`, `kit/view/dock.rs`, `kit/view/chat.rs`,
`kit/debug_input/run.rs`); the region hiding in the window; the layout
file's write/watch/error path and the reset key in the window; and the
`cli/cases.py` case, which needs the Kit app.
