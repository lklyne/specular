# ADR 0055: The interface as mods: scripted chrome and tools on the Kit shell, pages as canvas extensions

**Status:** Proposed, revised 2026-11-26. The first version (2026-11-25) put the scripts on Rune's Luau lane and went on hold when [ADR 0051](./0051-rune-shell-around-the-compositor.md) was deferred. This version moves them to `gpui-shell` on the GPUI Kit shell that stays ([ADR 0040](./0040-gpui-kit-hybrid-shell.md)), and opens the API from "restyle the controls Rust lists" to "show, hide, replace or add any part of the chrome, and add tools". Not built.
**Date:** 2026-11-25
**Related:** [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), the panel models. [ADR 0016](./0016-tools-as-capability-registry.md), tools as a registry. [ADR 0052](./0052-one-runner-one-state-document.md), the state document and action table this reuses. [ADR 0054](./0054-every-editor-is-an-actor.md), actors. [ADR 0051](./0051-rune-shell-around-the-compositor.md), the deferred Rune shell the first version depended on.

## Context

Herdr's desktop app (`herdrdev/herdr-app`) is built so that a person, or their agent, can rebuild its window. Read at its 2026-11-26 head, with its guide (`defaults/AGENTS.md`) and contributor rules (`AGENTS.md`):

- **The interface is a folder of files.** `~/.config/herdr-app` holds `layouts/<name>.json` (the window and its whole look), `keys.json`, `settings.json` and `mods/<name>/` (a `main.js`, a `mod.json`, icons). A save applies at once.
- **Nothing shipped is special.** The sidebar, tab bar, menus, prompt, confirm dialog, toast and pickers are 21 mods, about 9,000 lines of JavaScript, written on the API a user gets. A few of the app's own screens (welcome, settings, palette) are in a `frame/` folder that is rewritten on every start and not meant to be edited.
- **Clone to own.** Shipped layouts and mods are replaced on start. `mod.clone` makes a copy that is the person's and is never updated.
- **It cannot be broken.** A bad file keeps the last good version and writes `<name>.error` beside it. `ctrl-alt-shift-escape`, which nothing can rebind, sets the layout and keys aside and returns to the default.
- **A rule keeps the native core small.** Before anything is added to the core: does the app itself act on it, can only the app see it, can mods not do it correctly? Three noes and it is a mod, "even at the price of some repetition".
- **One way to read, one way to write.** Mods read live data (`spaces()`, `agents()`, `terminals()`, `focused()`, `layout()`, `theme()`, `commands()`) and act through `run(command, args)`, the command table keys, the palette and the CLI use.
- **Mods decide what to show.** A mod is not handed a list of controls. It reads the data and draws what it likes; the tab bar decides what a tab looks like.
- **Agents are first-class authors.** The guide is written into the app home on every start, a test fails if it misses a command or a setting, and generated `herdr.d.ts` and `gpui-kit.d.ts` are the full API. `herdr-app check <mod>` loads a mod and renders it once with no window. `herdr-app shot` saves the window as a PNG. A mod adds palette entries from its `mod.json`.

The runtime is `gpui-shell` from GPUI Kit: JavaScript on QuickJS describing GPUI elements, which Rust keeps and draws. Herdr carries a fork (`crates/herdr-shell`, 97 `HERDR:` patch sites). Upstream `gpui-shell` 0.11.0 pins `gpui-pre =0.3.14`, the same GPUI Specular pins with `gpui-kit =0.11.0`. Its capability grant is empty by default: no file, process, network, storage or clipboard access until the host grants it, per plugin, from a manifest that is read without running any code. Herdr grants mods everything, and its security doc says that is not a security boundary.

Herdr's headless core, `herdr-core`, adds an API that only grows: the OpenRPC document is generated from the Rust types, and a test fails on any change that would break a client of the last release.

What it cost them is in the changelog: window freezes while mods ran, a VM leak that broke every mod, idle mods waking 20 times a second. `gpui-shell` 0.11.0 has no CPU watchdog or heap limit of its own. The first version of this ADR said Herdr re-renders only the mods that read a changed key. Its docs do not say that: `setShared` re-renders every mod, and a mod's own state change re-renders that mod. Narrow re-renders would be ours to build.

Specular already has the parts that are hard to add late: one `Action` table every input goes through, panels as pure functions of `&App`, the HTTP API and CLI over the same verbs, and a shell that is GPUI Kit.

## Decision

