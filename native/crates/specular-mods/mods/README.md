# Mods

A mod is a folder with a `mod.json` and a `main.js`. It draws a part of the
Specular window from the app's state and changes the app by running commands
by name. A save reloads it at once; a save that does not load keeps the last
good build running. This folder is rewritten on every start: edit a copy (see
"Your own mods").

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

Put it in a region with the layout file, `../layouts/default.json`:

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
  `dock`, `sidebar`, `chat`), whether a region is hidden, and the `props` each
  mod gets. `"mod": null` is the app's own panel.
- `../layouts/.state/`: what was hidden and shown at run time
  (`ui.toggle`), apart from the file you edit.

## What a mod reads

`state(key)` returns a key's value and subscribes the mod to it: the script
renders again when that key changes and at no other time, never per frame.
The camera is in no key, so panning and zooming never run a script; the zoom
readout is the `Zoom` host component. The keys: `panels/toolbar`,
`panels/tool_options`, `panels/dock`, `panels/strip`, `panels/menu`,
`panels/palette`, `panels/share`, `theme`, `settings`, `keys`, `layout`.
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
- **item**: `item.add` `{kind: sticky|text|shape, x: number, y: number, text?: string}`
- **text**: `text.bold`, `text.italic`, `text.code`, `text.strikethrough`, `text.bullet-list`, `text.numbered-list`, `text.task-list`, `text.body`, `text.heading-1`, `text.heading-2`, `text.heading-3`, `text.heading-4`, `text.heading-5`, `text.heading-6`
- **arrange**: `arrange.bring-forward`, `arrange.send-backward`, `arrange.bring-to-front`, `arrange.send-to-back`, `arrange.group`, `arrange.ungroup`, `arrange.auto-layout`, `arrange.lay-out` `{mode: row|column|grid}`, `arrange.group-layout` `{axis: row|column|none}`, `arrange.group-gap` `{gap: number}`
- **comment**: `comment.annotate-selection`, `comment.resolve` `{id?: comment id}`, `comment.delete` `{id?: comment id}`, `comment.focus` `{id?: comment id}`, `comment.reveal` `{id: comment id}`
- **page**: `page.new-tab`, `page.edit-address`, `page.back`, `page.forward`, `page.reload`, `page.stop`, `page.navigate` `{url: address, page?: entity id}`, `page.frame`, `page.frame-color`, `page.rotate`, `page.scheme`, `page.orientation` `{orientation: portrait|landscape}`, `page.toggle-sync`
- **tab**: `tab.show` `{item?: entity id}`, `tab.open` `{item: entity id}`, `tab.close` `{item?: entity id}`, `tab.next`, `tab.previous`, `tab.lens` `{lens: fill|device}`, `tab.others` `{shown: bool}`
- **tool**: `tool.set` `{tool: select|page|text|sticky|document|shape|draw|comment|inspect}`, `tool.ellipse`, `tool.diamond`, `tool.highlighter`
- **view**: `view.zoom-in`, `view.zoom-out`, `view.zoom-reset`, `view.zoom-to-fit`, `view.zoom` `{percent: number}`, `view.camera` `{x: number, y: number, zoom: number}`, `view.reveal` `{id: entity id}`, `view.theme` `{theme: light|dark|system}`, `view.toggle-sidebar`
- **ui**: `ui.toggle` `{id: tabs|toolbar|dock|sidebar|chat|<control id>}`, `ui.show` `{id: tabs|toolbar|dock|sidebar|chat|<control id>}`, `ui.hide` `{id: tabs|toolbar|dock|sidebar|chat|<control id>}`, `ui.reset`
- **share**: `share.copy-link`, `share.access` `{scope: edit|comment|view|none}`, `share.join` `{link: url, fresh?: bool}`, `share.stop`
- **control**: `control.press` `{id: control name}`

## Host components

Elements a mod places but cannot draw itself, imported from `specular`:

- `Icon`, `Icon.new(id, { name, size, color, tint, active, control })`: a
  panel glyph by its name in the state (`tools[].icon`). `control` names the
  control it sits on, so the window's scripted runs find the button.
- `Zoom`, `Zoom.new(id, { compact })`: the zoom readout and its levels.
- `ToolOptions`, `ToolOptions.new(id, { place })`: the options of the tool
  in hand, inside its button's `relative()` box.

## Tools

- `specular mod check <dir> [--state fixture.json]` loads a mod with no
  window, renders it once against a state dump (`specular state > fixture.json`;
  a fresh app's state when none is given), and prints its element tree with
  ids and its `console.log` lines. It exits non-zero and writes
  `<name>.error` when the mod fails.
- `specular mod stats` prints, per loaded mod, its script renders, the mean
  and slowest render and the keys it read. A mod that renders while nothing
  changes is a bug.
- `console.log` goes to the app's log (`gpui_shell::script`).

## Your own mods

A mod folder whose `mod.json` has `"cloned_from"` is yours and is never
replaced. Copy a shipped mod to a new name, add `"cloned_from": "toolbar"`,
and point the layout at it. Mods are trusted code: a mod has your files,
network and processes, like a shell script you run.
