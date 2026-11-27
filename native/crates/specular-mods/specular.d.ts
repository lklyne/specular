// The `specular` module (ADR 0055): what a mod imports to reach the app.
// This file is checked in and written into the mods folder on every start.
// `api` 1.2. Within a generation the surface only grows: 1.1 added canvas
// items, tools and commands (ADR 0057); 1.2 added the window's own panels,
// overlays, the look and text fields (ADR 0059).

/**
 * Every state key. `panels/*` are the app's own panel models as plain data;
 * a control's `id` is what `run("control.press", { id })` takes. The camera
 * is in no key, so a pan or a zoom never wakes a mod: the zoom readout is the
 * `Zoom` host component. A mod's own props are not a key it names: read them
 * with `props()`.
 *
 * - `panels/toolbar`: `{chat, theme, sections: [{tools: [..]}, {zoom: {id}}], tools: "docked"|"top"|"bottom", view}`
 * - `panels/tool_options`: the armed tool's options `{controls: [..]}`, or null
 * - `panels/dock`: the selection's controls `{controls: [..]}`, or null while a tool has options
 * - `panels/strip`: the tab row `{tabs: [{id, label, icon, active}], add, chat, zoom}`: `chat` is the right panel's toggle or null, `zoom` is `{id}` while no mod draws the tools, else null
 * - `panels/menu`: the open context menu `{at: [x, y], menu: {controls}}`, or null
 * - `panels/palette`: `[{label, group, command, chord, enabled, checked, keywords}]`
 * - `panels/share`: the Share button and its access dropdown
 * - `panels/welcome`: the first-run view while no space is open `{title, body: [line], choices: [{id, label, detail, primary}], note}`, or null. A choice is pressed with `control.press {id}`
 * - `theme`: `{appearance: "light"|"dark", choice: "system"|"light"|"dark"}`
 * - `settings`: `{showSidebar, showChat, tools, hideAgentCursors, hideSyncedCursors}`
 * - `keys`: `[{keys, context, command, args}]`, every key binding
 * - `layout`: `{file, toggles: {hidden: {id: bool}}, hidden: [region names]}`
 * - `notices`: `[{id, level: "info"|"warning"|"error", text, action: {label, run, args} | null, dismiss}]`: a page that stopped or answered an HTTP error, a share that failed, mods a canvas needs (`mods.install`). `notice.dismiss {id}` takes one away when `dismiss` is true
 * - `canvas/mod_items`: the active canvas's mod items `[{id, mod, width, height, props}]`, without their positions (1.1)
 * - `tool`: the armed mod tool's stroke `{tool: "<mod>/<tool>", number, phase: "down"|"up", points: [[x, y]]}` in canvas units, or null (1.1)
 * - `call`: the last mod command run `{command: "<mod>/<command>", number, args}`, or null. Act once per `number` (1.1)
 * - `panels/sidebar`: `{visible, add, sections: [{id, title, folded, rows}]}`: the canvases section's rows are `{id, canvas, label, active, count, rename}`, the others `{id, label, icon, trailing, selected, dimmed, expanded, children}`. A row is pressed with `control.press {id}`, renamed by pressing its `rename` id (1.2)
 * - `panels/chat`: `{available, visible, width, title, threads: [{id, title, draft, active, running}], transcript: {messages: [{id, role, text, images}], streaming, run, error} | null, composer: {placeholder, running, model: {label, options: [{name, description, chosen}]}}}`, driven by the `chat.*` commands (1.2)
 * - `layouts`: `[{name, title, active}]`, every layout; `layout.switch {name}` shows one (1.2)
 * - `look`: `{appearance: "light"|"dark", theme, colors: {name: css color}, font, fontSize, radius, background}`, the layout's resolved look (1.2)
 * - `overlays`: the open overlays, bottom to top `[{id, mod, kind, props, width, height, at, edge, frame}]` (1.2)
 * - `selection`: `{ids: [entity id], edges: [edge id]}` (1.2)
 * - `canvas/items`: every item of the active canvas, back to front `[{id, kind, label, parent, ..detail}]`, without places or sizes: a page adds `url`, text `text`, a shape `shape` and `text`, a mod item `mod` (1.2)
 * - `canvas/info`: `{id, name, items, edges, canvases: [{id, name, active}]}` (1.2)
 */
