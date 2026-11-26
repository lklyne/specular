# ADR 0055: The interface as mods: scripted chrome and tools on the Kit shell, pages as canvas extensions

**Status:** Proposed, revised 2026-11-26. The first version (2026-11-25) put the scripts on Rune's Luau lane and went on hold when [ADR 0051](./0051-rune-shell-around-the-compositor.md) was deferred. This version moves them to `gpui-shell` on the GPUI Kit shell that stays ([ADR 0040](./0040-gpui-kit-hybrid-shell.md)), and opens the API from "restyle the controls Rust lists" to "show, hide, replace or add any part of the chrome, and add tools and new kinds of canvas item". Not built.
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

**2. Canvas items and tools are mods too.**

A mod can add a new kind of thing to the canvas, not only arrange the chrome around it. An image feed, a screen mirrored from a phone, a video recorder or a page-like viewer is a mod, and needs no Rust change unless it needs a native capability the host does not have yet (see "Host modules" below).

- **Mod items.** `Kind` gets one variant, `Kind::Mod(ModItem)`: the mod's name, its props as JSON, and the frame every item has. It moves, resizes, groups, syncs, undoes and saves like any other item, because those are done by the canvas on the frame, not by the mod. With one variant the no-wildcard rule holds and the compiler lists every place that must handle mod items once.
- **In the `.canvas` file.** A mod item is a JSON Canvas node whose `specular` fields name the mod and hold its props. Its base `type` is a fallback other tools can show: a `link` or `file` node when the item wraps a URL or a file, otherwise a `text` node with the mod's name. A space whose mod is missing shows a placeholder and keeps the props untouched.
- **What an item shows.** Its content is one of three surfaces, all composited by the canvas pass so a pan or zoom never enters the script:
  - **A description**: rects, paths, text, images and video frames in item space, kept by the canvas and redrawn from the description until the script changes it. Paths can be stamped strokes: a texture stamped along the points with per-point size, opacity and jitter, and a blend mode, for a simple textured brush. This is what an image viewer or a chart uses.
  - **Web content**: an HTML file from the mod's folder or a granted URL, hosted by CEF the way a page is, with the bridge to read the snapshot and `run`. A page-like viewer is this.
  - **A frame source**: a stream of frames from a host module (a camera, a screen, a video file, a phone, a local process), drawn through the same import path as a page's frames (`PageSource` in `specular-core`). A recorder or a mirror is this.
- **Input.** A mod item gets pointer, wheel and key events in item space once it is entered, the way a page is entered, and the canvas keeps select, move and resize when it is not.
- **State.** Durable state is the item's props, changed through `run("item.set", {id, props})`, so it is one undo step on the actor that made it and syncs through ADR 0054's mirror. Transient state (a playing stream, a scroll offset) stays in the mod.
- **Tools.** A mod can register a tool: an id, an icon, a cursor, a key, and pointer handlers in world coordinates. While the pointer is down the tool returns a preview description that the canvas draws in world space. On release it commits through `run`, as one undo step, usually by placing one of its own items. `Tool` gets one variant, `Tool::Mod(ModToolId)`, and the registry of ADR 0016 lists shipped and scripted tools together.
- **The shipped kinds.** "Nothing shipped is special" applies here too, in time. Page, file, text, drawing and shape stay native while their speed depends on it. The test of this lane is that a mod can build a page-like item from the same surfaces, so a shipped kind can move to a mod whenever its native code stops paying for itself.

**3. Arrangement, visibility, keys and theme are files.**

- A layout is JSON: which mod sits in which region, sizes, and what is hidden. Any element can be hidden by id: a whole region, a mod, or one control in a shipped mod (`"hidden": ["toolbar/inspect", "dock", "sidebar"]`), with no clone needed. `run("ui.toggle", {id})` and the palette do the same at run time, and a key can be bound to it.
- Every element a shipped mod draws carries a stable id, which `specular mod check` and the headless runner use to name it.
- Keys and the theme are files beside the layout. Shipped files are replaced on start. `specular mod clone <name>` makes a copy that is the person's, records what it came from, and is never updated.
- A save reloads what changed. A bad layout, keys or theme file keeps the last good version and writes `<name>.error` beside it. A mod that fails at load keeps its old build running; one that loads and then fails in `render` shows an error card in its region until the next save. One fixed key, which nothing can rebind, returns to the shipped layout and keys.
- `mod.json` can add palette entries, as Herdr's does.

