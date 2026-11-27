# Specular mods

<!-- Written by Specular on every start. Edits here are overwritten. -->

This folder is **where Specular's window and canvas can be changed, as
files**. A mod is a folder with a `mod.json` and a `main.js`: it draws a part
of the window, or adds a kind of canvas item, a toolbar tool or commands, from
the app's state, and changes the app by running commands by name, the same
commands the palette, the keys and the `specular` CLI run. Save a file and the
app reloads the mod within a second. Shipped mods are Specular's: clone
before editing (see "Your own mods"). The toolbar, the tab row, the Share
button and the notices are built from the same parts you have.

## What you can and can't do

You can: redraw the toolbar, the tab row, the Share button, the notices, the
canvas menu and the first-run view; hide any region; add kinds of canvas
items drawn from a display list or shown as a web page; add toolbar tools;
add commands to the palette, the keys and the dock; run any app command;
read files, run programs and fetch URLs; keep state shared between mods.

You can't: draw the sidebar, the chat panel, the dock or the palette; change
the built-in items, tools or the theme's colours; read the camera, the
selection or a live page's content; get pointer or key input inside an
item. The full list is "What a mod can't do" at the end.

## What mods are written in

Mods use **gpui-kit** (its gpui-shell): JavaScript that describes **GPUI**
elements, a native GPU UI (`div()`, flex, children, `.on_click`). There's no
DOM: the script describes, Rust draws. Canvas items are display lists the
app draws, or an HTML file (`items.web`). **The type files here are the
truth**: `gpui-kit.d.ts` (written into a mod's folder when it loads) for
elements and `specular.d.ts` for the app's state and commands. They're
rewritten on every start to match this version.

## Changing things safely

- Edits apply at once. If a mod is invalid, the app keeps the last good
  build running and writes why to `mods/<name>.error`. Check after every
  edit.
- `specular mod check <mod folder>` loads a mod once with no window and
  renders it. Never call a mod done without a passing check; also try it in
  the app.
- **Escape hatch:** Control+Option+Shift+Escape always works: it sets your
  layout aside (`mine.json.aside`) and goes back to the shipped one. Nothing
  can rebind it.

## Seeing the window

`specular shot [file.png]` saves the canvas as the window shows it as a PNG
and prints its absolute path (default: `specular-shot.png` in your current
folder). Use it when debugging items and tools: inspect spacing, colours and
previews. It captures the canvas, not the panels around it, even when the
window is covered. Specular must be running with an open window.

## A mod in one screen

`mods/hello/mod.json`:

```json
{ "api": 1, "title": "Hello", "place": "toolbar",
  "palette": [{ "title": "Fit the canvas", "run": "view.zoom-to-fit" }] }
```

`mods/hello/main.js`:

```js
import { View, div } from "gpui-kit";
import { h_flex, Button } from "gpui-base";
import { state, run, colors } from "specular";

export default class Hello extends View {
  render() {
    const bar = state("panels/toolbar");          // renders again when it changes
    const tool = bar.sections.flatMap((s) => s.tools || []).find((t) => t.active);
    const k = colors();
    return h_flex().size_full().gap(8).p(8).items_center().bg(k.toolbar)
      .child(div().text_color(k.text).child(`Tool: ${tool ? tool.label : "none"}`))
      .child(Button.new("hello.select").px(8).py(4).rounded(6).bg(k.hover)
        .on_click(() => run("control.press", { id: "tool.select" }))
        .child("Select"));
  }
}
```

Put it in a region with your own layout file, `../layouts/mine.json` (copy
`default.json` to start it):

```json
{ "toolbar": { "mod": "hello", "props": {} } }
```

Then check it with no window: `specular mod check ~/path/to/mods/hello`.

## Where things are

- `mods/<name>/`: a mod. `mods/<name>.error` says why it did not load, and
  goes away on the next good load.
- `mods/specular.d.ts`: the whole `specular` module, with every state key and
  command. `gpui-kit.d.ts`, written into a mod's folder when it loads, has the
  element and style surface (`div`, `h_flex`, `Button`, `.bg(..)`, ...).
- `../layouts/default.json`: which mod fills which region (`tabs`, `toolbar`,
  `dock`, `sidebar`, `chat`) and which overlay (`menu`, `share`, `toast`,
  `palette`, `welcome`), whether it is hidden, and the `props` each mod gets.
  `"mod": null` is the app's own panel. The app writes the shipped layout
  over this file on every start, so an edit lasts until the next start.
- `../layouts/mine.json`: your own layout. When it is there it is the one
  in force, and the app never writes it. Control+Option+Shift+Escape sets it
  aside as `mine.json.aside` and brings the shipped layout back.
- `../layouts/.state/`: what was hidden and shown at run time
  (`ui.toggle`), apart from the file you edit.

## What a mod reads

`state(key)` returns a key's value and subscribes the mod to it: the script
renders again when that key changes and at no other time, never per frame.
The camera is in no key, so panning and zooming never run a script; the zoom
readout is the `Zoom` host component. The keys: `panels/toolbar`,
`panels/tool_options`, `panels/dock`, `panels/strip`, `panels/menu`,
`panels/palette`, `panels/share`, `panels/welcome`, `theme`, `settings`, `keys`, `layout`,
`notices`, and for canvas items, tools and commands `canvas/mod_items`, `tool` and
`call` (see below).
`specular state [key]` prints them from the running app.

`props()` is what the layout hands the mod. By convention `props().hidden`
lists the ids of controls to leave out, and a shipped mod honours it:
`{ "mod": "toolbar", "props": { "hidden": ["tool.inspect", "zoom"] } }`.

`colors()` is the chrome's palette for the theme drawn now (`toolbar`,
`toolbarText`, `popup`, `hover`, `raisedFill`, ...), as `#rrggbbaa` strings.
`cx.theme().colors` has GPUI Kit's generic tokens too.

