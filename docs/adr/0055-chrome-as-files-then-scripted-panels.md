# ADR 0055: The interface as mods: scripted chrome on the Kit shell, pages as canvas extensions

**Status:** Proposed, revised 2026-11-26 after review. Not built.

- The first version (2026-11-25) put the scripts on Rune's Luau lane and went on hold when [ADR 0051](./0051-rune-shell-around-the-compositor.md) was deferred.
- The second moved them to `gpui-shell` on the GPUI Kit shell that stays ([ADR 0040](./0040-gpui-kit-hybrid-shell.md)) and opened the API to tools, canvas items and GPU shaders.
- This one keeps the runtime and the openness, and cuts the scope to what can be built and measured first: the chrome as mods, arrangement as files, and the page bridge. Mod-defined canvas items and tools moved to [ADR 0057](./0057-mod-items-and-tools.md). The `gpu` host module moved to [ADR 0058](./0058-gpu-host-module-for-mods.md). The permission model is dropped: mods are trusted, as Herdr's are.

**Date:** 2026-11-25
**Related:** [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), the panel models. [ADR 0052](./0052-one-runner-one-state-document.md), the state document, which this does not wait for. [ADR 0054](./0054-every-editor-is-an-actor.md), actors. [ADR 0051](./0051-rune-shell-around-the-compositor.md), the deferred Rune shell the first version depended on.

## Context

Herdr's desktop app (`herdrdev/herdr-app`) is built so that a person, or their agent, can rebuild its window. Read at its 2026-11-26 head, with its guide (`defaults/AGENTS.md`) and contributor rules (`AGENTS.md`):

- **The interface is a folder of files.** `~/.config/herdr-app` holds `layouts/<name>.json` (the window and its whole look), `keys.json`, `settings.json` and `mods/<name>/` (a `main.js`, a `mod.json`, icons). A save applies at once.
- **Nothing shipped is special.** The sidebar, tab bar, menus, prompt, confirm dialog, toast and pickers are 21 mods, about 9,000 lines of JavaScript, written on the API a user gets. A few of the app's own screens (welcome, settings, palette) are in a `frame/` folder that is rewritten on every start and not meant to be edited.
- **Clone to own.** Shipped layouts and mods are replaced on start. `mod.clone` makes a copy that is the person's and is never updated.
- **It cannot be broken.** A bad file keeps the last good version and writes `<name>.error` beside it. `ctrl-alt-shift-escape`, which nothing can rebind, sets the layout and keys aside and returns to the default.
- **A rule keeps the native core small.** Before anything is added to the core: does the app itself act on it, can only the app see it, can mods not do it correctly? Three noes and it is a mod, "even at the price of some repetition".
- **One way to read, one way to write.** Mods read live data (`spaces()`, `agents()`, `terminals()`, `focused()`, `layout()`, `theme()`, `commands()`) and act through `run(command, args)`, the command table keys, the palette and the CLI use. The table is one 236-line file: `Command { name, args, about }` (`src/commands.rs`).
- **Narrow re-renders, in about 50 lines.** `publish(key, …)` bumps a per-key revision when a value changes, `LiveReads` records which keys each mod read, and a mod re-renders only when one of those moved (`src/mods.rs`). Only `setShared` re-renders every mod.
- **Mods decide what to show.** A mod is not handed a list of controls. It reads the data and draws what it likes. A layout hides whole nodes, and passes `props` to the mod it places.
- **Mods are trusted.** Herdr grants every mod files, processes and network, and its security doc says so: "treat it like a shell script you would run yourself".
- **Agents are first-class authors.** The guide is written into the app home on every start, a test fails if it misses a command or a setting, and a checked-in `herdr.d.ts` is the full API. `herdr-app check <mod>` loads a mod and renders it once with no window. A mod adds palette entries from its `mod.json`.

What it cost them is in the changelog: window freezes while mods ran, and idle mods waking 20 times a second.

Specular already has parts that are hard to add late: panels as pure functions of `&App`, every change an `Action` or a `Command` through one `update`, and a shell that is GPUI Kit. It does not yet have one command table. That is the first piece of work below.

## Decision

Three lanes and two rules.

**1. The chrome is mods.**