export type StateKey =
  | "panels/toolbar"
  | "panels/tool_options"
  | "panels/dock"
  | "panels/strip"
  | "panels/menu"
  | "panels/palette"
  | "panels/share"
  | "panels/welcome"
  | "theme"
  | "settings"
  | "keys"
  | "layout"
  | "notices"
  | "canvas/mod_items"
  | "tool"
  | "call"
  | "panels/sidebar"
  | "panels/chat"
  | "layouts"
  | "look"
  | "overlays"
  | "selection"
  | "canvas/items"
  | "canvas/info";
/**
 * Every command `run` takes, with its arguments and what it does.
 *
 * - `app.quit`: Quit Specular.
 * - `app.open-space`: Choose another space folder to open.
 * - `app.open`: Choose a canvas file to open.
 * - `app.save`: Write every canvas now.
 * - `app.settings`: Open the settings.
 * - `app.hide`: Hide Specular.
 * - `app.hide-others`: Hide every other app.
 * - `app.minimize`: Minimize the window.
 * - `app.command-palette`: Open the command palette.
 * - `window.close`: Close the window, as its red button does.
 * - `window.zoom`: Zoom the window, as its green button does.
 * - `canvas.new`: Add an empty canvas and show it.
 * - `canvas.duplicate` `{canvas?: id}`: Copy a canvas, the active one by default, and show the copy.
 * - `canvas.delete` `{canvas?: id}`: Remove a canvas, the active one by default, and trash its file.
 * - `canvas.switch` `{canvas: id}`: Show another canvas of the space.
 * - `canvas.rename` `{name: text, canvas?: id}`: Give a canvas, the active one by default, another name.
 * - `canvas.begin-rename` `{canvas?: id}`: Start typing a new name for a canvas in its sidebar row.
 * - `edit.cancel`: Back out one stage, as Escape does: a draft, a drag, a tool, a page, the selection.
 * - `edit.undo`: Undo the latest step, or the latest step of the text being edited.
 * - `edit.redo`: Redo the latest undone step.
 * - `edit.cut`: Put the selection on the clipboard and remove it.
 * - `edit.copy`: Put the selection on the clipboard.
 * - `edit.paste`: Paste the clipboard at the pointer.
 * - `edit.duplicate`: Copy the selection into free space beside it and select the copies.
 * - `edit.delete`: Remove the selection, or the focused comment.
 * - `edit.select-all`: Select everything that is not inside a group.
 * - `edit.select` `{ids?: [entity id], edges?: [edge id]}`: Replace the selection; ids that name nothing are dropped.
 * - `edit.nudge` `{dx: number, dy: number}`: Move the selection by exactly this many canvas units.
 * - `item.add` `{kind: sticky|text|shape|mod, x: number, y: number, text?: string, mod?: name, props?: object, width?: number, height?: number}`: Place a new item with its top-left at a canvas point, in the tool's style, and select it. A `mod` item is one of an installed mod's, with its props.
 * - `text.bold`: Toggle bold on the selected text.
 * - `text.italic`: Toggle italic on the selected text.
 * - `text.code`: Toggle inline code on the selected text.
 * - `text.strikethrough`: Toggle strikethrough on the selected text.
 * - `text.bullet-list`: Make the selected lines a bullet list, or plain lines again.
 * - `text.numbered-list`: Make the selected lines a numbered list, or plain lines again.
 * - `text.task-list`: Make the selected lines a task list, or plain lines again.
 * - `text.body`: Make the selected lines body text.
 * - `text.heading-1`: Make the selected lines a level 1 heading.
 * - `text.heading-2`: Make the selected lines a level 2 heading.
 * - `text.heading-3`: Make the selected lines a level 3 heading.
 * - `text.heading-4`: Make the selected lines a level 4 heading.
 * - `text.heading-5`: Make the selected lines a level 5 heading.
 * - `text.heading-6`: Make the selected lines a level 6 heading.
 * - `arrange.bring-forward`: Move the selection one slot forward in the stack order.
 * - `arrange.send-backward`: Move the selection one slot backward in the stack order.
 * - `arrange.bring-to-front`: Move the selection in front of everything.
 * - `arrange.send-to-back`: Move the selection behind everything.
 * - `arrange.group`: Wrap the selected items in a new group and select it.
 * - `arrange.ungroup`: Take the selected group apart and select what was inside it.
 * - `arrange.auto-layout`: Make the selection an auto-layout group that packs its members.
 * - `arrange.lay-out` `{mode: row|column|grid}`: Lay the selected items out in a row, a column or a grid, evening the gaps.
 * - `arrange.group-layout` `{axis: row|column|none}`: Pack the selected group's members along an axis, or let them sit where they are.
 * - `arrange.group-gap` `{gap: number}`: Set the gap the selected auto-layout group packs with, in canvas units.
 * - `comment.annotate-selection`: Open a comment draft on the region the selected entities span.
 * - `comment.resolve` `{id?: comment id}`: Mark a comment resolved, or every comment on the focused one's mark.
 * - `comment.delete` `{id?: comment id}`: Remove a comment, or every comment on the focused one's mark.
 * - `comment.focus` `{id?: comment id}`: Give a comment the focus, or with no id let go of it.
 * - `comment.reveal` `{id: comment id}`: Give a comment the focus and bring what it is on into view.
 * - `page.new-tab`: Make a page in a free spot, show its tab and put the caret in its address.
 * - `page.edit-address`: Put the caret in the address of the page the dock holds.
 * - `page.back`: Take the entered or selected page an entry back in its history.
 * - `page.forward`: Take the entered or selected page an entry forward in its history.
 * - `page.reload`: Load the entered or selected page's address again.
 * - `page.stop`: Abandon the entered or selected page's load in flight.
 * - `page.navigate` `{url: address, page?: entity id}`: Take a page, the one in the dock by default, to an address.
 * - `page.frame`: Move the selected pages' device frame to its next style.
 * - `page.frame-color`: Move the selected pages' device colour to the other one.
 * - `page.rotate`: Turn the selected pages between portrait and landscape.
 * - `page.scheme`: Show the selected pages in the color scheme they are not shown in.
 * - `page.orientation` `{orientation: portrait|landscape}`: Set the selected pages' orientation.
 * - `page.toggle-sync`: Put the selected pages in one sync set, or take them out of theirs.
 * - `tab.show` `{item?: entity id}`: Show one page or Document in its tab, or with no item the canvas.
 * - `tab.open` `{item: entity id}`: Open a page or Document in a tab from the canvas.
 * - `tab.close` `{item?: entity id}`: Close an item's tab, the one showing by default; the item stays on the canvas.
 * - `tab.next`: Show the tab after the one showing, going round to the first.
 * - `tab.previous`: Show the tab before the one showing, going round to the last.
 * - `tab.lens` `{lens: fill|device}`: Look at the item of the tab showing through this lens.
 * - `tab.others` `{shown: bool}`: Whether Device draws what is around its item.
 * - `tool.set` `{tool: select|page|text|sticky|document|shape|draw|comment|inspect|<mod>/<tool>}`: Arm a tool.
 * - `tool.ellipse`: Arm the shape tool with ellipses.
 * - `tool.diamond`: Arm the shape tool with diamonds.
 * - `tool.highlighter`: Arm the draw tool with the highlighter.
 * - `view.zoom-in`: Zoom in one step about the middle of the viewport.
 * - `view.zoom-out`: Zoom out one step about the middle of the viewport.
 * - `view.zoom-reset`: Zoom to 100% about the middle of the viewport.
 * - `view.zoom-to-fit`: Show everything on the canvas, centred.
 * - `view.zoom` `{percent: number}`: Zoom to this many percent about the middle of the viewport.
 * - `view.camera` `{x: number, y: number, zoom: number}`: Move the camera: the pan in screen pixels, and the zoom.
 * - `view.reveal` `{id: entity id}`: Select an entity and pan it into view.
 * - `view.theme` `{theme: light|dark|system}`: Choose the theme and save the choice.
 * - `view.toggle-sidebar`: Show the sidebar, or hide it.
 * - `ui.toggle` `{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}`: Hide a region of the window or a control if it is shown, show it if hidden.
 * - `ui.show` `{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}`: Show a region of the window or a control.
 * - `ui.hide` `{id: tabs|toolbar|dock|sidebar|chat|menu|share|toast|palette|welcome|<control id>}`: Hide a region of the window or a control, giving its room to the canvas.
 * - `ui.reset`: Set aside what was hidden and shown, and go back to the shipped layout.
 * - `notice.dismiss` `{id: notice id}`: Take away a page's notice in the `notices` key; it comes back if the page fails again.
 * - `item.set` `{id: mod item id, props: object}`: Set keys of a mod item's props, one undo step; a null takes a key away.
 * - `item.draw` `{id: mod item id, draws: [{type: rect|ellipse|path|text|image, ..}]}`: Give a mod item the display list it shows, in item space. Not an undo step.
 * - `tool.preview` `{draws: [{type: rect|ellipse|path|text|image, ..}]}`: Show what the armed mod tool would make, in canvas units; an empty list clears it.
 * - `mods.install`: Install the mods this canvas uses and the app does not have, from the space folder.
 * - `share.copy-link`: Copy the active canvas's link, publishing it first when it is not shared.
 * - `share.access` `{scope: edit|comment|view|none}`: Choose who the active canvas's link lets in; none stops sharing it.
 * - `share.join` `{link: url, fresh?: bool}`: Share the active canvas through a link's live room, or a new canvas with fresh.
 * - `share.stop`: Stop sharing the active canvas, keeping its document as it is.
 * - `control.press` `{id: control name}`: Do what a click on the control with this name does.
 * - `layout.switch` `{name?: layout name}`: Show another layout, or the next one (1.2).
 * - `overlay.open` `{mod, id?, props?, kind?: dialog|popup|drawer|window, width?, height?, at?: [x, y], edge?: left|right|top|bottom, frame?: none}`: Open a mod above the window (1.2).
 * - `overlay.close` `{id?: overlay id}`: Close an overlay, the top one by default (1.2).
 * - `chat.toggle`: Show the right panel if it is hidden, hide it if shown (1.2).
 * - `chat.new-thread`: Start a thread on the canvas showing (1.2).
 * - `chat.select` `{id: thread id}`: Show a thread of `panels/chat` (1.2).
 * - `chat.back`: Go back from a thread to the list (1.2).
 * - `chat.close` `{id?: thread id}`: Close the thread named, or the one showing (1.2).
 * - `chat.send` `{text: message}`: Send a message to the thread showing, or to a new one (1.2).
 * - `chat.stop`: Stop the agent's run on the thread showing (1.2).
 * - `chat.model` `{name: model name}`: Choose the model new runs use, by a name `panels/chat` lists (1.2).
 */