`shared(key)` / `setShared(key, value)` pass small values between mods.

## What a mod does

`run(name, args)` runs a command, as the palette, a key or the CLI would. A
command that changes the canvas is one undo step. A control in a panel model
is pressed with `run("control.press", { id })`, by the `id` beside it in the
state. `commands()` lists them all; `notify(text, level)` writes a line to the
app's log. The commands:

- **app**: `app.quit`, `app.open-space`, `app.open`, `app.save`, `app.settings`, `app.hide`, `app.hide-others`, `app.minimize`, `app.command-palette`
- **canvas**: `canvas.new`, `canvas.duplicate` `{canvas?: id}`, `canvas.delete` `{canvas?: id}`, `canvas.switch` `{canvas: id}`, `canvas.rename` `{name: text, canvas?: id}`, `canvas.begin-rename` `{canvas?: id}`
- **edit**: `edit.cancel`, `edit.undo`, `edit.redo`, `edit.cut`, `edit.copy`, `edit.paste`, `edit.duplicate`, `edit.delete`, `edit.select-all`, `edit.select` `{ids?: [entity id], edges?: [edge id]}`, `edit.nudge` `{dx: number, dy: number}`
- **item**: `item.add` `{kind: sticky|text|shape|mod, x: number, y: number, text?: string, mod?: name, props?: object, width?: number, height?: number}`, `item.set` `{id: mod item id, props: object}`, `item.draw` `{id: mod item id, draws: [{type: rect|ellipse|path|text|image, ..}]}`
- **text**: `text.bold`, `text.italic`, `text.code`, `text.strikethrough`, `text.bullet-list`, `text.numbered-list`, `text.task-list`, `text.body`, `text.heading-1`, `text.heading-2`, `text.heading-3`, `text.heading-4`, `text.heading-5`, `text.heading-6`
- **arrange**: `arrange.bring-forward`, `arrange.send-backward`, `arrange.bring-to-front`, `arrange.send-to-back`, `arrange.group`, `arrange.ungroup`, `arrange.auto-layout`, `arrange.lay-out` `{mode: row|column|grid}`, `arrange.group-layout` `{axis: row|column|none}`, `arrange.group-gap` `{gap: number}`
- **comment**: `comment.annotate-selection`, `comment.resolve` `{id?: comment id}`, `comment.delete` `{id?: comment id}`, `comment.focus` `{id?: comment id}`, `comment.reveal` `{id: comment id}`
- **page**: `page.new-tab`, `page.edit-address`, `page.back`, `page.forward`, `page.reload`, `page.stop`, `page.navigate` `{url: address, page?: entity id}`, `page.frame`, `page.frame-color`, `page.rotate`, `page.scheme`, `page.orientation` `{orientation: portrait|landscape}`, `page.toggle-sync`
- **tab**: `tab.show` `{item?: entity id}`, `tab.open` `{item: entity id}`, `tab.close` `{item?: entity id}`, `tab.next`, `tab.previous`, `tab.lens` `{lens: fill|device}`, `tab.others` `{shown: bool}`
- **tool**: `tool.set` `{tool: select|page|text|sticky|document|shape|draw|comment|inspect|<mod>/<tool>}`, `tool.ellipse`, `tool.diamond`, `tool.highlighter`, `tool.preview` `{draws: [{type: rect|ellipse|path|text|image, ..}]}`
- **view**: `view.zoom-in`, `view.zoom-out`, `view.zoom-reset`, `view.zoom-to-fit`, `view.zoom` `{percent: number}`, `view.camera` `{x: number, y: number, zoom: number}`, `view.reveal` `{id: entity id}`, `view.theme` `{theme: light|dark|system}`, `view.toggle-sidebar`
- **ui**: `ui.toggle` `{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}`, `ui.show` `{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}`, `ui.hide` `{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}`, `ui.reset`
- **notice**: `notice.dismiss` `{id: notice id}`
- **mods**: `mods.install`
- **share**: `share.copy-link`, `share.access` `{scope: edit|comment|view|none}`, `share.join` `{link: url, fresh?: bool}`, `share.stop`
- **control**: `control.press` `{id: control name}`

