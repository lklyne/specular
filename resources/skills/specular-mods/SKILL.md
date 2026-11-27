---
name: specular-mods
description: "Build, change or debug a Specular mod: a folder of mod.json and main.js that draws part of the window, puts its own items on the canvas, adds a toolbar tool, or adds commands to the palette, keys and dock. Use only when the user asks for a Specular mod, a new Specular canvas item type, tool, brush, panel or palette command. To drive the app or its canvas, use the specular skill instead."
allowed-tools: Bash(specular:*)
---

# Building Specular mods

A mod is a folder with a `mod.json` and a `main.js`. Its script reads the
app's state by key, renders again only when a key it read changes, and changes
the app by running commands by name, the same commands the palette, the keys
and the `specular` CLI run. A save reloads a mod at once. A save that does not
load keeps the last good build running and writes `<name>.error` beside it.

## Before you start

Check that the app is running and answers:

```sh
specular state theme
```

If it fails, say the Specular app isn't running and stop; `specular mod check`
still works on a folder without the app. Don't start, kill or restart an app
you didn't start.

The installed files are the authority for the surface, not this skill. Two
files have all of it. Read them before you write a mod:

- `<app home>/mods/README.md`: the guide, with every state key and command.
- `<app home>/mods/specular.d.ts`: the `specular` module's types, including
  `ModDraw`, the display list.

`<app home>` is `~/Library/Application Support/Specular` on macOS. In the
repository, both files and the shipped mods are in
`native/crates/specular-mods/` (`mods/README.md`, `specular.d.ts`,
`mods/<name>/`).

## Pick the kind of mod

| You want | Write | Example to copy |
|---|---|---|
| A panel drawn differently (toolbar, tab row, notices, menu) | a panel mod, placed by the layout file | `toolbar`, `toast` |
| A new kind of canvas item | `items` in `mod.json`, a display list from `item.draw` | `feed` |
| A canvas item that is live web content | `items.web`, an HTML file in the mod's folder | `watercolor` |
| A tool in the toolbar | `tools` in `mod.json`, following the `tool` key | `ink` |
| A command in the palette, on a key or in the dock | `commands` in `mod.json`, acting on the `call` key | `feed` (`feed/shuffle`) |

A mod can be several of these at once. `ink` is a tool, an item type and a
command.

## Start from a copy

```sh
specular mod clone ink my-pen      # copies a shipped mod to mods/my-pen
```

Edit `mods/my-pen/mod.json` and `main.js`. A clone is never overwritten. A
shipped mod's folder is rewritten on every start, so do not edit it in place.

## mod.json

```json
{
  "api": "1.1",
  "title": "My pen",
  "items": { "size": [320, 240], "web": "item.html" },
  "tools": [{ "name": "pen", "title": "My pen", "icon": "pen-line" }],
  "commands": [{ "name": "clear", "title": "Clear my pen", "icon": "trash", "key": "alt+shift+c", "dock": true }],
  "palette": [{ "title": "Add a my-pen card", "run": "item.add", "args": { "kind": "mod", "mod": "my-pen" } }]
}
```

- `api` is `1` (1.0, panels only) or `"1.1"` (items, tools, commands).
- `items.size` is a new item's size; `items.web` makes items live web content.
- `icon` is one of the app's glyphs by kebab-case name (`pen-line`,
  `brush-pen`, `brush-highlighter`, `image`, `code`, `trash`, `reload`). An
  unknown name falls back to a default glyph.
- `key` is modifiers (`cmd`, `shift`, `alt`) and one character joined by `+`.
- `palette` entries run any app command with fixed args. `commands` are the
  mod's own, run as `<mod>/<name>`.

Unknown fields are refused, and the error says which.

## The patterns every canvas mod uses

A mod with items, tools or commands runs out of sight whatever the layout says.
Its `render()` returns an empty `div()` and does its work from state.

**Draw an item once per change.** Keep what you last sent, and send again only
when the item's size or props change. Moving an item changes no key, so it
never wakes the script.

```js
import { View, div } from "gpui-kit";
import { state, run } from "specular";

const drawn = new Map();

export default class MyItems extends View {
  render() {
    const items = (state("canvas/mod_items") || []).filter((item) => item.mod === "my-pen");
    for (const item of items) {
      const key = JSON.stringify([item.width, item.height, item.props]);
      if (drawn.get(item.id) === key) continue;
      drawn.set(item.id, key);
      run("item.draw", { id: item.id, draws: [
        { type: "rect", x: 0, y: 0, width: item.width, height: item.height, fill: "#ffffff", radius: 8 },
        { type: "text", x: 12, y: 12, text: item.props.title || "Untitled", size: 14 },
      ] });
    }
    return div();
  }
}
```

**Act once per number.** The `tool` and `call` keys carry a `number` that
counts up. Remember the last one you acted on, and skip the value the script
first sees when it loads, so a reload does not replay an old stroke or call.