export type CommandName =
  | `${string}/${string}`
  | "app.quit"
  | "app.open-space"
  | "app.open"
  | "app.save"
  | "app.settings"
  | "app.hide"
  | "app.hide-others"
  | "app.minimize"
  | "app.command-palette"
  | "window.close"
  | "window.zoom"
  | "canvas.new"
  | "canvas.duplicate"
  | "canvas.delete"
  | "canvas.switch"
  | "canvas.rename"
  | "canvas.begin-rename"
  | "edit.cancel"
  | "edit.undo"
  | "edit.redo"
  | "edit.cut"
  | "edit.copy"
  | "edit.paste"
  | "edit.duplicate"
  | "edit.delete"
  | "edit.select-all"
  | "edit.select"
  | "edit.nudge"
  | "item.add"
  | "text.bold"
  | "text.italic"
  | "text.code"
  | "text.strikethrough"
  | "text.bullet-list"
  | "text.numbered-list"
  | "text.task-list"
  | "text.body"
  | "text.heading-1"
  | "text.heading-2"
  | "text.heading-3"
  | "text.heading-4"
  | "text.heading-5"
  | "text.heading-6"
  | "arrange.bring-forward"
  | "arrange.send-backward"
  | "arrange.bring-to-front"
  | "arrange.send-to-back"
  | "arrange.group"
  | "arrange.ungroup"
  | "arrange.auto-layout"
  | "arrange.lay-out"
  | "arrange.group-layout"
  | "arrange.group-gap"
  | "comment.annotate-selection"
  | "comment.resolve"
  | "comment.delete"
  | "comment.focus"
  | "comment.reveal"
  | "page.new-tab"
  | "page.edit-address"
  | "page.back"
  | "page.forward"
  | "page.reload"
  | "page.stop"
  | "page.navigate"
  | "page.frame"
  | "page.frame-color"
  | "page.rotate"
  | "page.scheme"
  | "page.orientation"
  | "page.toggle-sync"
  | "tab.show"
  | "tab.open"
  | "tab.close"
  | "tab.next"
  | "tab.previous"
  | "tab.lens"
  | "tab.others"
  | "tool.set"
  | "tool.ellipse"
  | "tool.diamond"
  | "tool.highlighter"
  | "view.zoom-in"
  | "view.zoom-out"
  | "view.zoom-reset"
  | "view.zoom-to-fit"
  | "view.zoom"
  | "view.camera"
  | "view.reveal"
  | "view.theme"
  | "view.toggle-sidebar"
  | "ui.toggle"
  | "ui.show"
  | "ui.hide"
  | "ui.reset"
  | "notice.dismiss"
  | "item.set"
  | "item.draw"
  | "tool.preview"
  | "mods.install"
  | "share.copy-link"
  | "share.access"
  | "share.join"
  | "share.stop"
  | "control.press"
  | "layout.switch"
  | "overlay.open"
  | "overlay.close"
  | "chat.toggle"
  | "chat.new-thread"
  | "chat.select"
  | "chat.back"
  | "chat.close"
  | "chat.send"
  | "chat.stop"
  | "chat.model";