## Canvas items, tools and commands (API 1.1)

A mod can put its own items on the canvas, add a tool to the toolbar, and
add commands to the command palette, the keys and the dock. A mod that does
any of these runs whatever the layout says, out of sight, so its script
renders when its keys change. Write `"api": "1.1"` in its `mod.json`:

```json
{ "api": "1.1", "title": "Feed",
  "items": { "size": [320, 400], "web": "item.html" },
  "tools": [{ "name": "pen", "title": "Ink pen", "icon": "pen-line" }],
  "commands": [{ "name": "shuffle", "title": "Shuffle the feed", "icon": "reload",
                 "key": "cmd+shift+u", "dock": true }] }
```

**Items.** `run("item.add", { kind: "mod", mod: "feed", x, y, props })` places
one (one undo step). The `canvas/mod_items` key lists the active canvas's
items, `[{id, mod, width, height, props}]`, without their positions: a move
wakes no script. A mod draws each item by sending its display list,
`run("item.draw", { id, draws })`: rects, ellipses, paths, text and images in
item space (`ModDraw` in `specular.d.ts`). `run("item.set", { id, props })`
changes an item's props as one undo step. With `items.web` the item is that
HTML file of the mod's folder shown as live web content, given its id as
`#item=<id>`; its display list is then what shows where the mod is not
installed.

The last display list an item drew is saved in the .canvas file as its
snapshot, and the canvas lists the mods it needs (copied into the space's
`mods/` folder when an item is placed). A canvas opened where a mod is not
installed shows each item's snapshot, or a placeholder card, and offers
"Install mods" in the `notices` key (`mods.install`). Opening a canvas never
runs a mod by itself.

**Tools.** A mod tool is armed with `tool.set {tool: "<mod>/<tool>"}` or its
toolbar button. While it is in hand a drag on the canvas is published as the
`tool` key, `{tool, number, phase: "down"|"up", points}` in canvas units.
Show what it would make with `run("tool.preview", { draws })` (canvas units;
`[]` clears it), and on `phase: "up"` commit it, usually with `item.add`.

**Commands.** Each of `commands` is `<mod>/<name>`: listed in the command
palette with its key, run by its key on the canvas, by
`run("<mod>/<name>", args)` or `specular run <mod>/<name>`, and, with
`"dock": true`, a button in the dock while the mod's tool is in hand or its
items are selected (run with `{ids}`, the selected items). Running one
publishes the `call` key, `{command, number, args}`: act once per `number`.
`key` is modifiers and one character joined by `+` (`cmd`, `shift`, `alt`).
`icon` is one of the app's glyphs by its kebab-case name.

