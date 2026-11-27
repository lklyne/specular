# ADR 0059: The whole window from mods

**Status:** Accepted, 2026-11-27.
**Date:** 2026-11-27
**Related:** [ADR 0055](./0055-chrome-as-files-then-scripted-panels.md), mods and the layout file. [ADR 0057](./0057-mod-items-and-tools.md), mod items and tools. [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), the panel models.

## Context

ADRs 0055 and 0057 made the small panels mods (toolbar, tab row, menu, Share, notices, first-run view) and let mods add canvas items, tools and commands. Herdr goes further: the whole window is built from mods, a person or their agent can add a layout with its own theme and fonts and switch to it, and every key can be rebound. An audit against Herdr found six gaps, in this order:

1. The layout file names five fixed regions. There is no tree, no second layout, and no theme, colours or fonts in it.
2. A mod cannot open an overlay, a popup, a dialog or a drawer.
3. Keys cannot be rebound.
4. A mod cannot hold the keyboard: the canvas takes it back after every command.
5. The sidebar, the chat panel, the selection and the canvas's items are in no state key.
6. The sidebar, dock, palette, chat and dialogs are still Rust.

A goal came with it: an agent told "add this new shape" can do it with the mods skill alone. That needs mod tools that sit with the app's own shapes.

## Decision

**A layout is a tree, one file per layout.**

- `layouts/<name>.json` is one whole window. Nodes are `row`, `column`, `stack` (shows one child, `active`) and `block`. A block's `content` is a mod (`{"mod": "sidebar", "props": {}}`), one of the app's own panels (`{"panel": "sidebar"}`), or `"canvas"`.
- Any node takes `id`, `size` (`{"px": 256}` or `{"weight": 1}`), `min`, `max`, `hidden`, `background`, `border`, `radius`, `padding`, and on a row or column `gap`.
- The root also takes the look: `theme` (`light`, `dark`, `{"light": .., "dark": ..}`, or a file in `themes/`), `colors` (any colour by name), `font` and `font_size` for the chrome. A mod reads them through `colors()` and `look()`, and the app's own panels draw with the same colours.
- The root names what lies over the window as it names blocks: `dock`, `menu`, `share`, `toast`, `palette`, `welcome`.
- **Where the canvas is.** The canvas surface reaches from the window's top-left corner to the canvas block's bottom-right one. Blocks above and to the left lie over it, and what they cover is the canvas's insets; blocks below and to the right sit beside it. That is today's window exactly (tab row and sidebar over, chat beside), and a sidebar moved to the right or a status bar below just narrows the canvas.
- The geometry is a pure function in `specular-interact` (`chrome::solve`), so the window and a headless run place the canvas by the same numbers.
- Three ids follow the app's own state: `sidebar` shows and hides with Command+B, `toolbar` takes no room while the tools float, and `chat` shows with the right panel at the width it was dragged to.
- `layout.switch {name?}` (⌃⇧L) moves to the next layout or the one named. The `layouts` key lists them. Toggles are kept per layout.
- The app ships `default` (the toolbar, tab row and other small panels as mods, the sidebar, dock, palette and chat still the app's own), `mods` (every panel a mod), `classic` (only the app's own panels, the fallback) and `studio` (an example with its own theme, fonts, the mod sidebar on the right and a status bar). `default` moves to the mod panels once they have run in `mods` with nothing missing. They are written again on every start. Every other file in the folder is the person's and is never written. A file in the old five-region shape is read as the tree it means.

**Mods open overlays.** `overlay.open {mod, id, props, kind, width, height, at, edge, frame}` opens a copy of a mod as a `dialog` (centred over a scrim, or near the top with `edge: top`), a `popup` (at a point, closed by a press outside), a `drawer` (along an edge) or a `window` (at a point, with no scrim, until closed). Opening an id that is open replaces it, so one key can toggle a palette. It answers with an id the copy reads as `props().overlay` and closes with `overlay.close {id}`. Escape closes the top dialog or popup. The `overlays` key lists what is open. Overlays are app state, so a headless run and the API see them too.

**`keys.json` rebinds any key.** Sections `global`, `canvas`, `editing`, `page` and `layout:<name>` map a chord (`cmd-shift-k`) to a command (`"view.zoom-in"`, `"tool.set:shape"`, `{"command": .., "args": ..}`), or to `null` to free it. It is read before the built-in table; the more specific section wins, and a mistake is written to `keys.error` beside it. The menus show the chords it sets. A key can run a window command (`app.command-palette`, `app.quit` ..), but the menu's own key equivalents stay as the menu has them. The reset key, ⌃⌥⇧⎋, cannot be rebound.

**A mod can hold the keyboard through a text field.** The `Field` host component is a text field the window draws; its text is published on `shared/field/<id>` and Enter on `shared/field/<id>/submit`. While a field has the focus, keys go to it, Escape still closes the overlay, and running a command does not take the keys back. A `Field` with a `control` prop draws one of the app's own model fields instead, so a dock mod edits a width or a URL as the app's dock does. Mods get no raw key events: a key a mod wants is a command bound in `keys.json`.

**More state.** `panels/sidebar`, `panels/chat`, `selection` (`{ids, edges}`), `canvas/items` (`[{id, kind, label, parent, ..}]`, no positions, so a drag wakes no mod), `canvas/info`, `layouts`, `look` and `overlays`. The camera stays out of every key. The chat panel is driven by `chat.*` commands.

**Mod tools join the shapes.** A mod tool with `"group": "shape"` leaves the toolbar and is listed with the app's shapes in the shape tool's options; while it is armed, the shape button shows active and the dock shows the mod's commands. A drawn mod item takes no pointer or key input of its own: an item that must be interactive is a live page (`items.web`), which already takes both.

**The panels are mods.** The sidebar, dock, palette, chat, status bar and the confirm and prompt dialogs ship as mods on the API a person's mod gets, and `mods` and `studio` use them. The app's own panels stay for `default` and `classic` until the switch.

The mod API is 1.2.

## Alternatives

- **Lay out with GPUI's flexbox.** Simpler to draw, but a headless run, which draws nothing, could not know where the canvas is. The tree is solved once in pure code and the window draws absolutely positioned boxes.
- **Make the canvas a block like any other, with nothing over it.** Cleaner, but every page and pan would move under a sidebar opening, which today it does not. The "over above and left, beside below and right" rule keeps today's behaviour and still lets a layout put things anywhere.
- **Floating blocks with springs and peeks, as Herdr has.** Overlays of kind `drawer` and `window` cover what was asked for with less.
- **Raw pointer and key input for mod items.** It would let a drawn item be a widget, but it puts a script on the input path and doubles what a live page already does. Interactive items are pages.
- **A focus key and raw keys for mods.** A text field drawn by the window covers typing, with IME and selection for free; everything else is a command and a key binding.

## Consequences

- A person or an agent can rebuild the window, its look and its keys from files, and switch between layouts.
- Each overlay and block runs its own copy of a mod: a heap each, small beside the window.
- The app's own panels are now a second implementation of the same panels until they are deleted. Delete each one when its mod has run as the default for a release with nothing missing.
- The render gates (a median script render under 2 ms, none over 8 ms, nothing on idle or camera frames) cover every shipped panel mod.