/** The value of a state key, or null. Reading a key subscribes the mod to it: the mod renders again when it changes, and only then. */
export function state(key: StateKey): any;
/** What the layout hands this mod (`props` on its region's node), or null. `props().hidden` lists the ids of controls to leave out. */
export function props(): any;
/** A value shared between mods, or null. */
export function shared(key: string): any;
/** Sets a shared value. Every mod that read it renders again. Returns whether it changed. */
export function setShared(key: string, value: any): boolean;
/** Runs a command by name, as the palette, a key or the CLI would. Throws when the command is unknown, its arguments are wrong, or it cannot run now. The command itself runs just after the call returns. */
export function run(name: CommandName, args?: object): { ok: true; name: string };
/** How an overlay sits: a dialog is centred over a scrim, a popup sits at `at` and closes on a press outside, a drawer runs along `edge`, a window stays until closed. Escape closes the top dialog, popup or drawer. */
export type OverlayKind = "dialog" | "popup" | "drawer" | "window";
/** Opens a copy of the mod `name` over the window and answers its id (the `id` given, else the mod's name). The copy reads its id as `props().overlay`. Opening an id already open moves it to the top with the new spec. */
export function openOverlay(name: string, spec?: {
  id?: string; props?: object; kind?: OverlayKind; width?: number; height?: number;
  at?: [number, number]; edge?: "left" | "right" | "top" | "bottom"; frame?: "none";
}): string;
/** Closes the overlay `id`, or the top one. */
export function closeOverlay(id?: string): void;
/** Every command: `{ name, args, about, enabled }`. */
export function commands(): Array<{ name: CommandName; args: string; about: string; enabled: boolean }>;
/** Shows a line to the person (today: the app's log). */
export function notify(text: string, level?: "info" | "success" | "warning" | "error"): void;
/** The chrome's colours for the theme drawn now, as `#rrggbbaa` literals. Reads the `theme` key. */
export function colors(): {
  toolbar: string; toolbarBorder: string; toolbarText: string; toolbarTextStrong: string;
  popup: string; popupBorder: string; text: string; textMuted: string; hover: string; on: string;
  divider: string; keyText: string; sidebar: string; sidebarRule: string; interactive: string;
  interactiveHover: string; raisedFill: string; raisedRim: string; raisedShadowNear: string;
  raisedShadowFar: string; tabHover: string; primary: string; primaryHover: string;
  primaryForeground: string; error: string; warning: string; focusRing: string; hoverText: string; backdrop: string; accent: string;
  [name: string]: string;
};