Four lanes, two rules, and a sandbox.

**1. The chrome is mods.**

- A mod is a folder: `mod.json` and `main.js`, run by `gpui-shell` inside the Kit shell. It describes GPUI elements; Rust keeps and draws them.
- The shipped toolbar, dock, sidebar, tab row, menus, context menu, palette, settings and chat panel are mods, written on the API a person's own mod gets. The Kit adapters in `specular-shell/src/kit/view` are rewritten as mods instead of being kept.
- A mod reads a snapshot of the app: the space, the canvas and its items, the selection, the active tool, the tool list, settings, theme, keys, and the panel models (`toolbar(&App)`, `dock(&App)` and the rest) as plain data. The models are a convenience, not a contract. A mod may draw them as they are, or ignore them and draw its own controls from the data underneath.
- A mod changes the app only through `run(action, args)`: the `Action` table and the HTTP API's verbs, one table. Every write is a `Command` with an inverse, so a mod's edit is an undo step like any other, and with ADR 0054 it is undone on the actor that made it.
- A mod keeps its own state, and may share small values with other mods (`shared(key)` / `setShared`) in the space's own `specular` fields.
- Script runs on a snapshot change, an event or a timer, never per frame. The camera is not in the snapshot, so a pan or zoom cannot enter a script. Anything that shows a camera value, such as the zoom readout, is a host element the mod places.

**2. Tools are mods too.**

- A mod can register a tool: an id, an icon, a cursor, a key, and handlers for pointer down, move and up in world coordinates over the canvas.
- While the pointer is down, the tool returns a preview description (paths, rects, text, a handle) that the canvas pass draws in world space. The canvas keeps redrawing that description through a pan or zoom without entering the script.
- On release, the tool commits through `run`: create an item, add points to a drawing, set a property. Its commit is one undo step.
- `Tool` gets one variant, `Tool::Mod(ModToolId)`, so the no-wildcard rule holds and every place that matches on `Tool` handles scripted tools once. The registry of ADR 0016 lists shipped and scripted tools together, and the toolbar mod reads that list.
- A pen tool, which places Bézier points and commits a path, is the first test of this lane. Its result has to be an existing kind (a drawing stroke, a shape). How the canvas draws a kind, the hit test and the gestures of the built-in tools stay native. A new kind is a Rust change, and has to pass the core rule below.

**3. Arrangement, visibility, keys and theme are files.**

- A layout is JSON: which mod sits in which region, sizes, and what is hidden. Any element can be hidden by id: a whole region, a mod, or one control in a shipped mod (`"hidden": ["toolbar/inspect", "dock", "sidebar"]`), with no clone needed. `run("ui.toggle", {id})` and the palette do the same at run time, and a key can be bound to it.
- Every element a shipped mod draws carries a stable id, which `specular mod check` and the headless runner use to name it.
- Keys and the theme are files beside the layout. Shipped files are replaced on start. `specular mod clone <name>` makes a copy that is the person's, records what it came from, and is never updated.
- A save reloads what changed. A bad layout, keys or theme file keeps the last good version and writes `<name>.error` beside it. A mod that fails at load keeps its old build running; one that loads and then fails in `render` shows an error card in its region until the next save. One fixed key, which nothing can rebind, returns to the shipped layout and keys.
- `mod.json` can add palette entries, as Herdr's does.

**4. Canvas extensions are pages.**

- An HTML file in the space folder is already a live page on the canvas. A page from the space gets a bridge: the same snapshot to read, and `run(action, args)` to write.
- A custom canvas widget is then one HTML file, which an agent can write, on an item that already moves, resizes, syncs and undoes like any other.
- The bridge is for pages loaded from the space folder. A page from the web never gets it.
- Herdr has nothing like this (no HTML, no web view). It is ours because the canvas already hosts pages.

**Rule: nothing shipped is special.** A capability a shipped mod uses is one a person's mod can use. If the chrome needs something mods cannot do, the API grows; the chrome does not get a private door.

**Rule: the native core answers for itself.** Before a feature goes into Rust, answer Herdr's three questions: does the app itself act on it, can only the app see it, can a mod not do it correctly? Three noes and it is a mod.

## Sandbox

Specular loads live web pages and `.canvas` files from disk, so Herdr's "read the mod before you install it" is not enough. The control is open; the access is not.