**Versions.** `api` is `1` (1.0) or `"1.1"`. A mod loads on an app of the
same generation whose revision is at least its own.

## Host components

Elements a mod places but cannot draw itself, imported from `specular`:

- `Icon`, `Icon.new(id, { name, size, color, tint, active, control })`: a
  panel glyph by its name in the state (`tools[].icon`). `control` names the
  control it sits on, so the window's scripted runs find the button.
- `Zoom`, `Zoom.new(id, { compact })`: the zoom readout and its levels.
- `ToolOptions`, `ToolOptions.new(id, { place })`: the options of the tool
  in hand, inside its button's `relative()` box.
- `Favicon`, `Favicon.new(id, { tab, icon, size, color, control })`: a tab's
  glyph, which becomes the page's own icon once the window has it.
- `WindowDrag`, `WindowDrag.new(id, {})`: bare title bar to drag the window
  by; a double click zooms it as the system says. It fills its box.
- `Share`, `Share.new(id, {})`: the Share slot: the `share` mod when the
  layout names one, else the app's own Share button.
- `Prism`, `Prism.new(id, {})`: the first-run view's turning Prism, 160
  points square.

## The CLI

- `specular mod check <dir> [--state fixture.json]` loads a mod with no
  window, renders it once against a state dump (`specular state > fixture.json`;
  a fresh app's state when none is given), and prints its element tree with
  ids and its `console.log` lines. It exits non-zero and writes
  `<name>.error` when the mod fails.
- `specular mod clone <name> [new-name] [--use]` copies a mod to a folder
  you own (`my-<name>` by default) and prints where it is. `--use` points
  the layout at the copy; the app writes its shipped layout over the file
  on its next start, so run it again then (or keep your own layout edit).
- `specular mod stats` prints, per loaded mod, its script renders, the mean
  and slowest render and the keys it read. A mod that renders while nothing
  changes is a bug.
- `specular shot [file.png]` saves the canvas as a PNG (see "Seeing the
  window").
- `console.log` goes to the app's log (`gpui_shell::script`).

## The shipped mods

| Mod | Draws | Reads | In the shipped layout |
|---|---|---|---|
| `toolbar` | the tools, the zoom readout, the theme button | `panels/toolbar`, `panels/tool_options` | yes |
| `tabs` | the tab row: tabs, add, title bar, zoom, Share, right panel toggle | `panels/strip` | yes |
| `share` | the Share button and its popover | `panels/share` | yes |
| `toast` | the notices, in the canvas's lower right corner | `notices` | yes |
| `menu` | the canvas's context menu | `panels/menu` | no: `"menu": { "mod": "menu" }` |
| `welcome` | the first-run view | `panels/welcome` | no: `"welcome": { "mod": "welcome" }` |

The menu mod leaves the keys with the canvas: Escape closes it, but the
arrows and Return do not move through it as the app's own menu's do.

## Your own mods

A mod folder whose `mod.json` has `"cloned_from"` is yours and is never
replaced. `specular mod clone tabs my-tabs --use` copies a mod to a new
folder, marks it, and points the layout at it; edit `mods/my-tabs/main.js`
and each save reloads it. Without `--use`, set a region's `"mod"` yourself.
Mods are trusted code: a mod has your files, network and processes, like a
shell script you run.

## Check before you finish

```sh
specular mod check ~/Library/Application\ Support/Specular/mods/<name>
specular mod stats
specular shot /tmp/mod.png
```

`check` loads and renders the mod. It exits nonzero with the exact error.
Run it after every change. `stats` must show no renders while nothing
changes. Then look at the picture.

## A canvas mod

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

## What a mod can't do

Check this list before you promise a feature. If something is in neither this
guide nor the types, assume a mod can't do it.

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

## Keep in mind

- One mod per idea. Don't edit this guide or the `.d.ts` files (the app
  rewrites them). Don't touch other mods unless the request needs it.
- Use `colors()` so the mod follows the person's theme.
- Read ids from `specular state` and command results; don't guess them.