/** The active layout's look: its appearance, theme name, resolved colours, font, control radius and background. */
export function look(): {
  appearance: "light" | "dark";
  theme: string | null;
  colors: Record<string, string>;
  font: string | null;
  fontSize: number | null;
  radius: number | null;
  background: any;
};

/** A Rust-built element: give it an id and its props, and style the box it sits in. */
interface HostComponent<Props> {
  new(id: string, props: Props): any;
}
/** `Icon`: a panel glyph by its name in the state (`tools[].icon`, `"select-tool"`, `{ shape: "ellipse" }`), `size` px square (20), drawn in `color` (`#rrggbb[aa]`), showing the stored colour `tint` (`tools[].tint`). `control` names the control the glyph sits on, so the window's scripted runs (`control NAME`) find it where the mod drew it. */
export const Icon: HostComponent<{ name: any; size?: number; color?: string; tint?: string | null; active?: boolean; control?: string }>;
/** `Zoom`: the zoom readout and its levels' list, which follow the camera with no script render. `compact` drops the caret, for a narrow column. */
export const Zoom: HostComponent<{ compact?: boolean }>;
/** `ToolOptions`: the options of the tool in hand, beside its button: place it inside the button's `relative()` box. `place` is `panels/toolbar`'s `tools`. */
export const ToolOptions: HostComponent<{ place: "docked" | "top" | "bottom" }>;
/** `Field`: a line of text the window holds, so the keys stay in it while a person types. With `control`, the model field of that id in `panels/dock` or `panels/tool_options` (an address, a size), which sends what is typed as the app's own does. Otherwise a mod's own field: what is typed is `shared("field/<id>")`, and each Return moves `shared("field/<id>/submit")` on by one. `value` is where it starts, and `autofocus` gives it the keys as it is made. A field is made once for its `id`: name it anew for a fresh one. `radius`, `background`, `border` and `focusRing` (colours as `colors()` names them, or `#rrggbb[aa]`) restyle its box, so a layout's fields match its look; a field styled this way shows its focus by turning its own edge `focusRing` (the theme's `focusRing` by default) instead of drawing the app's rounded ring. Headless it is a placeholder. */
export const Field: HostComponent<{ control?: string; id?: string; value?: string; placeholder?: string; autofocus?: boolean; radius?: number; background?: string; border?: string; focusRing?: string }>;
/** `Swatch`: a dot of a canvas colour as the app's dock draws it. `color` is a colour as the panel keys and the \`.canvas\` file name it (`"neutral"`, `"3"`, `"#e0e0e0"`), never a CSS colour a mod's element takes: pass it here instead of to `bg()`. `palette` and `role` are the swatches control's own; `null` draws the hollow choice. */
export const Swatch: HostComponent<{ color: string | null; palette?: "soft" | "vivid"; role?: "fill" | "ink"; size?: number }>;
/** A paint as a layout block takes it: a colour, a gradient, or a repeating pattern over a base colour (1.2). */
export type Paint =
  | string
  | { gradient: [string, string]; angle?: number }
  | { from: string; to: string; angle?: number }
  | { color?: string; pattern: "stripes" | "hatch" | "checker"; ink: string; width?: number; gap?: number; size?: number; direction?: "horizontal" | "vertical" };