- A mod starts with `gpui-shell`'s empty grant. Reading the snapshot, calling `run`, and its own storage need no grant. Files, processes, network and clipboard are asked for in `mod.json`, shown before the mod runs, and granted per mod. Shipped mods ask for what they use, and the list is checked in a test.
- A `.canvas` file or a web page can never name, load or select a mod.
- A layout names mods, so a shared layout is code and is installed as code.
- Every entry into script runs under a CPU budget and a heap cap. `gpui-shell` does not provide these; we add them with QuickJS's interrupt handler and memory limit, and a mod that trips one is stopped and shows its error card.
- The bridge in lane 4 is limited to the space folder's own pages.

## Tooling for agents

- `specular mod check <dir>` loads a mod with no window, renders it once against a fixture snapshot, and prints its element tree with ids. `gpui-shell`'s description-only snapshot is the base for it.
- `specular shot [file.png]` saves the window as a PNG.
- A generated guide and `specular.d.ts` are written into the mods folder on every start. A test fails if the guide misses an action or a layout key.
- The action table and the HTTP verbs are published as one schema, with the last release's copy checked in. A test fails on a change that would break a mod written against it: a removed action, a renamed argument, a new required input. A real break needs a new API generation, decided on purpose.
- Counters for script renders and time in script per mod, in the debug HUD and in `specular state`.

## Alternatives

**The first version: Luau on Rune.** A sandboxed VM with a watchdog and a heap cap built in, and S7 of [the spike](../plans/rune-spike.md) showed a scripted toolbar equal to the Rust one. It depends on the Rune shell, which is deferred, and `rune-luau` needs a C++ build.

**Scripts restyle the controls Rust lists, and nothing more.** The first version's line: Rust decides which controls exist, scripts lay them out. Easier to test, but a person cannot drop a control, add a tool, or build a panel the models do not foresee. Turned down for the more open line above; the models stay as data a mod can use.

**Herdr's trust model: mods get everything.** Simplest, and it is how Herdr ships. Not safe for an app that opens files and pages from other people.

**Keep the chrome in Rust; add scripting later.** No second language now. The Kit adapters are then written twice, and the API is designed without a real client.

**Process plugins.** Any language, no native UI.

## What the Rune spike still shows

S7 ran the toolbar as a Luau script (181 lines; the Rust one is 273) beside the Rust toolbar, on Linux with no window. [The scripted toolbar](./assets/0055/luau-toolbar-shape-armed.png). Its findings carry over to `gpui-shell`, because they are about the shape, not the engine:

- A script over `toolbar(&App)` matched the Rust toolbar's accessibility tree and pixels, and wrote through `run(id)`, with the host refusing a disabled control.
- A saved edit reloaded in about 20 ms with no rebuild, and the app's state and the script's own state survived it.
- With the zoom readout in the model, 100 frames of pinch rendered the panel 68 times. That is why the camera stays out of the snapshot.

## What is not known

- What `gpui-shell` costs inside the Kit shell: build time, binary size, and a render of the sidebar on a canvas with hundreds of items.
- How a scripted tool's pointer moves stay cheap: coalesced to the display rate, or with the preview kept host-side between script calls.
- Whether a pen tool can commit a useful path to today's kinds, or needs a path kind.
- Whether `specular mod check`'s element tree is enough for the headless runner to retire the built-in renderer (ADR 0044's open blocker).
- How pages in lane 4 get the bridge: CEF's message router, or the devtools channel `dom_query` already uses.

## First steps

1. Add `gpui-shell` 0.11 to the Kit shell and run one mod in the sidebar region. Measure build time and size.
2. The toolbar as a mod over `toolbar(&App)`, with `check` and a snapshot test.
3. The layout file with `hidden`, `ui.toggle` and the reset key.
4. A pen tool as a mod, through `Tool::Mod`.
5. The CPU budget and heap cap, and the render counters.

## Consequences

- A person or an agent can hide, show, rearrange, restyle and replace any part of the window, and add tools, by editing files; and add to the canvas by writing a page.
- The chrome is in JavaScript. The script API is `gpui-shell`'s element vocabulary plus ours, and moves when GPUI Kit's does. We take it as a crates.io dependency and keep any patch upstream, not in a fork.
- A second language for the chrome, with its own tests: a mod plus a snapshot gives an element tree and the actions it ran, checked with no window.
- The API has to be kept. The snapshot schema and the action table become public, and the release check makes a break deliberate.
- Opening tools to mods means pointer events enter script while a scripted tool is active. Pan, zoom and the built-in tools never do.