- A mod is a folder: `mod.json` and `main.js`, run by `gpui-shell` inside the Kit shell. It describes GPUI elements; Rust keeps and draws them.
- The shipped toolbar, dock, tab row, menus, context menu, palette, share and toast become mods, written on the API a person's own mod gets. The sidebar, settings and chat follow only if the small panels meet their numbers (see "Phases").
- A mod reads the app through keyed state: `panels/toolbar`, `panels/dock` and the rest are the existing panel models as plain data, beside `theme`, `settings` and `keys`. Raw keys (`selection`, `items`) are added when a real mod needs one, not before.
- A mod changes the app only through `run(name, args)`, which looks the name up in the command registry. A document write is a `Command` with an inverse, so a mod's edit is an undo step like any other. View changes (the tool, zoom, a panel opening) are not undo steps, as today.
- A mod keeps its own state, and may share small values with other mods (`shared(key)` / `setShared`). Both live in the app home, never in the `.canvas` file, so chrome state stays out of the document, undo and sync.
- Script runs on a change to a key it read, an event or a timer, never per frame. The camera is not in the state, so a pan or zoom cannot enter a script. Anything that shows a camera value, such as the zoom readout, is a host element the mod places.

**2. Arrangement, visibility, keys and theme are files.**

- A layout is JSON: which mod sits in which region, sizes, what is hidden, and `props` handed to each mod. A region or a mod is hidden by the layout itself (`"hidden": true` on its node).
- One control inside a shipped mod is hidden through props, and the mod honours it: `{ "mod": "toolbar", "props": { "hidden": ["inspect"] } }`. The host does not filter a mod's element tree. The ids are the `ControlId`s the panels already use.
- `run("ui.toggle", {id})` and the palette do the same at run time. What was toggled is kept in `layouts/.state/`, apart from the layout file, so a click never rewrites a file a person edits.
- Keys and the theme are files beside the layout. Shipped files are replaced on start. `specular mod clone <name>` makes a copy that is the person's, records what it came from, and is never updated.
- A save reloads what changed. A bad layout, keys or theme file keeps the last good version and writes `<name>.error` beside it. A mod that fails at load keeps its old build running; one that loads and then fails in `render` shows an error card in its region until the next save. One fixed key, which nothing can rebind, returns to the shipped layout and keys.

**3. The lightest extension is a page.**

- An HTML file in the space folder is already a live page on the canvas. A page from the space gets a bridge: the same keyed state to read, and `run(name, args)` to write.
- A custom canvas widget is then one HTML file, which an agent can write, on an item that already moves, resizes, syncs and undoes like any other.
- Herdr has nothing like this (no HTML, no web view). It is ours because the canvas already hosts pages. It needs the registry and the state but no script runtime, so it ships before the mods do.

**Rule: nothing shipped is special.** A capability a shipped mod uses is one a person's mod can use. If the chrome needs something mods cannot do, the API grows; the chrome does not get a private door.

**Rule: the native core answers for itself.** Before a feature goes into Rust, answer Herdr's three questions: does the app itself act on it, can only the app see it, can a mod not do it correctly? Three noes and it is a mod.

## Trust

Mods are trusted code, as in Herdr. A mod is something a person or their agent wrote or chose to install, and it has that person's access. There is no capability prompt, no per-mod grant and no list of shipped grants to keep.

**The one exception: content from other people never becomes code.** A `.canvas` file or a web page can never name, load or select a mod, and a page from the web never gets the bridge. This costs one check where mods are loaded and one where the bridge is attached, and it is what lets a person open someone else's canvas safely.

A layout names mods, so a layout from someone else is code, and is read before it is installed like any mod.

A permission model can be added later without changing a mod's code: `mod.json` is our own file and can grow a `capabilities` key, and `gpui-shell`'s `Capabilities` are already set per mod by the host. It is left out until mods are shared between people.

## The runtime

`gpui-shell` 0.11 from GPUI Kit, taken from crates.io with `default-features = false` (the defaults pull in a web view we do not need). It pins `gpui-pre =0.3.14`, the GPUI Specular already pins with `gpui-kit =0.11.0`.

