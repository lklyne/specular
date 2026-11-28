# Specular mods

<!-- Written by Specular on every start. Edits here are overwritten. -->

This folder is **where Specular's window and canvas can be changed, as
files**. A mod is a folder with a `mod.json` and a `main.js`: it draws a part
of the window, or adds a kind of canvas item, a toolbar tool or commands, from
the app's state, and changes the app by running commands by name, the same
commands the palette, the keys and the `specular` CLI run. Save a file and the
app reloads the mod within a second. A save that does not load keeps the
old build running and writes why to `mods/<name>.error`, which the next
good save takes away. Shipped mods are Specular's: clone
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
Specular record the screen). Specular must be running with an open window
on the current Space; a minimized window still captures, and one on another
Space makes `--whole` fail and say so.

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
  window"). `default`, `classic` (every panel the app's own), `studio`
  and `mods` (every panel a mod) are shipped and written again on every
  start;
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
size), `canvas/info` (the canvas showing, its project, its counts and every
canvas), `projects` (every project open, see "Projects"), and
for canvas items, tools and commands `canvas/mod_items`, `tool` and `call`
(see below), and for agent threads `chat/threads`, `chat/context` and a
`chat/thread/<id>` key for each thread (see "Chat threads anywhere"), and
`agents`, the agents with a cursor on the canvas (see "Agents on the
canvas").
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
- **project** (1.7): `project.open`, `project.close` `{project: id}`, `project.reveal` `{project: id}`, `project.git` `{project: id, canvases: commit|ignore}`, `project.fold` `{project: id}`, `project.details` `{project: id}`, `project.bind` `{project: id, origin: url}`, `project.unbind` `{origin: url}`, `project.auto-fix` `{origin: url, on: bool}`
- **canvas**: `canvas.new` `{project?: id}`, `canvas.duplicate` `{canvas?: id}`, `canvas.delete` `{canvas?: id}`, `canvas.switch` `{canvas: id}`, `canvas.rename` `{name: text, canvas?: id}`, `canvas.begin-rename` `{canvas?: id}`
- **edit**: `edit.cancel`, `edit.undo`, `edit.redo`, `edit.cut`, `edit.copy`, `edit.paste`, `edit.duplicate`, `edit.delete`, `edit.select-all`, `edit.select` `{ids?: [entity id], edges?: [edge id]}`, `edit.nudge` `{dx: number, dy: number}`
- **item**: `item.add` `{kind: sticky|text|shape|mod, x: number, y: number, text?: string, mod?: name, props?: object, width?: number, height?: number}`, `item.set` `{id: mod item id, props: object}`, `item.draw` `{id: mod item id, draws: [{type: rect|ellipse|path|text|image, ..}]}`
- **text**: `text.bold`, `text.italic`, `text.code`, `text.strikethrough`, `text.bullet-list`, `text.numbered-list`, `text.task-list`, `text.body`, `text.heading-1`, `text.heading-2`, `text.heading-3`, `text.heading-4`, `text.heading-5`, `text.heading-6`
- **arrange**: `arrange.bring-forward`, `arrange.send-backward`, `arrange.bring-to-front`, `arrange.send-to-back`, `arrange.group`, `arrange.ungroup`, `arrange.auto-layout`, `arrange.lay-out` `{mode: row|column|grid}`, `arrange.group-layout` `{axis: row|column|none}`, `arrange.group-gap` `{gap: number}`
- **comment**: `comment.annotate-selection`, `comment.resolve` `{id?: comment id}`, `comment.delete` `{id?: comment id}`, `comment.focus` `{id?: comment id}`, `comment.reveal` `{id: comment id}`
- **page**: `page.new-tab`, `page.edit-address`, `page.back`, `page.forward`, `page.reload`, `page.stop`, `page.navigate` `{url: address, page?: entity id}`, `page.frame`, `page.frame-color`, `page.rotate`, `page.scheme`, `page.orientation` `{orientation: portrait|landscape}`, `page.toggle-sync`
- **tab**: `tab.show` `{item?: entity id}`, `tab.open` `{item: entity id}`, `tab.close` `{item?: entity id}`, `tab.next`, `tab.previous`, `tab.lens` `{lens: fill|device}`, `tab.others` `{shown: bool}`
- **tool**: `tool.set` `{tool: select|page|text|sticky|document|shape|draw|comment|inspect|<mod>/<tool>}`, `tool.ellipse`, `tool.diamond`, `tool.highlighter`, `tool.preview` `{draws: [{type: rect|ellipse|path|text|image, ..}]}`, `tool.stroke` `{tool?: <mod>/<tool>, draws: [{type: path, stroke, line, taper?, speed?}]}`
- **view**: `view.zoom-in`, `view.zoom-out`, `view.zoom-reset`, `view.zoom-to-fit`, `view.zoom` `{percent: number}`, `view.camera` `{x: number, y: number, zoom: number}`, `view.follow` `{agent?: agent id}`, `view.reveal` `{id: entity id}`, `view.theme` `{theme: light|dark|system}`, `view.toggle-sidebar`
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
`radius`, `padding`, `gap` and `material`. The ids `tabs`, `toolbar`, `sidebar` and
`chat` follow the app: the sidebar block shows while the sidebar is open,
the chat block takes the panel's width, and `ui.toggle {id}` hides any
block by its id. Blocks above or left of the canvas lie over its edge and
the canvas is seen between them; blocks below or right sit beside it.

**Glass and blur.** `"material": "glass"` puts macOS's Liquid Glass under a
block (macOS 26; a blur on an earlier macOS) and `"material": "blur"` a
frosted blur; the canvas behind the block shows through, bent or frosted,
and the block's `radius` rounds it. The material is the system's own view
under everything the window paints, so it shows only where the paint over
it is clear: the app's own panels leave their fill off on a material, a
`background` such as `"#ffffff14"` tints it, and an opaque one hides it.
A mod on a material leaves its own `bg()` off, or translucent. The layout
root's `background` paints round the canvas, under the blocks, so leave it
off where a material should show the canvas. Inside a mod, the `Material`
host component is the same surface in any box.

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
popups and dialogs, 6 by default), `spacing` (the unit the app's padding
and gaps are multiples of, 4 by default), `border_width` (its panel edges,
rules and popup borders, 1 by default) and `background` (shown round the
canvas).
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
  "spacing": 4, "border_width": 1,
  "colors": { "popup": "#fff", "popupBorder": "#000", "hover": "#000",
              "hoverText": "#fff", "on": "#ddd", "text": "#000" } }