/** `Pattern`: a box filling its parent (place it in a `relative()` box), painted with `paint`: pinstripes, hatching, a checked desktop. Hatching and checks are one GPU fill; stripes are a box each, at most 400, reaching `width` and `height` (the window's size when left out). Headless it is a placeholder. */
export const Pattern: HostComponent<{ paint: Paint; width?: number; height?: number }>;
/** `Favicon`: a tab's glyph: the page's own icon once the window has it, else the panel glyph `icon` (`tabs[].icon`) as `Icon` draws it. `tab` is the tab's id. */
export const Favicon: HostComponent<{ tab: string; icon: any; size?: number; color?: string; control?: string }>;
/** `WindowDrag`: bare title bar. A press that moves drags the window, and a double click does what the system says a title bar's does. It fills the box it is put in. */
export const WindowDrag: HostComponent<{}>;
/** `Share`: what the layout puts in the Share slot: the `share` mod when the layout names one, else the app's own Share button and popover. */
export const Share: HostComponent<{}>;
/** `Prism`: the turning Prism at the head of the first-run view, 160 points square, drawn on the GPU by the window. */
export const Prism: HostComponent<{}>;

/**
 * One mark of a mod item's display list (`item.draw`) or a tool's preview
 * (`tool.preview`), in item space or canvas units. Colours are `#rrggbb` or
 * `#rrggbbaa`. An image's `src` is a file in the space folder or an
 * `http(s)` URL. The last list an item drew is saved in the .canvas file as
 * its snapshot, which shows where the mod is not installed.
 */
export type ModDraw =
  | { type: "rect"; x: number; y: number; width: number; height: number; fill?: string; stroke?: string; line?: number; radius?: number }
  | { type: "ellipse"; x: number; y: number; width: number; height: number; fill?: string; stroke?: string; line?: number }
  | { type: "path"; points: Array<[number, number]>; closed?: boolean; fill?: string; stroke?: string; line?: number }
  | { type: "text"; x: number; y: number; text: string; size?: number; color?: string; width?: number; align?: "left" | "center" | "right" }
  | { type: "image"; x: number; y: number; width: number; height: number; src: string; fit?: "contain" | "cover" | "fill"; radius?: number };