**4. The lightest extension is a page.**

- An HTML file in the space folder is already a live page on the canvas. A page from the space gets a bridge: the same snapshot to read, and `run(action, args)` to write.
- A custom canvas widget is then one HTML file, which an agent can write, on an item that already moves, resizes, syncs and undoes like any other.
- The bridge is for pages loaded from the space folder. A page from the web never gets it.
- Herdr has nothing like this (no HTML, no web view). It is ours because the canvas already hosts pages.

**Rule: nothing shipped is special.** A capability a shipped mod uses is one a person's mod can use. If the chrome needs something mods cannot do, the API grows; the chrome does not get a private door.

**Rule: the native core answers for itself.** Before a feature goes into Rust, answer Herdr's three questions: does the app itself act on it, can only the app see it, can a mod not do it correctly? Three noes and it is a mod.

## Host modules: the native side of a mod

A mod is JavaScript, but what it can reach is Rust. `gpui-shell` calls these host modules: named imports, written in Rust, that a host gives its scripts (Herdr's is the `herdr` module). Ours start with:

- `specular`: the snapshot, `run`, items, tools, layout, notifications.
- `net`: `fetch` and sockets, per host.
- `fs`: files in the space folder and the mod's own folder.
- `media`: decode images and video, play audio, and turn a stream into a frame source: a URL (HLS, MJPEG, WebRTC), a file, or a local process that writes frames.
- `capture`: camera, microphone and screen, through the OS's own permission prompts, and recording to a file.
- `process`: start a helper program and talk to it over its pipes.
- `gpu`, later: the WebGPU API (`navigator.gpu`, WGSL) bound onto the canvas's own wgpu device, so a mod can render into a texture the canvas pass draws with no copy. wgpu already implements WebGPU, and naga validates every shader, which is the sandbox for a mod's own WGSL.

Each one is behind a capability in `mod.json`. When a mod needs something no host module offers (a phone's screen over USB, say), the answer is a new host module in Rust, added under the core rule, and every mod gets it. Most needs can be met sooner by `process` plus `media`: a helper program that already speaks to the device writes frames, and the mod shows them.

## Why JavaScript, and where Rust stays

Herdr's app is about 37,000 lines of Rust and 9,000 of JavaScript, and the split is by who writes the code and when:

- **The host is Rust, built and shipped once.** Layout, drawing, terminals, input, system access, speed. `gpui-shell`'s own README puts the line here: "the host owns rendering, layout, input and system capabilities; the script owns composition, presentation and business logic".
- **Mods are JavaScript because they arrive after the build.** `gpui-shell` exists so that "a new panel, a side tool or a piece of business logic arrives as a script loaded into the same process — no rebuild, no binary to redistribute, and no fork". It chose JavaScript "because application code reads better in it and the language is more widely known". A save reloads a mod in place.

The same split fits us, and the alternatives do not:

- **Mods in Rust** need a Rust toolchain on the person's machine, a rebuild per change, and either a fork of the app or loading native libraries. Rust has no stable ABI, so a library built against one release breaks on the next. A native library cannot be sandboxed, and it cannot hot reload.
- **WebAssembly mods** can be sandboxed and written in any language, Rust included. Nothing binds GPUI elements to WebAssembly today, and an agent writing a panel would compile on every save.
- **JavaScript on `gpui-shell`** is what Herdr ships, on the GPUI version we pin, with a capability model that starts empty. It is the language agents write best, and it reloads on save.

So: Rust for the host, the canvas, the document and host modules; JavaScript for the chrome, mod items, tools and everything a person or an agent adds.

## The watercolor brush and `webgpu-brush`

The brush test uses `webgpu-brush` (npm `webgpu-brush` 3.0.0, Lyle's WebGPU port of p5.brush), not stamped strokes. It paints natural brushes, washes and hatching in WGSL compute and render passes, with the geometry kept on the GPU. Two facts decide where it can run:

- It needs the browser's WebGPU API (`navigator.gpu`, a `GPUDevice`, WGSL). QuickJS has none, so it cannot run in a plain mod script.
- It can paint into a device it is handed (`initDevice({ device })`, the path its three.js adapter uses) and can run with no canvas, so it does not need a DOM to draw.

So the brush is built in two steps:

1. **Now, as web content.** The brush's items show web content: a page from the mod's folder runs `webgpu-brush` in CEF, which has WebGPU, and the canvas draws its frames like any page's. The tool sends the stroke's points over the bridge, and the points are the item's props, so the stroke reloads, syncs and undoes as data. This needs no new host module.
2. **Later, on the `gpu` host module.** The same code is handed the canvas's own device and paints into a texture the canvas pass draws. That drops a browser per brush layer and the frame copy, and is the general route for any mod that brings its own shaders.

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
- How a scripted tool's or an entered mod item's pointer moves stay cheap: coalesced to the display rate, or with the preview kept host-side between script calls.
- How much work the `gpu` host module is: binding WebGPU onto QuickJS over wgpu (Deno does the same over V8 in `deno_webgpu`), and the limits that keep one mod from starving the canvas of GPU time or memory.
- What a canvas of many mod items costs: how many descriptions the canvas pass can keep, and how many frame sources it can import at once.
- Whether web content in a mod item can share the page pipeline (CEF offscreen, frame import) without a page's own state, or needs its own host.
- Whether `specular mod check`'s element tree is enough for the headless runner to press controls a mod adds that no Rust model lists.
- How pages in lane 4 get the bridge: CEF's message router, or the devtools channel `dom_query` already uses.

## How much of Herdr is mods

The header of Herdr's `src/window.rs` says it: "Every visible part that isn't a terminal is a mod." What stays in Rust is the part that has to be fast or has to be the app's own:

- **Native (about 37,000 lines of app Rust).** The layout engine and its locked primitives (blocks, splits, stacks, floats, windows, pane drag and resize), the title bar, overlay hosting, terminal drawing and the VT (about 8,000 lines), key dispatch, the command table (101 commands), the model and its sync with herdr-core, settings, updates, and the mod host itself (about 2,200 lines). The `gpui-shell` fork underneath is another 84,000 lines, and herdr-core is a separate Rust program.
- **Mods (about 6,000 lines of JavaScript, 21 mods).** The spaces sidebar, the agents list, the tab bar, menus, pickers, the prompt, the confirm dialog, the toast, the files and changes panels, the worktree flow, and the pane header. A further 3,000-line `win98` mod is a showcase theme.
- **The app's own screens (about 2,200 lines of JavaScript in `frame/`).** Settings, welcome, the palette, feedback and what's new are written on the same runtime but rewritten on every start and not open to editing. This is the one place Herdr bends "nothing shipped is special".

Herdr's terminal is our canvas: the one surface that stays native and that mods place, but do not draw.

## What becomes a mod, what stays native

**Becomes a shipped mod.** Each one replaces a Kit adapter in `specular-shell/src/kit/view`.

| Today | Kit adapter | Becomes |
|---|---|---|
| Toolbar | `toolbar.rs`, 334 lines | `mods/toolbar` |
| Dock (the item and tool popup) | `dock.rs` with `controls.rs`, `dropdown`, `field.rs`, about 1,200 lines shared | `mods/dock`, with the shared controls as a JS module |
| Tab row (view strip) | `tabs.rs`, 320 | `mods/tabs` |
| Sidebar | `sidebar.rs`, 324 | `mods/sidebar`, on a virtual list |
| Canvas context menu | `canvas_menu.rs`, 292 | `mods/menu` |
| Palette | `palette.rs` and `palette_row.rs`, 320 | `mods/palette` |
| Share | `share.rs`, 141 | `mods/share` |
| Onboarding and first run | `onboarding`, 257 | `mods/welcome` |
| Settings | `settings.rs` and `settings_repos.rs`, 428 | `mods/settings` |
| Chat panel | `chat` and `chat.rs`, about 2,200 | `mods/chat`, last |
| Update notice, page notices | small | `mods/toast` |

**Stays native.**

- The canvas: the compositor, the wgpu pass, CEF pages, and everything drawn on an item: selection handles, guides, text editing and IME, edges, comment pins, presence and agent cursors.
- Gestures and hit tests for the built-in tools, and how each native `Kind` draws, until a kind moves to a mod.
- Host modules: networking, media decode, frame sources, capture, processes.
- The document, `Command`s and undo, sync, persistence, the HTTP API, the agent helper.
- The window: the region layout engine, the canvas slot, key dispatch, the macOS menu bar (its items come from the command table), the mod host, and the sandbox.

## How the core is refactored

Most of the work is in Rust, and most of it is worth doing whether or not a line of script ever ships.

1. **One command table.** Today what the app can do is spread over the `Action` enum (72 variants), the palette's entries (`interact/src/palette.rs`), the key map (`bindings.rs`), the selection verbs (`verbs.rs`) and the routes in `specular-api`. These become one registry. Each entry has a name, typed arguments, a line of help and an enabled rule. The palette, keys, menus, the HTTP API, the CLI and mods all read it. Its generated schema is the API snapshot the release check compares.
2. **One published state.** ADR 0052's state document becomes the mod read API. It is split into keys (`selection`, `tool`, `tools`, `items`, `space`, `settings`, `theme`, `panels/*`), and each key gets a version. A mod that read only `selection` re-renders only when `selection` moves. This is the narrow re-render Herdr does not have.
3. **Models become queries.** `toolbar(&App)`, `dock(&App)` and the rest stay in `specular-interact`, still pure and still tested, but published as data under `panels/*`. The smart part (which properties apply to this selection, and their current values) becomes its own query, `properties(selection)`, so a mod that draws its own dock does not have to redo it.
4. **A layout tree from a file.** The Kit shell's fixed regions (`shell.rs`, `slot.rs`) become a tree read from `layouts/<name>.json`: rows, columns, stacks, a `canvas` block, a `mod` block, and `hidden` ids. Herdr's locked primitives are the model.
5. **Stable ids on every control**, so a layout can hide one and a script can name one.
6. **Mod items and tools in the core.** `Kind::Mod(ModItem)` with its `.canvas` form, and the canvas pass learns to draw a kept description and a frame source on an item. `Tool::Mod(ModToolId)` and a world-space preview layer. Tool defaults (ADR 0009) are keyed by tool id, so a scripted tool keeps its own. The page pipeline's frame import is generalized from pages to any frame source.
7. **Headless by name, kept.** Headless runs already press controls by name through the models (`named_controls`), with nothing drawn. A scripted panel has to keep that: `specular mod check` gives an element tree with the same ids, so the scenarios can press a mod's own controls too, including ones no Rust model lists.

## Phases

Each phase ships on its own and leaves the app working.

1. **Rust groundwork, no script.** The command table and its schema check, the keyed state, the layout file with `hidden` and `ui.toggle`, stable ids, and the reset key. Hiding and showing anything works at the end of this phase.
2. **The runtime.** `gpui-shell` 0.11 in the Kit shell, the CPU budget and heap cap, `specular mod check` and `specular shot`, the generated guide and `.d.ts`. The toolbar is the first shipped mod, and its Kit adapter is deleted.
3. **The small panels.** Dock, tab row, context menu, palette, share, welcome, toast. Each one deletes its Kit adapter in the same PR, and the scenarios press its controls through `check`'s tree.
4. **Mod items and tools.** `Kind::Mod`, the three surfaces, the `media`, `net` and `capture` host modules, and `Tool::Mod`. Four mods test it: a watercolor brush (a tool on the main toolbar beside stickies and documents, with a live preview while drawing, painting with [`webgpu-brush`](https://github.com/lklyne/webgpu-brush); see below), an image feed (a description, with `net`), a video recorder (a frame source from `capture`, saving to the space folder), and a phone mirror (a frame source from a helper process).
5. **The big panels.** The sidebar on a virtual list, then settings, then chat, the largest and the one with streaming. Chat goes last and can stay native longer without blocking anything else.
6. **Opening it up.** `specular mod clone`, installing other people's mods with the capability prompt, and the page bridge of lane 4.

## Consequences

- A person or an agent can hide, show, rearrange, restyle and replace any part of the window, and add tools and new kinds of canvas item, by editing files; and add to the canvas by writing a page.
- The chrome is in JavaScript. The script API is `gpui-shell`'s element vocabulary plus ours, and moves when GPUI Kit's does. We take it as a crates.io dependency and keep any patch upstream, not in a fork.
- A second language for the chrome, with its own tests: a mod plus a snapshot gives an element tree and the actions it ran, checked with no window.
- The API has to be kept. The snapshot schema and the action table become public, and the release check makes a break deliberate.
- Opening tools and items to mods means pointer events enter script while a scripted tool is active or a mod item is entered. Pan, zoom and the built-in tools never do.
- A `.canvas` file can hold items that need a mod the reader does not have. They show as placeholders and keep their data, and other JSON Canvas tools see the fallback node.