```

`radius` scales every corner the app draws (6 is its own; 0 squares them
all, pills included), `spacing` scales the app's padding and gaps (3 is
denser, 5 roomier), `border_width` its lines (2 for a heavy outline, 0 for
none), `font` and `font_size` set the chrome's text, and
`colors` renames any colour `colors()` lists (`#rgb`, `#rgba`, `#rrggbb`,
`#rrggbbaa`). `hover` is a hovered row's fill and `hoverText` its text;
`on` and `onText` a selected one's. A layout's own `radius`, `spacing`,
`border_width`, `font`, `font_size` and `colors` go over its theme's. A mod
reads them all through `colors()` and `look()`: build a panel mod's padding
from `look().spacing` and its lines from `look().borderWidth`, and it
follows the theme as the app's own panels do.

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
drawer. Each open is a new instance of the mod, its `init()` run again, so
an overlay starts empty every time; keep what should outlast it with
`shared(...)` or its store. The shipped `confirm` and `prompt` mods are dialogs any mod or agent
can open (see their `main.js`).

**Text.** `Field.new(id, { id, placeholder, value, autofocus })` is a line of
text the window holds: the keys stay in it while a person types, what is
typed is `shared("field/<id>")`, and each Return moves
`shared("field/<id>/submit")` on. A field is made once for its id and
`autofocus` takes the keys when it is made, so a field that should start
empty and focused each time (a palette's) takes a new id in `init()`. With `{ control }` it is a model field of
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

**Shape tools.** A tool with `"group": "shape"` is listed among the shape
tool's shapes instead of having a button: picking it arms the mod's tool,
and the shape button stays on. Use it for a tool that needs its own drag;
for a new silhouette, add a shape instead (below).

## Canvas items, tools and commands (API 1.1)

A mod can put its own items on the canvas, add a tool to the toolbar, and
add commands to the command palette, the keys and the dock. A mod that does
any of these runs whatever the layout says, out of sight, so its script
renders when its keys change. Write `"api": "1.6"` in its `mod.json` for a `stroke` (`"1.1"` without):

```json
{ "api": "1.6", "title": "Feed",
  "items": { "size": [320, 400], "web": "item.html" },
  "tools": [{ "name": "pen", "title": "Pen", "icon": "pen-line",
              "stroke": [{ "type": "path", "stroke": "#1f2328", "line": 4, "taper": true, "speed": 0.6 }] }],
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
Give the tool a `stroke` (API 1.6): paths with no points, which the app
draws over the live stroke every frame, as fast as its own tools. Then the
script has nothing to do until `phase: "up"`, when it commits, usually with
`item.add`; the app draws the stroke until then. `run("tool.stroke", {
tool, draws })` changes the brush for this run (a colour button). A tool
with no `stroke` draws its own preview with `run("tool.preview", { draws })`
(canvas units; `[]` clears it): that follows the `tool` key, so it trails
the pointer by the frames the script takes.

**Brush paths.** An open path with `taper: true` narrows to a point at both
ends; `speed` (0 to 1) thins it where its points are far apart, where the
pointer moved fast. It is filled in its `stroke` colour, `line` wide at its
widest. Layer several, faint and wide under dark and narrow, for a wash.

**Commands.** Each of `commands` is `<mod>/<name>`: listed in the command
palette with its key, run by its key on the canvas, by
`run("<mod>/<name>", args)` or `specular run <mod>/<name>`, and, with
`"chat": true`, a button in the agent panel's header for the open thread
(see "Chat threads anywhere"); `"dock": true`, a button in the dock while the mod's tool is in hand or its
items are selected (run with `{ids}`, the selected items). Running one
publishes the `call` key, `{command, number, args}`: act once per `number`.
`key` is modifiers and one character joined by `+` (`cmd`, `shift`, `alt`).
`icon` is one of the app's glyphs by its kebab-case name.

**Versions.** `api` is `1` (1.0), `"1.1"`, `"1.2"` (the whole window,
above), `"1.3"` (chat threads anywhere, below) `"1.4"` (shapes, below), `"1.5"` (agents) or `"1.6"` (a tool's `stroke`,
`tool.stroke`, brush paths). A mod loads on an app of the same generation whose revision is at
least its own.

## Shapes (API 1.4)

A new silhouette is a shape, not a mod item. List it under `shapes` and
the app does the rest as it does for a hexagon: the shape tool's list
shows it, a drag draws it, and it takes the palette's colours, the fill
and border styles, the border width, a label and the theme. Its outline is
kept in the `.canvas` file, so a person without the mod still sees it
drawn and styled.

```json
{ "api": "1.4", "title": "Star",
  "shapes": [{ "name": "star", "title": "Star", "icon": "star",
               "path": "M50,0 L63.2,31.8 L97.6,34.5 L71.4,57 L79.4,90.5 L50,72.5 L20.6,90.5 L28.6,57 L2.4,34.5 L36.8,31.8 Z",
               "params": { "points": 5 } }],
  "commands": [{ "name": "points", "title": "More star points", "icon": "plus", "dock": true }] }
```

`path` is SVG path data in a 0 to 100 box, stretched to the shape's rect:
`M`, `L`, `H`, `V`, `Q`, `C` and `Z`, absolute or relative (no arcs; write
a curve with its control points). Colours, widths and dashes are the
app's: don't put them in the path. `params` are the values a new shape
starts with; only your mod reads them.

To change shapes after they are drawn, add a dock command (it shows
beside their style while they are selected) and answer its `call` with
`run("shape.set", { ids, params, path })`: new values and a new outline,
one undo step. `canvas/items` gives each shape's `custom`, `{shape,
params}`, to read the values back. `item.add {kind: "shape", shape:
"star/star"}` places one by name. The shipped `star` mod is the whole
example, in about thirty lines.

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
and starts it under that id, not opened, if it is not made yet, so a view
can name its thread before the first message. `chat.stop {thread}` stops
its run. `images` sends pasted images with
the words, each `{type, data}`: a MIME type (`image/png`, `image/jpeg`,
`image/gif` or `image/webp`) and the bytes in base64, bare or as the `data:`
URL a `FileReader` gives. `chat.new-thread {id, open: false}`
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

**Don't draw a composer yourself.** A panel mod places the `Composer` host
component (see "Host components"): the panel's own composer, sending to
the thread you name. A web page (a web item or an overlay) loads the
shared library beside the mods, `mods/lib/chat.js`, with a classic script
(a `file://` page cannot import a module):

```html
<script src="../lib/chat.js"></script>
<specular-chat thread="thread_abc"></specular-chat>
```

`<specular-chat thread closable>` is the agent panel's thread view whole,
in the panel's look: the header (the title, which switches thread, then
New, Archive, and Close with `closable`), the transcript and the composer.
It changes its own `thread` as a person switches or starts one and fires
`thread-change` with `{thread}`, so the page can keep it; Close fires
`close`. Its parts stand alone too: `<specular-thread thread empty>` draws
a thread live from its key (markdown, images, what the agent made, the
run, the error) and fires `thread` with the thread's value each time it
changes, and `<specular-composer thread placeholder>` is the panel's
composer: queued words, pasted images, the field, the pill and folder from
`chat/context`, the model and Send. It fires a cancelable `send` with
`{text, images, thread}` before it runs `chat.send`; a listener may change
`thread`. `SpecularChat.ready(fn)`, `.followTheme()` (the appearance and
the layout's colours and font), `.markdown(text)`, `.escape(text)` and
`.freshThreadId()` are there too. The colours are the panel's, as CSS
variables named as `colors()` names them (`--sc-sidebar`, `--sc-text`,
`--sc-textMuted`, `--sc-toolbar`, `--sc-input`, `--sc-inputBorder`, ...,
in `mods/lib/chat.css`), which a page may set. The app rewrites
`mods/lib/` on every start.

**Buttons in the agent panel.** A mod command with `"chat": true` in
`mod.json` is a button in the agent panel's header while a thread is open,
the app's panel and the `chat` mod alike, run with `{thread}`: the open
thread's id. `panels/chat` lists them as `actions: [{command, title,
args}]`. The shipped `thread` mod's `place` is one ("Put this thread on
the canvas").

**Threads on the canvas.** The shipped `thread` mod makes each thread a
canvas item: `thread/new` (the palette's "Add a chat thread block") starts
a thread and places a block showing it, `thread/place {thread?}` places
one for a thread (the panel's open one by default), and `thread/panel`
(the dock button) opens a block's thread in the panel. A block is an
`items.web` page (`thread/item.html`): it follows `canvas/mod_items` for
its `props.thread`, `chat/thread/<id>` through the page bridge, and
draws it with `<specular-chat>` from `mods/lib/chat.js` (enter the item to type; Return sends, Shift+Return is
a new line). Until its first message it names a thread nobody has, which
that message starts, and keeps it in its props. Its display list is the title and the last few messages, the
snapshot a saved canvas keeps. Copy it (`specular mod clone thread`) to
draw threads your own way.

The panel itself draws markdown, images and comment chips natively. A mod
that shows a transcript draws it itself in a panel mod (the shipped
`chat` mod shows plain text), or with `<specular-thread>` in a web page.

## Projects (API 1.7)

The sidebar's top level is projects (ADR 0062): folders the person opened,
each a plain folder of canvases or a code repo, with its canvases under it.
A repo keeps its canvases in `.specular/canvases/`, and the person says
whether they go into git (`project.git`). The sites bound to a project send
their comments to the agent working in its folder.

- `projects` lists them: `[{id, label, folder, canvasFolder, repo, branch,
  git, active, canvases: [{id, name, active}]}]`.
- In `panels/sidebar`, each project is a section with `kind: "project"`:
  its head folds it, and `add`, `details`, `reveal`, `close`, `git.commit`,
  `git.ignore` and each origin's `auto` and `remove` are control ids to
  press. `open` asks for a folder to open as another project.
- `project.*` acts on them by id, and `canvas.new {project}` makes a canvas
  in one.

## Sidebar sections (API 1.8)

A mod adds a section to the sidebar, under the projects and their canvases,
with `"section": "sidebar"` in its `mod.json`. It runs whatever the layout
says, and is drawn there in name order: in the app's own sidebar, or where
a sidebar mod places `SidebarSections`. The shipped `notes` and `pages`
mods are the canvas's Notes and Pages, drawn from the `kind: "notes"` and
`kind: "pages"` sections of `panels/sidebar`.

`"starts_off": true` keeps a mod off until a person turns it on in
Settings > Mods (`modsOn` in the preferences file). `notes` and `pages`
start off, so the sidebar is just the projects until one is turned on.
With no section on, the app's own sidebar drops its Canvases head and shows
each project's head.

```json
{ "api": "1.8", "title": "Recent", "section": "sidebar", "starts_off": true }
```

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
- `SidebarSections`, `SidebarSections.new(id, {})`: the section mods (see
  "Sidebar sections"), one under the other. A sidebar mod places it.
- `Prism`, `Prism.new(id, {})`: the first-run view's turning Prism, 160
  points square.
- `Swatch`, `Swatch.new(id, { color, palette, role, size })`: a dot of a
  canvas colour. The panel keys name colours as the `.canvas` file stores
  them (`"neutral"`, `"3"`), which `bg()` refuses: draw them with this, with
  the swatches control's `palette` and `role`.
- `Pattern`, `Pattern.new(id, { paint, width, height })`: a box filling
  its parent, painted with a paint (see "Patterns"): pinstripes, hatching,
  checks. Put it first in a `relative()` box and the content after it.
- `Popover`, `Popover.new(id, { top, left, right, bottom }).child(...)`: a
  box over the rest of the window, out of the flow at those offsets from its
  `relative()` box. A press on it is its children's alone, so a dropdown
  that hangs over the canvas does not also press the canvas (and drop the
  selection). Put a mod's dropdowns and menus in one.
- `Composer`, `Composer.new(id, { id, thread, placeholder, autofocus })`:
  the agent panel's composer for any thread: the field, pasted images, the
  context pill, the folder, the send mode, the model and Send. Return or
  Send sends to `thread` (started under that id if it is not made yet), or
  to the thread the panel has open when there is none. The window keeps it
  by `id`, so it redraws as a person types without the mod rendering.
- `Material`, `Material.new(id, { kind, radius, id })`: glass (`kind:
  "glass"`, the default) or a blur (`"blur"`) filling its parent, as a
  block's `material` is (see "Glass and blur"). Put it first in a
  `relative()` box and leave the boxes over it clear or translucent; give
  each one in a mod its own `id`. Headless it is a placeholder.
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
  you own (`my-<name>` by default) and prints where it is. `--use` names
  the copy wherever your own layout, `layouts/mine.json` (started from the
  shipped one), named the original: a block's `content` or an overlay. A
  mod the layout does not name goes in the block its `mod.json` `place`
  names. It then asks a running app to show `mine`, which the app never
  writes over.
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
| `tabs` | the tab row: tabs, add, title bar, zoom, the agents on the canvas, Share, right panel toggle | `panels/strip`, `agents` | yes |
| `share` | the Share button and its popover | `panels/share` | yes |
| `toast` | the notices, in the canvas's lower right corner | `notices` | yes |
| `menu` | the canvas's context menu | `panels/menu` | `mods` layout |
| `welcome` | the first-run view | `panels/welcome` | no: `"welcome": { "mod": "welcome" }` |
| `sidebar` | the projects and their canvases, a New project footer, and the section mods | `panels/sidebar` | `mods`, `studio` |
| `notes`, `pages` | the canvas's Notes and Pages, as sidebar sections | `panels/sidebar` | off until turned on (`starts_off`) |
| `dock` | the selection's or the tool's controls, and the lens | `panels/dock`, `panels/tool_options`, `panels/toolbar` | `mods`, `studio` |
| `palette` | the command palette, in an overlay | `panels/palette` | `mods`, `studio` |
| `chat` | the agent threads, transcript and composer | `panels/chat` | `mods`, `studio` |
| `status` | a status line: canvas, counts, selection, layout | `canvas/info`, `selection`, `layouts` | `studio` |
| `confirm`, `prompt` | dialogs a mod or agent opens with `openOverlay` | `props()` | opened when asked |
| `star` | a star among the shape tool's shapes | `tool`, `canvas/mod_items`, `call` | always (a canvas mod) |
| `thread` | a chat thread as a canvas item, live, with a composer | `canvas/mod_items`, `chat/thread/<id>`, `call` | always (a canvas mod) |
| `feed` | image-feed canvas items, a palette entry and a Shuffle command | `canvas/mod_items`, `call` | always (a canvas mod) |
| `watercolor` | a brush the app draws, its strokes as items, a colour command | `tool`, `canvas/mod_items`, `call` | always (a canvas mod) |

The menu mod leaves the keys with the canvas: Escape closes it, but the
arrows and Return do not move through it as the app's own menu's do.

## Your own mods

A mod folder whose `mod.json` has `"cloned_from"` is yours and is never
replaced. `specular mod clone tabs my-tabs --use` copies a mod to a new
folder, marks it, and points the layout at it; edit `mods/my-tabs/main.js`
and each save reloads it. Without `--use`, set a block's
`"content": {"mod": "my-tabs"}` yourself.
When a release stops shipping a mod, its folder goes on the next start,
unless you changed or added a file there; then it stays, as yours.
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

**The app draws the stroke, the mod commits it.** Give the tool a `stroke`
in `mod.json`, and draw the item with the same paths, so the stroke looks
the same before and after release. While the tool is in hand the drag is the
`tool` key, `{tool: "<mod>/<tool>", number, phase: "down"|"up", points}` in
canvas units. Ignore `down`. On `up`, place what the stroke made with
`item.add` (one undo step, which also ends the app's drawing of it), with
its points made relative to the item's top-left; send `tool.preview` with
`[]` instead when there is nothing to place. See `watercolor/main.js`.

**Props are the item's data.** `item.set {id, props}` merges keys into an
item's props as one undo step (`null` removes a key). Keep everything the item
needs to draw itself in its props. The last display list is saved in the
`.canvas` file as the item's snapshot, so a person without the mod still sees
it.

**Web items read their props through the bridge.** An `items.web` page is
opened as `file://<mod>/item.html#item=<id>`. Wait for `window.specular` (it
may arrive after your script, with a `specular-ready` event), then
`specular.subscribe("canvas/mod_items", ...)` and find your id. Still send a display list from `main.js`: it is the
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
no hover, double click, modifiers, pressure, wheel or keys. A brush is paths
only, and its width follows the gaps between points, not pen pressure. A
preview the script sends (`tool.preview`) trails the pointer by the frames
the script takes; only a `stroke` keeps up. A shape's outline is one closed silhouette: no arcs,
no inner lines drawn apart from it, and no style of its own. Its glyph in
the shape tool's list is one of the app's.

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

## Agents on the canvas

Every agent that drives the app (the CLI in a Claude Code session, or a
thread in the chat panel) has one cursor, on pages and on the canvas alike.
The `agents` key lists the agents with a cursor on the canvas showing:

```json
{ "agents": [{ "id": "…", "name": "Agent 1", "color": "#3b82f6", "label": "moving" }],
  "following": null }
```

`name` is the thread's title for a chat thread, the name the caller gave
(`SPECULAR_CLIENT_NAME`) or "Agent N". `color` is the cursor's. The key holds
no positions, so it moves when an agent comes, goes or starts doing something
else, never per frame.

`run("view.follow", { agent: id })` keeps the camera on that agent's cursor,
with a frame in its colour round the canvas, until the person pans, zooms or
presses Escape, or the agent goes; `run("view.follow", {})` stops. The shipped
`tabs` mod draws the agents as badges beside Share (`"hidden": ["agents"]`
leaves them out); a mod draws them anywhere else from the same key.

