# Specular mods

<!-- Written by Specular on every start. Edits here are overwritten. -->

This folder is **where Specular's window and canvas can be changed, as
files**. A mod is a folder with a `mod.json` and a `main.js`: it draws a part
of the window, or adds a kind of canvas item, a toolbar tool or commands, from
the app's state, and changes the app by running commands by name, the same
commands the palette, the keys and the `specular` CLI run. Save a file and the
app reloads the mod within a second. Shipped mods are Specular's: clone
before editing (see "Your own mods"). Every panel of the window has a
shipped mod built from the same parts you have, and a layout file says
which mod (or the app's own panel) goes where.

## What you can and can't do

You can: lay out the whole window as a tree of rows, columns and blocks
in a layout file, several of them, switched at run time; give a layout its
own theme, colours, font and background; redraw every panel (tab row,
toolbar, sidebar, dock, agent panel, palette, menu, notices, Share, first
run) or add your own blocks (a status line); open dialogs, popups, drawers
and windows over the window; type into fields; rebind any key in
`keys.json`; add kinds of canvas items drawn from a display list or shown
as a web page; add toolbar tools and shapes to the shape tool; add commands
to the palette, the keys and the dock; run any app command; read files, run
programs and fetch URLs; keep state shared between mods.

You can't: draw the canvas itself or change the built-in items; read the
camera or a live page's content; get pointer or key input inside a
display-list item (a web item gets both). The full list is "What a mod
can't do" at the end.

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
- **Escape hatch:** Control+Option+Shift+Escape always works: it forgets
  what was hidden at run time and shows the shipped `default` layout. Your
  layout files are left as they are. Nothing can rebind it.
- **Settings > Mods** lists every installed mod with a switch. A mod turned
  off doesn't run: its tools, commands and items go, and a layout that
  places it shows the app's own panel there. The choice is kept between
  launches (`modsOff` in the preferences file).

## Seeing the window

`specular shot [file.png]` saves the canvas as the window shows it as a PNG
and prints its absolute path (default: `specular-shot.png` in your current
folder). Use it when debugging items and tools: inspect spacing, colours and
previews. It captures the canvas, not the panels around it, even when the
window is covered. `specular shot --whole` captures the whole window as the
screen shows it, panels, layout and look included (macOS asks once to let
Specular record the screen). Specular must be running with an open window,
shown on the current Space: a hidden, minimized or other-Space window makes
`--whole` fail and say so.

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

Put it in a block of your own layout, `../layouts/mine.json` (copy
`default.json` to start it, and change the toolbar block's content):

```json
{ "id": "toolbar", "type": "block", "size": { "px": 36 },
  "content": { "mod": "hello", "props": {} } }
```

`layout.switch {name: "mine"}` (or the palette's "Layout: …") shows it.

Then check it with no window: `specular mod check ~/path/to/mods/hello`.

## Where things are

- `mods/<name>/`: a mod. `mods/<name>.error` says why it did not load, and
  goes away on the next good load.
- `mods/specular.d.ts`: the whole `specular` module, with every state key and
  command. `gpui-kit.d.ts`, written into a mod's folder when it loads, has the
  element and style surface (`div`, `h_flex`, `Button`, `.bg(..)`, ...).
- `../layouts/<name>.json`: one whole window each (see "The whole
  window"). `default`, `classic` (every panel the app's own), `studio`,
  `mods` (every panel a mod) and `chats` (two chat panes beside the
  canvas) are shipped and written again on every start;
  any other file is yours and never written. A save applies at once; a bad
  one keeps the last good layout and writes `<name>.error` beside it.
- `../layouts/.state/`: what was hidden and shown at run time
  (`ui.toggle`) for each layout, and `active`, the layout shown last.
- `../themes/<name>.json`: a theme a layout names, the one place a look's
  tokens are set (see "Themes").
- `../keys.json`: your own keys (see "Keys").

## What a mod reads

`state(key)` returns a key's value and subscribes the mod to it: the script
renders again when that key changes and at no other time, never per frame.
The camera is in no key, so panning and zooming never run a script; the zoom
readout is the `Zoom` host component. The keys: `panels/toolbar`,
`panels/tool_options`, `panels/dock`, `panels/strip`, `panels/menu`,
`panels/palette`, `panels/share`, `panels/welcome`, `panels/sidebar`,
`panels/chat`, `theme`, `settings`, `keys`, `layout`, `layouts`, `look`,
`overlays`, `notices`, `selection` (`{ids, edges}`), `canvas/items` (every
item, back to front, with its kind, label, parent and words, but no place or
size), `canvas/info` (the canvas showing, its counts and every canvas), and
for canvas items, tools and commands `canvas/mod_items`, `tool` and `call`
(see below), and for agent threads `chat/threads`, `chat/context` and a
`chat/thread/<id>` key for each thread (see "Chat threads anywhere").
`specular state [key]` prints them from the running app.

`props()` is what the layout hands the mod. By convention `props().hidden`
lists the ids of controls to leave out, and a shipped mod honours it:
`{ "mod": "toolbar", "props": { "hidden": ["tool.inspect", "zoom"] } }`.

`colors()` is the chrome's palette for the theme drawn now (`toolbar`,
`toolbarText`, `popup`, `hover`, `raisedFill`, `accent`, ...), as
`#rrggbbaa` strings, with the layout's theme and `colors` over it. `look()`
is the layout's look: `{appearance, theme, colors, font, fontSize,
radius, background}`.
`cx.theme().colors` has GPUI Kit's generic tokens too.

`shared(key)` / `setShared(key, value)` pass small values between mods.

## What a mod does

`run(name, args)` runs a command, as the palette, a key or the CLI would. A
command that changes the canvas is one undo step. A control in a panel model
is pressed with `run("control.press", { id })`, by the `id` beside it in the
state. `commands()` lists them all; `notify(text, level)` writes a line to the
app's log. The commands:

- **app**: `app.quit`, `app.open-space`, `app.open`, `app.save`, `app.settings`, `app.hide`, `app.hide-others`, `app.minimize`, `app.command-palette`
- **window**: `window.close`, `window.zoom`
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
- **layout**: `layout.switch` `{name?: layout name}` (the next one without a name)
- **overlay**: `overlay.open` `{mod: name, id?, props?, kind?: dialog|popup|drawer|window, width?, height?, at?: [x, y], edge?: left|right|top|bottom, frame?: none}`, `overlay.close` `{id?: overlay id}`
- **chat**: `chat.toggle`, `chat.new-thread` `{id?: thread id, open?: bool}`, `chat.select` `{id: thread id}`, `chat.back`, `chat.close` `{id?: thread id}`, `chat.send` `{text: message, images?: [{type, data}], thread?: thread id}`, `chat.stop` `{thread?: thread id}`, `chat.model` `{name: model name}`
- **notice**: `notice.dismiss` `{id: notice id}`
- **mods**: `mods.install`
- **share**: `share.copy-link`, `share.access` `{scope: edit|comment|view|none}`, `share.join` `{link: url, fresh?: bool}`, `share.stop`
- **control**: `control.press` `{id: control name}`

## The whole window (API 1.2)

**A layout is a tree.** `layouts/<name>.json` is a root node with the
look on it. A node is a `row`, a `column`, a `stack` (shows one child, its
`active` id) or a `block` with `content`: `"canvas"` (exactly one),
`{ "mod": name, "props": {} }` or `{ "panel": "tabs"|"toolbar"|"sidebar"|"chat" }`
(the app's own). Each node may have an `id`, a `size` (`{ "px": 256 }` or
`{ "weight": 2 }`, the rest shared by weight), `min`, `max`, `hidden`,
`background` (a paint: a colour, a colour name such as `"toolbar"`, a
gradient `{ "from": ..., "to": ..., "angle": 90 }`, or a pattern, below), `border` `{width, color}`,
`radius`, `padding` and `gap`. The ids `tabs`, `toolbar`, `sidebar` and
`chat` follow the app: the sidebar block shows while the sidebar is open,
the chat block takes the panel's width, and `ui.toggle {id}` hides any
block by its id. Blocks above or left of the canvas lie over its edge and
the canvas is seen between them; blocks below or right sit beside it.

```json
{ "title": "Mine", "id": "root", "type": "column",
  "theme": "studio", "font": "Geist", "font_size": 12,
  "palette": { "mod": "palette" }, "dock": { "mod": "dock" },
  "toast": { "mod": "toast" }, "share": { "mod": "share" },
  "children": [
    { "id": "tabs", "type": "block", "size": { "px": 38 }, "background": "toolbar",
      "content": { "mod": "tabs" } },
    { "id": "body", "type": "row", "children": [
      { "id": "toolbar", "type": "block", "size": { "px": 36 }, "content": { "mod": "toolbar" } },
      { "id": "canvas", "type": "block", "content": "canvas" },
      { "id": "sidebar", "type": "block", "size": { "px": 260 }, "content": { "mod": "sidebar" } },
      { "id": "chat", "type": "block", "size": { "px": 400 }, "content": { "mod": "chat" } } ] },
    { "id": "status", "type": "block", "size": { "px": 24 }, "content": { "mod": "status" } } ] }
```

**The look.** On the root: `theme` (a theme file's name, or
`{ "light": ..., "dark": ... }`), `colors` (names over the theme's),
`font`, `font_size`, `radius` (the corners of the app's own controls and
popups and dialogs, 6 by default) and `background` (shown round the canvas).
`traffic_lights` moves the window's close, minimize and zoom buttons
(`{ "x": 14, "y": 13 }`, the first button's corner from the window's
top-left) or hides them (`"hidden"`) for a layout that draws its own with
`window.close`, `app.minimize` and `window.zoom`. Left out, they sit where
macOS puts them. `window_border` (`{ "width": 1, "color": "#000" }`)
draws a line round the whole window, over every block, rounded to the
radius macOS gives the window's corners, so a square-cornered look keeps
its edge where the corners cut it; with `"padding": [0, 6]` on the root
the chrome sits clear of the curve. The overlays
`menu`, `share`, `toast`, `palette`, `welcome` and `dock` are keys of the
root too: `{ "mod": name, "props": {} }`, or left out for the app's own.

**Themes.** A theme file holds every token the app's own panels, menus,
popups and dialogs and every mod draw with, so a look is one file:

```json
{ "appearance": "light", "radius": 0, "font": "Geneva", "font_size": 12,
  "colors": { "popup": "#fff", "popupBorder": "#000", "hover": "#000",
              "hoverText": "#fff", "on": "#ddd", "text": "#000" } }
```

`radius` scales every corner the app draws (6 is its own; 0 squares them
all, pills included), `font` and `font_size` set the chrome's text, and
`colors` renames any colour `colors()` lists (`#rgb`, `#rgba`, `#rrggbb`,
`#rrggbbaa`). `hover` is a hovered row's fill and `hoverText` its text;
`on` and `onText` a selected one's. A layout's own `radius`, `font`,
`font_size` and `colors` go over its theme's. A mod reads them all through
`colors()` and `look()`.

**A square-cornered look** (a retro one, say) takes four things: `radius: 0`
in its theme, so every corner the app draws is square; `hover`/`hoverText`
and `on`/`onText` pairs that read on each other (a System 7 menu is
`"on": "#000", "onText": "#fff", "hover": "#bbb", "hoverText": "#000"`);
a root `window_border` in the outline colour, which follows the curve
macOS cuts the window's corners on; and room inside the title and status
bars' blocks (`padding`), so nothing sits under that curve. Check it with
`specular shot --whole`, at each corner.

**Patterns.** A paint can repeat over a base colour: `{ "color": "#ddd",
"pattern": "stripes", "ink": "#bbb", "width": 1, "gap": 1 }` for
pinstripes (`"direction": "vertical"` turns them), `{ "pattern": "hatch",
"ink": "#0002", "width": 1, "gap": 4 }` for diagonal hatching, and `{
"pattern": "checker", "ink": "#0001", "size": 2 }` for a checked desktop.
A layout's `background` and any block's take one; in a mod, the `Pattern`
host component paints its box with one.

**Overlays.** `openOverlay(name, spec)` (or `overlay.open`) puts a copy of a
mod over the window and answers its id: a `dialog` in the middle over a
scrim (`edge: "top"` keeps it near the top), a `popup` at `at` that a press
outside closes, a `drawer` along an `edge`, or a `window` that stays. The
copy reads its id as `props().overlay` and closes itself with
`closeOverlay(props().overlay)`. Escape closes the top dialog, popup or
drawer. The shipped `confirm` and `prompt` mods are dialogs any mod or agent
can open (see their `main.js`).

**Text.** `Field.new(id, { id, placeholder, value, autofocus })` is a line of
text the window holds: the keys stay in it while a person types, what is
typed is `shared("field/<id>")`, and each Return moves
`shared("field/<id>/submit")` on. With `{ control }` it is a model field of
the dock (an address, a size). `radius`, `background`, `border` and
`focusRing` restyle its box to match the layout; a styled field shows its
focus by turning its own edge `focusRing`, with no rounded ring outside it.
A layout's `colors` can rename `focusRing` for every field and control.
Commands a mod runs while one of its fields has the keys leave them there.

**Keys.** `keys.json` beside the layouts gives keys to commands, over the
app's: sections `global`, `canvas`, `editing`, `page` and
`layout:<name>`; each key names a command (`"layout.switch"`), a command
with arguments (`"tool.set:{\"tool\":\"shape\"}"` or
`{ "command": ..., "args": {} }`), or `null` to take the app's key away.
Keys are written as `cmd+shift+k`, `alt+1`, `shift+tab`, `escape`. The
`keys` key lists yours first, and the menus show them.

```json
{ "global": { "cmd+shift+l": "layout.switch" },
  "canvas": { "s": "tool.set:{\"tool\":\"shape\"}", "r": null } }
```

**Shapes.** A tool with `"group": "shape"` is listed among the shape tool's
shapes instead of having a button: picking it arms the mod's tool, and the
shape button stays on. See the shipped `star` mod: a tool and an item kind
that make a new shape in about eighty lines.

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

**Versions.** `api` is `1` (1.0), `"1.1"`, `"1.2"` (the whole window,
above) or `"1.3"` (chat threads anywhere, below). A mod loads on an app of the same generation whose revision is at
least its own.

## Chat threads anywhere (API 1.3)

The agent panel shows one thread at a time. A mod can show any number,
each on its own: in a canvas item, in a pane beside the panel, in an
overlay window, several side by side.

**Reading.** `chat/threads` is the active canvas's threads with no
transcripts: `{open, threads: [{id, key, title, draft, open, running,
updatedAt}], model: {label, options}}`. `open` is the thread the panel has
open, if any. It moves when a thread starts, is renamed, opens, or starts
or stops working, not on every word. Each thread of the active canvas
also has its own key, `chat/thread/<id>`:

```
{ id, title, draft, open, running, updatedAt,
  messages: [{id, role: "user"|"agent", text, images, artifacts}],
  streaming, making, run: {label, log} | null, error,
  queued: [text], canSendEmpty, placeholder, emptyHint }
```

A thread's key moves only with that thread, so a pane showing one thread
does not render while another streams. It reads `null` for a thread that
is closed, on another canvas, or not made yet.

`chat/context` is what the next message goes with, as the panel's composer
shows it under its field: `{pill: {label, kind: "dom"|"comment"|"selection"|"canvas"},
folder, folderPath, auto: {origin, on} | null, model: {label, options: [{name,
value, description, chosen}]}}`. Every view's message goes with the same
context. `chat.model {name}` takes an option's `value`.

**Acting on one thread.** `chat.send {text, thread}` sends to that thread
whichever the panel has open (the panel's comment draft is left alone),
and `chat.stop {thread}` stops its run. `images` sends pasted images with
the words, each `{type, data}`: a MIME type (`image/png`, `image/jpeg`,
`image/gif` or `image/webp`) and the bytes in base64, bare or as the `data:`
URL a `FileReader` gives. The shipped `thread` block takes them from a paste. `chat.new-thread {id, open: false}`
starts a thread without opening it in the panel. The mod picks the id
(letters, digits, `_` and `-`, at most 64), so it knows it at once, since
`run` answers before the command runs:

```js
const id = `thread_${Date.now().toString(36)}`;
run("chat.new-thread", { id, open: false });
run("chat.send", { text: "Sketch three layouts", thread: id });
```

`chat.select {id}` opens a thread in the panel, and `chat.close {id}`
archives it.

**Panes.** The shipped `chat` mod with `props.pane` is a pane of its own:
it keeps its thread in `shared("chat/pane/<pane>")`, lists threads from
`chat/threads`, and sends to its thread, so two blocks with two panes
show two threads side by side. `props.thread` pins a pane to one
thread. The shipped `chats` layout is the canvas with two panes:

```json
{ "id": "chats", "type": "row", "size": { "px": 760 }, "children": [
  { "id": "chat.a", "type": "block", "content": { "mod": "chat", "props": { "pane": "a" } } },
  { "id": "chat.b", "type": "block", "content": { "mod": "chat", "props": { "pane": "b" } } } ] }
```

A pane opens as a floating window too:
`openOverlay("chat", { kind: "window", width: 380, height: 520, props: { pane: "float" } })`.

**Threads on the canvas.** The shipped `thread` mod makes each thread a
canvas item: `thread/new` (the palette's "Add a chat thread block") starts
a thread and places a block showing it, `thread/place {thread?}` places
one for a thread (the panel's open one by default), and `thread/panel`
(the dock button) opens a block's thread in the panel. A block is an
`items.web` page (`thread/item.html`): it follows `canvas/mod_items` for
its `props.thread`, `chat/thread/<id>` through the page bridge, and
draws the transcript with markdown, images and what the agent made, and a
composer with the panel's context pill, folder, model and Send (enter the
item to type; Return sends, Shift+Return is a new line). Its display list is the title and the last few messages, the
snapshot a saved canvas keeps. Copy it (`specular mod clone thread`) to
draw threads your own way.

The panel itself draws markdown, images and comment chips natively. A mod
that shows a thread draws them itself: a web item or overlay in HTML, a
panel mod in GPUI elements (the shipped `chat` mod shows plain text).

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
- `Swatch`, `Swatch.new(id, { color, palette, role, size })`: a dot of a
  canvas colour. The panel keys name colours as the `.canvas` file stores
  them (`"neutral"`, `"3"`), which `bg()` refuses: draw them with this, with
  the swatches control's `palette` and `role`.
- `Pattern`, `Pattern.new(id, { paint, width, height })`: a box filling
  its parent, painted with a paint (see "Patterns"): pinstripes, hatching,
  checks. Put it first in a `relative()` box and the content after it.
- `Field`, `Field.new(id, { id, placeholder, value, autofocus })` or
  `Field.new(id, { control })`: a line of text (see "Text"), restyled by
  `radius`, `background`, `border` and `focusRing`.

## The CLI

- `specular mod check <dir> [--state fixture.json]` loads a mod with no
  window, renders it once against the running app's state (or a fresh
  app's when none answers, or `--state fixture.json` from `specular state >
  fixture.json`), and prints its element tree with
  ids and its `console.log` lines. It exits non-zero and writes
  `<name>.error` when the mod fails.
- `specular mod clone <name> [new-name] [--use]` copies a mod to a folder
  you own (`my-<name>` by default) and prints where it is. `--use` points
  the layout at the copy; the app writes its shipped layout over the file
  on its next start, so run it again then (or keep your own layout edit).
- `specular mod stats` prints, per loaded mod, its script renders, the mean
  and slowest render and the keys it read. A mod that renders while nothing
  changes is a bug.
- `specular shot [file.png] [--whole]` saves the canvas, or the whole
  window, as a PNG (see "Seeing the window").
- `console.log` goes to the app's log (`gpui_shell::script`).

## The shipped mods

| Mod | Draws | Reads | In the shipped layout |
|---|---|---|---|
| `toolbar` | the tools, the zoom readout, the theme button | `panels/toolbar`, `panels/tool_options` | yes |
| `tabs` | the tab row: tabs, add, title bar, zoom, Share, right panel toggle | `panels/strip` | yes |
| `share` | the Share button and its popover | `panels/share` | yes |
| `toast` | the notices, in the canvas's lower right corner | `notices` | yes |
| `menu` | the canvas's context menu | `panels/menu` | `mods` layout |
| `welcome` | the first-run view | `panels/welcome` | no: `"welcome": { "mod": "welcome" }` |
| `sidebar` | the canvases, notes and pages | `panels/sidebar` | `mods`, `studio` |
| `dock` | the selection's or the tool's controls, and the lens | `panels/dock`, `panels/tool_options`, `panels/toolbar` | `mods`, `studio` |
| `palette` | the command palette, in an overlay | `panels/palette` | `mods`, `studio` |
| `chat` | the agent threads, transcript and composer; with `props.pane`, one thread of its own | `panels/chat`, or `chat/threads` and `chat/thread/<id>` | `mods`, `studio`; panes in `chats` |
| `status` | a status line: canvas, counts, selection, layout | `canvas/info`, `selection`, `layouts` | `studio` |
| `confirm`, `prompt` | dialogs a mod or agent opens with `openOverlay` | `props()` | opened when asked |
| `star` | a star among the shape tool's shapes | `tool`, `canvas/mod_items`, `call` | always (a canvas mod) |
| `thread` | a chat thread as a canvas item, live, with a composer | `canvas/mod_items`, `chat/thread/<id>`, `call` | always (a canvas mod) |

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

**The window.** Every panel can be a mod, but the canvas itself is the
app's: a mod can't draw into it, change selection outlines, handles,
snapping or hit-testing. The settings window, the macOS menu bar, the page
inspector and the page tools' own popups stay the app's. The shipped panel
mods cover the common paths; the app's own panels still do more (dragging
rows, a canvas row's menu, images and comment chips in the composer,
markdown in the transcript), so `default` keeps them until the mods catch up.
Threads are read for the active canvas only: a thread of another canvas
reads `null` until that canvas shows, and a thread is sent the panel's
context (the canvas and the selection) whichever view sent to it.

**The canvas.** A mod can't restyle the built-in items or tools, or read the
camera, item positions, or the content of live web pages. `canvas/items`
says what is there, not where.

**Items.** A display-list item can't be entered and gets no pointer, wheel
or key input; an `items.web` item is a live page and gets all of them once
entered. A display list has rects, ellipses, paths, text and images only: no
gradients, shadows, blurs, clipping, transforms, fonts or rich text.
Resizing doesn't scale it. `item.draw` isn't an undo step, and several
`run()` calls are several undo steps. The web viewer and sync peers show
snapshots only.

**Tools and shapes.** A tool gets one press-to-release drag in canvas units:
no hover, double click, modifiers, pressure, wheel or keys. The preview is
one render behind. A mod shape is a mod item, not a built-in shape: it has
no label to type in and none of the shape dock's controls unless its mod
adds them, and its glyph is one of the app's.

**Keys and focus.** `keys.json` can rebind every key but the reset key. The
window's own keys (Quit, Open, Save, Settings, Hide, Minimize, the palette)
stay menu keys: a `keys.json` key for one of their chords works over the
canvas but the menu still takes it elsewhere. A mod can't take the keys
except through a `Field`, or read a key pressed elsewhere.

**Running.** A mod runs only when installed in the app's mods folder; a canvas
never runs one. Install mods installs only from the canvas's space folder.

## Keep in mind

- One mod per idea. Don't edit this guide or the `.d.ts` files (the app
  rewrites them). Don't touch other mods unless the request needs it.
- Use `colors()` so the mod follows the person's theme.
- Read ids from `specular state` and command results; don't guess them.
