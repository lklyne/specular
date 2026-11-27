# specular mods: change the person's window

Specular shows canvases of pages, notes and items. Mods draw parts of its
window and add kinds of canvas items, toolbar tools and commands. Use mods
only to change what the app is or does. For putting things on a canvas,
arranging them or driving pages, use the specular skill and its verbs.

## Inspect before acting

```sh
specular state            # every key a mod can read, from the running app
specular state theme      # one key
specular commands         # every command, its arguments and whether it runs now
specular mod stats        # loaded mods: renders, render times, keys read
```

These talk to the running app. Do not start, stop or restart the person's
app. If `state` fails, the app isn't running: say so. `specular mod check`
and `specular mod clone` work without it.

## Pick the kind of mod

| You want | Write | Copy |
|---|---|---|
| A panel drawn differently (toolbar, tab row, notices, menu) | a panel mod, placed by the layout | `toolbar`, `toast` |
| A new kind of canvas item | `items` in `mod.json`, a display list from `item.draw` | `feed` |
| A canvas item that is live web content | `items.web`, an HTML file in the mod | `watercolor` |
| A tool in the toolbar | `tools` in `mod.json`, following the `tool` key | `ink` |
| A command in the palette, on a key or in the dock | `commands` in `mod.json`, acting on the `call` key | `feed` |

One mod can be several of these. `ink` is a tool, an item kind and a command.

## Customize the window, items, tools or commands

First read `<app home>/mods/README.md`. It is the installed guide to mods:
the files, every state key and command, the patterns, and what a mod can't
do. Read `mods/specular.d.ts` beside it for the full API. Don't edit either:
the app rewrites them on start. `<app home>` is
`~/Library/Application Support/Specular` on macOS.

- Shipped mods belong to Specular: copy one with
  `specular mod clone <name> <new-name>` before editing. A clone is never
  overwritten; a shipped mod's folder is rewritten on every start.
- `mod.json` says what the mod adds: `items`, `tools`, `commands`,
  `palette`. Write `"api": "1.2"`: it adds overlays, `Field`, the look,
  shape tools and the window's keys to 1.1's items, tools and commands.
  Unknown fields are refused, and the error says which.
- A mod renders from state only. `state(key)` subscribes it to that key; it
  renders again when the key changes and at no other time. Never loop, poll
  or set a timer to redraw.
- `run(name, args)` queues a command and returns at once. A command that
  changes the canvas is one undo step.
- Draw an item once per change: keep what you last sent and send
  `item.draw` again only when its size or props change. A move wakes no
  script.
- The `tool` and `call` keys carry a `number` that counts up. Act once per
  number, and skip the value you see on load, so a reload doesn't replay.
- A tool previews with `tool.preview` while the drag is `down` and commits
  with `item.add` on `up`.
- Keep an item's data in its props (`item.set`). Its last display list is
  saved in the `.canvas` file as its snapshot, so a person without the mod
  still sees it.
- Keep display lists small: tens of marks, not thousands.
- Save to reload. A broken save keeps the old build running and writes
  `<name>.error` beside the folder; a still-working screen is not proof the
  edit worked.
- Before you promise a feature, check the guide's "What a mod can't do". If
  something is in neither the guide nor the types, assume a mod can't do it.
- Make only the requested change. Don't touch unrelated mods or layouts.

## Common asks (API 1.2)

- **A new shape.** A mod with a tool of `"group": "shape"` and an item: the
  tool shows in the shape tool's options, previews with `tool.preview`,
  places with `item.add {kind: "mod"}`, and its commands show in the dock.
  The shipped `star` mod is the pattern: `specular mod clone star <name>`.
- **A layout, theme, colours or fonts.** Copy a file from
  `<app home>/layouts/` to a new name, edit the tree and its root's
  `theme`, `colors`, `font` and `font_size`, then `specular run
  layout.switch '{"name":"<name>"}'`. The shipped layouts are rewritten
  on start; your copy is not.
- **A key.** Add it to `<app home>/keys.json` under `global`, `canvas`,
  `editing`, `page` or `layout:<name>`. A mistake is written to
  `keys.error`; `specular state keys` shows the result.
- **A dialog, popup or drawer.** `run("overlay.open", {mod, kind, props})`
  opens a copy of a mod; the shipped `confirm` and `prompt` mods are ready
  to use. Text input is the `Field` host component.
- **A panel.** Clone the shipped `sidebar`, `dock`, `palette`, `chat` or
  `status` mod and name the clone in a layout.

## After every mod change

```sh
specular mod check ~/Library/Application\ Support/Specular/mods/<name>
specular mod stats
specular shot /tmp/mod.png   # then look at the picture
```

`check` loads the mod with no window, renders it once against the running
app's state (a fresh app's when none answers; it says which on stderr), or
`--state fixture.json` from `specular state > fixture.json`, and
prints its element tree and its `console.log` lines. It exits nonzero with
the exact error and writes `<name>.error`. Never call a mod done without a
passing check.

`stats` must show no renders while nothing changes. A mod that renders when
idle is a bug.

Then run what you built (`specular run tool.set '{"tool":"<mod>/<tool>"}'`,
`specular run <mod>/<command>`) and confirm it in the running app with
`shot`; don't claim a visual check you haven't made.

`specular shot [file.png]` saves the canvas as the window shows it and
prints its absolute path (default: `specular-shot.png` in your current
folder). It has your items and your tool's preview, not the panels.
`specular shot --whole [file.png]` saves the whole window as the screen
shows it, panels, layout and theme included: use it to check a panel mod,
a layout or a look. macOS asks the person once to let Specular record the
screen; if the shot fails, ask them to allow it. Both need a running app
with an open window.

## Problems and safety

A mod is trusted code: it runs with the person's files, network and
processes. Don't add network calls, file writes outside the mod's folder, or
processes the person didn't ask for.

Placing a mod's first item on a canvas copies the mod into that space's
`mods/` folder. Say so when you do it in someone's space. A canvas never runs
a mod by itself: a person who opens it without the mod sees snapshots and an
"Install mods" offer.

Before you install a mod someone else wrote, from a link, a folder or that
offer, read every file and tell the person what it does with files, the
network and processes.

Control+Option+Shift+Escape always brings the shipped layout back. Read ids
and values from `specular state` and command results; don't guess item ids.