```js
let lastCall = null;
// in render():
const call = state("call");
const number = call ? call.number : 0;
if (lastCall !== null && number > lastCall && call.command === "my-pen/clear") {
  for (const id of (call.args && call.args.ids) || []) run("item.set", { id, props: { points: [] } });
}
lastCall = number;
```

**A tool previews, then commits.** While the tool is in hand the drag is the
`tool` key, `{tool: "<mod>/<tool>", number, phase: "down"|"up", points}` in
canvas units. On `down`, send `tool.preview` with a display list in canvas
units. On `up`, place what the stroke made with `item.add` (one undo step,
which also clears the preview), with its points made relative to the item's
top-left. See `ink/main.js`.

**Props are the item's data.** `item.set {id, props}` merges keys into an
item's props as one undo step (`null` removes a key). Keep everything the item
needs to draw itself in its props. The last display list is saved in the
`.canvas` file as the item's snapshot, so a person without the mod still sees
it.

**Web items read their props through the bridge.** An `items.web` page is
opened as `file://<mod>/item.html#item=<id>`. Wait for `window.specular` (it
may arrive after your script, with a `specular-ready` event), then
`specular.subscribe("canvas/mod_items", ...)` and find your id. See
`watercolor/item.html`. Still send a display list from `main.js`: it is the
snapshot and what shows where the mod is not installed.

## Check it

```sh
specular mod check ~/Library/Application\ Support/Specular/mods/my-pen
specular state canvas/mod_items      # what your items look like to the mod
specular run tool.set '{"tool": "my-pen/pen"}'
specular run my-pen/clear
specular mod stats                   # renders, render times, keys read
```

`specular mod check` renders the mod once with no window and prints its
element tree and `console.log` lines. It exits nonzero with the exact error.
Never call a mod done without a passing check. `specular mod stats` must show
no renders while nothing changes; a mod that renders when idle is a bug.

Then look at it in the running app:

```sh
specular shot /tmp/my-pen.png        # the canvas as the window shows it
```

Open the picture and look. It shows the canvas with your items and your
tool's preview, not the panels around it, so check a panel mod in the window
itself or ask the person to. A still-working screen is not proof a save
loaded: look for `<name>.error`. Don't claim a visual check you haven't made.

## Rules that keep a mod fast and safe

- Render from state only. Never loop, poll or set timers to redraw; a key
  change is the only reason to render.
- `run()` queues the command and returns at once. It never runs inside your
  render.
- Do not send `item.draw` or `tool.preview` when nothing changed.
- A canvas file never runs a mod. Placing a mod's first item on a canvas
  copies the mod into the space's `mods/` folder; a person who opens that
  canvas without the mod is offered "Install mods" and sees snapshots until
  then.
- Keep display lists small: tens of marks, not thousands. Thin a stroke's
  points before you save them.

## Safety rules

- Edit a clone (`specular mod clone`), never a shipped mod's folder: it is
  rewritten on every start.
- A mod runs with the person's access to files, the network and processes.
  Don't add network calls, file writes outside the mod's `.data/` folder, or
  processes the person didn't ask for.
- Placing a mod's first item on a canvas copies the mod into that space's
  `mods/` folder. Say so when you do it in someone's space.
- Read ids and values from `specular state` and command results. Don't guess
  item ids.
- Before you install a mod someone else wrote, from a link, a folder or a
  canvas's "Install mods" prompt, read every file in it and tell the person
  what it does with files, the network and processes.

## What a mod cannot do

Check this list before you promise a feature. If something is not here and not
in the guide, assume a mod can't do it.

**The window.** A mod can replace the tab row, the toolbar, the Share button,
the notices, the canvas context menu and the first-run view. It can only add
entries to the command palette and buttons to the dock; naming a mod for
either in the layout only hides it. The layers panel (sidebar) and the chat
panel can only be hidden: there is no state key for them. A mod can't change
the contents of tool options popups, the settings window, the macOS menu bar,
the page inspector, or the theme's colours.

**The canvas.** A mod can't restyle or extend the built-in items and tools,
change selection outlines, handles, snapping or hit-testing, or read the
camera, the selection (except the ids the dock passes to its commands), items
other than its own, or the content of live web pages.

**Items.** An item can't be entered and gets no pointer, wheel or key input.
A display list has rects, ellipses, paths, text and images only: no
gradients, shadows, blurs, clipping, transforms, fonts or rich text. Resizing
doesn't scale it. No video, camera, screen, audio or GPU shaders; use
`items.web` for anything richer. `item.draw` isn't an undo step, and several
`run()` calls are several undo steps. The web viewer and sync peers show
snapshots only.

**Tools.** A tool gets one press-to-release drag in canvas units: no hover,
double click, modifiers, pressure, wheel or keys. The preview is one render
behind. A tool's only dock controls are the mod's command buttons.

**Commands and keys.** A key is one character with Command, Shift or Option,
works only on the canvas, and loses to any built-in key. Palette entries go in
the Mods group with no ordering, check marks or disabled state. No context
menu or menu bar entries.

**Running.** A mod runs only when installed in the app's mods folder; a canvas
never runs one. Install mods installs only from the canvas's space folder.