We do not use Herdr's runtime. Theirs is a fork (`crates/herdr-shell`, 99 `HERDR:` patch sites) of `gpui-shell` 0.7.1 on `gpui-pre =0.3.8`, an older GPUI than ours. It is the reference for any patch we turn out to need.

Both enforce limits on script, and they differ:

| | Upstream 0.11 | Herdr's fork |
|---|---|---|
| Render budget | 2 s | 50 ms |
| Event budget | 5 s | 500 ms |
| Startup budget | 10 s | 5 s |
| Heap cap | none | 256 MB |
| Stack cap | yes | yes |
| Engine in release | JIT | plain interpreter |

- Upstream's budgets are private. An embedder cannot set them or choose the interpreter.
- Herdr moved to the interpreter after the JIT's upkeep froze the window for seconds, and found mod code no faster with it.
- Script runs on the UI thread, so a render budget is a frame budget. Two seconds stops a runaway mod. It does not keep the window smooth.

So the limits we rely on are measured, not enforced: `gpui-shell`'s `RuntimeMetrics` gives render counts and time in script per mod, and each phase is gated on them (see "Phases"). Whether we need upstream to expose the budgets and the interpreter switch is what phase 0 finds out.

## The `specular` host module

A mod is JavaScript, but what it can reach is Rust. `gpui-shell` calls these host modules: named imports, written in Rust, that a host gives its scripts.

- We write one: `specular`. It has the keyed state, `run`, `commands()`, `shared` and notifications.
- `gpui-shell` already ships `fs`, `fetch`, `net`, `websocket`, `process` and `os`. Mods use those as they are. It refuses a host module named `net` or `process`, so we could not replace them if we wanted to.
- A value crossing into a host module is JSON (null, bool, number, string, array, object). That is enough for everything in this ADR.

The mod host is one client of the registry and the state, beside the palette, the keys, the HTTP API, the CLI and the page bridge. The registry and the state live in `specular-interact` and do not depend on `gpui-shell`. A pin bump, a patch or a different runtime then changes `specular-shell` alone ([ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md)'s rule).

## `mod.json`

Our own file, parsed by Specular. `gpui-shell`'s own manifest refuses unknown keys, so it cannot carry ours. Herdr does the same.

```json
{
  "api": 1,
  "title": "Toolbar",
  "place": "top",
  "palette": [
    { "title": "Toggle the toolbar", "run": "ui.toggle", "args": { "id": "toolbar" } }
  ]
}
```

- `api` is the mod API generation the mod was written for. The versioned surface is the command names and their arguments, the state keys, and the `specular` module's functions. Within a generation it only grows.
- A mod asking for a newer generation than the app has is not loaded, and gets a `<name>.error` saying so. A break in the surface is a new generation, decided on purpose.
- `place` is where a mod goes when no layout names it. `palette` adds entries, as Herdr's does.

## Why JavaScript, and where Rust stays

Herdr's app is about 37,000 lines of Rust and 9,000 of JavaScript, and the split is by who writes the code and when:

- **The host is Rust, built and shipped once.** Layout, drawing, terminals, input, system access, speed. `gpui-shell`'s own README puts the line here: "the host owns rendering, layout, input and system capabilities; the script owns composition, presentation and business logic".
- **Mods are JavaScript because they arrive after the build.** `gpui-shell` exists so that "a new panel, a side tool or a piece of business logic arrives as a script loaded into the same process — no rebuild, no binary to redistribute, and no fork". It chose JavaScript "because application code reads better in it and the language is more widely known". A save reloads a mod in place.

The same split fits us, and the alternatives do not:

- **Mods in Rust** need a Rust toolchain on the person's machine, a rebuild per change, and either a fork of the app or loading native libraries. Rust has no stable ABI, so a library built against one release breaks on the next, and it cannot hot reload.
- **WebAssembly mods** can be written in any language, Rust included. Nothing binds GPUI elements to WebAssembly today, and an agent writing a panel would compile on every save.
- **JavaScript on `gpui-shell`** is the design Herdr ships, upstream is on the GPUI version we pin, it is the language agents write best, and it reloads on save.

So: Rust for the host, the canvas, the document and host modules; JavaScript for the chrome and what a person or an agent adds to it.

## Tooling for agents

Most of it is in `gpui-shell` already, and we wrap it.

- `specular mod check <dir>` loads a mod with no window, renders it once against a fixture, and prints its element tree with ids. It is `gpui-shell`'s `Plugin::check` and `RenderSnapshot::debug_tree`. The fixture is a JSON dump of the keyed state, which `specular mod state` writes from the running app. A failure exits non-zero and writes `<name>.error`.
- A mod's `console.log` goes to the app's log and to `check`'s output.
- A guide and `specular.d.ts` are written into the mods folder on every start. The `.d.ts` is a checked-in file. A test fails if either misses a command or a state key.
- The guide opens with a one-screen example mod.
- A screenshot of the window is the `screenshot` verb the CLI already has. No second command.
- Render counts and time in script per mod come from `RuntimeMetrics`, and `specular mod stats` prints them.

Left for later: typed argument schemas, and a test that compares the surface with the last release's. The `api` number and the names test come first.

## Alternatives

**The first version: Luau on Rune.** A sandboxed VM with a watchdog and a heap cap built in, and S7 of [the spike](../plans/rune-spike.md) showed a scripted toolbar equal to the Rust one. It depends on the Rune shell, which is deferred, and `rune-luau` needs a C++ build.

**Scripts restyle the controls Rust lists, and nothing more.** The first version's line: Rust decides which controls exist, scripts lay them out. This revision starts close to it, because `panels/*` is the first read API, but does not stop there: a mod may ignore the models and draw from raw keys once they are published.

**A capability sandbox per mod.** The second version's line: an empty grant, a prompt before a mod runs, a CPU budget and heap cap of our own. It needs a fork or upstream changes for the limits, a manifest and a prompt to design, and a grant list to keep, all before one mod has shipped. Herdr ships without it. Dropped for now; see "Trust" for how it comes back.

**Herdr's fork as the runtime.** It has the tighter budgets, the heap cap and the interpreter. It is four minor versions behind upstream and on an older GPUI, so adopting it means porting 99 patches forward or moving our GPUI pin back.

**Keep the chrome in Rust; add scripting later.** No second language now. The Kit adapters are then written twice, and the API is designed without a real client.

**Process plugins.** Any language, no native UI.

## What the Rune spike still shows

S7 ran the toolbar as a Luau script (181 lines; the Rust one is 273) beside the Rust toolbar, on Linux with no window. [The scripted toolbar](./assets/0055/luau-toolbar-shape-armed.png). Its findings carry over to `gpui-shell`, because they are about the shape, not the engine:

- A script over `toolbar(&App)` matched the Rust toolbar's accessibility tree and pixels, and wrote through `run(id)`, with the host refusing a disabled control.
- A saved edit reloaded in about 20 ms with no rebuild, and the app's state and the script's own state survived it.
- With the zoom readout in the model, 100 frames of pinch rendered the panel 68 times. That is why the camera stays out of the state.

## What is not known

- What `gpui-shell` costs inside the Kit shell: build time, binary size, and a render of the sidebar on a canvas with hundreds of items. Phase 0 measures the first two.
- Whether upstream's JIT in release builds behaves in a long-lived window, or whether we need the interpreter as Herdr did.
- Whether GPUI Kit would take a patch that exposes the budgets and the interpreter switch.
- Whether `specular mod check`'s element tree is enough for the headless runner to press controls a mod adds that no Rust model lists.
- How pages in lane 3 get the bridge: CEF's message router, or the devtools channel `dom_query` already uses.

## How much of Herdr is mods

The header of Herdr's `src/window.rs` says it: "Every visible part that isn't a terminal is a mod." What stays in Rust is the part that has to be fast or has to be the app's own:

- **Native (about 37,000 lines of app Rust).** The layout engine and its locked primitives (blocks, splits, stacks, floats, windows, pane drag and resize), the title bar, overlay hosting, terminal drawing and the VT (about 8,000 lines), key dispatch, the command table (78 commands), the model and its sync with herdr-core, settings, updates, and the mod host itself (about 2,200 lines). The `gpui-shell` fork underneath is another 84,000 lines, and herdr-core is a separate Rust program.
- **Mods (about 3,900 lines of JavaScript, 20 mods).** The spaces sidebar, the agents list, the tab bar, menus, pickers, the prompt, the confirm dialog, the toast, the files and changes panels, the worktree flow, and the pane header. A further 3,000-line `win98` mod is a showcase theme.
- **The app's own screens (about 2,200 lines of JavaScript in `frame/`).** Settings, welcome, the palette, feedback and what's new are written on the same runtime but rewritten on every start and not open to editing. This is the one place Herdr bends "nothing shipped is special".

Herdr's terminal is our canvas: the one surface that stays native and that mods place, but do not draw.

## What becomes a mod, what stays native

**Becomes a shipped mod.** Each one replaces a Kit adapter in `specular-shell/src/kit/view`.

| Today | Kit adapter | Becomes | Phase |
|---|---|---|---|
| Toolbar | `toolbar.rs`, 334 lines | `mods/toolbar` | 3 |
| Tab row (view strip) | `tabs.rs`, 320 | `mods/tabs` | 4 |
| Canvas context menu | `canvas_menu.rs`, 292 | `mods/menu` | 4 |
| Palette | `palette.rs` and `palette_row.rs`, 320 | `mods/palette` | 4 |
| Share | `share.rs`, 141 | `mods/share` | 4 |
| Update notice, page notices | small | `mods/toast` | 4 |
| Dock (the item and tool popup) | `dock.rs` with `controls.rs`, `dropdown`, `field.rs`, about 1,200 lines shared | `mods/dock`, with the shared controls as a JS module | 4 |
| Onboarding and first run | `onboarding`, 257 | `mods/welcome` | 4 |
| Sidebar | `sidebar.rs`, 324 | `mods/sidebar`, on a virtual list | later |
| Settings | `settings.rs` and `settings_repos.rs`, 428 | `mods/settings` | later |
| Chat panel | `chat` and `chat.rs`, about 2,200 | `mods/chat` | later |

Three pieces inside those panels cannot be written in JavaScript and become host components a mod places, the way it places the zoom readout: the text field with IME (`field.rs`, `ime.rs`), the GPU avatar in chat (`chat/avatar.rs`), and the onboarding prism (`onboarding/prism.rs`).

**Stays native.**

- The canvas: the compositor, the wgpu pass, CEF pages, and everything drawn on an item: selection handles, guides, text editing and IME, edges, comment pins, presence and agent cursors.
- Every tool, gesture and item kind, until [ADR 0057](./0057-mod-items-and-tools.md).
- The document, `Command`s and undo, sync, persistence, the HTTP API, the agent helper.
- The window: the regions, the canvas slot, key dispatch, the macOS menu bar (its items come from the command registry), and the mod host.

## How the core is refactored

All of it is Rust, in `specular-interact` unless it says otherwise, and worth doing whether or not a line of script ever ships.

**1. One command registry.** Today what the app can do is spread over:

- two enums: `Action` (64 top-level variants) and `ShellCommand`, which the window handles outside `update`;
- three name-to-action tables: `specular-shell/src/script/words.rs`, `script/facts_canvas/step.rs`, and the CLI's verb match in `specular-cli/src/main.rs`;
- two sets of labels: `menu.rs` and `palette.rs`;
- the HTTP routes, most of which build a document `Command` directly and never pass through `Action`.

These become one table, modeled on Herdr's:

```rust
pub struct CommandSpec {
    pub name: &'static str,
    pub args: &'static str,
    pub about: &'static str,
    pub enabled: fn(&App) -> bool,
    pub build: fn(&Value) -> Result<Run, RunError>,
}

pub enum Run {
    Action(Action),
    Shell(ShellCommand),
    Api(ApiRun),
}
```

- The palette, the menus, the three name tables, the CLI and mods read it. Labels are written once.
- The HTTP API gets one new route, `POST /run {name, args}`. The existing routes stay as they are, for the clients that use them.
- `args` is a line of help, not a schema, to begin with.

**2. Keyed state on the models.** `Models` in `specular-shell/src/kit/canvas/models.rs` already rebuilds every panel model when the app changes and compares the result.

- Each field becomes a key (`panels/toolbar`, `panels/dock`, …) with a revision that moves when the field's value does.
- A port of Herdr's `publish` and `LiveReads` records which keys each mod read and re-renders it only when one moved.
- The models get `Serialize`, so they cross into script and into `mod check`'s fixture.
- This does not wait for ADR 0052. If its state document is built, the keys become views of it.

**3. Insets from the layout.** Core hard-codes the chrome's geometry: `covered_left` and `covered_top` in `viewport.rs`, `TAB_ROW` and `DOCK_ROW` in `panel/ui.rs`, `TOOLS_DOCK_WIDTH` in `settings.rs`, and a hand copy in `specular-shell/src/script/chrome.rs`. The shell computes the insets from the layout and sends one `Event::Insets`, which replaces those constants. A hidden toolbar then gives its space back to the canvas with no other change.

**4. Regions with `hidden`, not a tree yet.** The window's regions are composed in `specular-shell/src/kit/view.rs` as overlays on a full-size canvas slot. The layout file names today's regions, what fills each, and whether it is hidden. A general tree of rows, columns and floating nodes waits until a person's layout needs one.

**5. One id per control.** `ControlId` already names each control for the headless runs (`named_controls`, `kit/view/named.rs`). The same id is what a layout's `props.hidden` lists and what `mod check` prints, so the scenarios press a mod's controls the way they press a panel's today.

## Phases

Each phase ships on its own, leaves the app working, and has a test that ends it and a result that stops it.

**0. Spike.** `gpui-shell` 0.11 in the Kit shell with `default-features = false`, and one panel drawn from `toolbar(&App)` beside the native toolbar.

- Ends with: build time and binary size before and after, render time and idle wakes for the panel, and the list of upstream patches we would need.
- Stops if: the release build freezes or needs a fork beyond a budget setter and an interpreter switch. Then the runtime is reconsidered before any other phase depends on it.

**1. Rust groundwork, no script.** The command registry and `POST /run`, keyed state on `Models`, `Event::Insets`, the layout file with `hidden` and `ui.toggle`, and the reset key.

- Ends with: the palette, menus, CLI and scenario words reading the registry, the three name tables deleted, and any region hidden or shown from a file with the canvas taking the space.

**2. The page bridge.** A page from the space folder reads the keyed state and calls `run`. The first test page is a small widget an agent writes in one file.

- Ends with: the widget placing an item through `run`, undone with one Cmd+Z, and a web page shown to have no bridge.

**3. The runtime and the toolbar.** The mod host, `mod.json` with `api`, the `specular` host module, `specular mod check`, the guide and `specular.d.ts`. The toolbar is the first shipped mod.

- Ends with: the toolbar mod matching the native toolbar's controls by id in the scenarios, a median render under 2 ms and none over 8 ms, and no script wakes while the app is idle. The Kit toolbar adapter is deleted in the PR that shows those numbers.
- Stops if: the numbers are missed after tuning. The native toolbar stays and the ADR is revisited.

**4. The small panels.** Tab row, context menu, palette, share, toast, then dock and welcome with their host components. `specular mod clone`.

- Ends with: each panel passing the same test as the toolbar before its adapter is deleted. A panel that misses it stays native, and that is an acceptable end state for that panel.

**Later, decided after phase 4's numbers.** The sidebar on a virtual list, settings and chat. A general layout tree. Sharing mods between people, and with it the permission model.

## Consequences

- A person or an agent can hide, show, rearrange, restyle and replace the small panels of the window by editing files, and add to the canvas by writing a page.
- The app has one command registry and one way to run a command by name, whatever happens to scripting.
- The chrome is in JavaScript. The script API is `gpui-shell`'s element vocabulary plus ours, and moves when GPUI Kit's does. We take it from crates.io. Whether we can stay off a fork is not known until phase 0.
- A second language for the chrome, with its own tests: a mod plus a state fixture gives an element tree, checked with no window.
- The API has to be kept. Command names and state keys become public, under an `api` number.
- A mod has the person's access. Installing one is running code, and the app says so where mods are installed.
- New tools, new kinds of canvas item and GPU shaders are not in this ADR. A page in the space folder is the way to add to the canvas until ADR 0057.
