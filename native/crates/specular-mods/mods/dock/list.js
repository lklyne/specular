// The dock's lists: what a dropdown opens, as sections of rows, rows or
// grids of glyphs, and segmented lines, and the faces and fills they share
// with the bar (`main.js`). `dock` is the `Dock` view, which holds which
// list is open and which row is being typed in.
import { div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { run, Icon, Field, Swatch } from "specular";

// The side of a control, and of a row of a list.
export const CONTROL = 24;
// A cell of a row or grid of glyphs.
const CELL = 32;

export const press = (id) => run("control.press", { id });

// A glyph's size: back and forward are thin strokes and read small, and
// reload, with the stop it turns into, keeps in step with them.
function glyphSize(icon) {
  if (icon === "chevron-left" || icon === "chevron-right") return 18;
  if (icon === "reload" || icon === "stop") return 16;
  return 14;
}

// The fill, rim and shadow of a toggle that is on.
export function raised(el, k) {
  return el
    .bg(k.raisedFill)
    .border_t_1()
    .border_color(k.raisedRimLit)
    .shadow([
      { spread: 0.5, color: k.raisedRim },
      { y: 1, blur: 2, color: k.raisedShadowNear },
      { y: 1, blur: 3, spread: -1, color: k.raisedShadowFar },
    ]);
}

// What a control or a choice shows: its glyph, its word, or a dot of its
// colour. `on` draws it in the full text colour and at rest in the muted
// one; `filled` is a row with the `on` fill, whose word takes `onText`;
// `group` is the row whose hover turns the word `hoverText`.
export function face(id, face, k, { on = false, filled = false, group = null } = {}) {
  if (face == null) return div();
  let row = h_flex().gap(4).items_center();
  if (face.icon) {
    row = row.child(
      Icon.new(`${id}.glyph`, {
        name: face.icon,
        size: glyphSize(face.icon),
        color: on ? k.text : k.textMuted,
        tint: face.color ?? null,
        active: on,
      }),
    );
  }
  if (face.text) {
    row = row.child(
      div()
        .whitespace_nowrap()
        .text_color(filled ? k.onText : on ? k.text : k.textMuted)
        .when(face.font === "mono", (el) => el.font_family("Geist Mono"))
        .when(group != null, (el) => el.group_hover(group, (el) => el.text_color(k.hoverText)))
        .child(face.text),
    );
  }
  if (!face.icon && !face.text) {
    row = row.child(Swatch.new(`${id}.dot`, { color: face.color ?? null, palette: "vivid", role: "ink", size: 16 }));
  }
  return row;
}

const rule = (k) => div().h(1).w_full().bg(k.divider);

const segmented = (section) => section?.options?.layout?.kind === "segmented";
// A row that opens more of a list ends the rows over it.
const opensMore = (section) =>
  section?.controls != null && section.controls.length > 0 && section.controls.every((c) => c.kind === "dropdown");

// How a list's rows are drawn: with far-end text or a key, the current row
// filled and the others muted ("presets"); else every row in the text
// colour and the current one checked ("menu").
function rowsOf(content) {
  const detailed = content.some((section) =>
    (section.options?.options || []).some((option) => option.trailing != null || option.chord != null));
  return detailed ? "presets" : "menu";
}

// One choice: a row of a list (`rows` is "menu" or "presets"), or a cell
// of a row or grid of glyphs (`rows` is null).
function choice(dock, option, rows, k) {
  const menu = rows === "menu";
  const filled = option.selected && !menu;
  const closes = (option.fields || []).length === 0;
  const height = rows == null ? CELL : option.face?.font ? 28 : CONTROL;
  const group = option.id;
  let row = Button.new(option.id)
    .group(group)
    .flex()
    .items_center()
    .justify_between()
    .gap(16)
    .h(height)
    .rounded(6)
    .text_size(12)
    .when(rows != null, (el) => el.w_full().px(8))
    .when(rows == null, (el) => el.w(CELL).justify_center())
    .when(!closes, (el) => el.pr(3))
    .when(filled, (el) => el.bg(k.on))
    .when(option.enabled === false, (el) => el.disabled(true).opacity(0.4))
    .when(option.enabled !== false, (el) =>
      el.hover((el) => el.bg(k.hover)).on_click((_, cx) => {
        if (closes) dock.close();
        press(option.id);
        cx.notify();
      }))
    .child(face(option.id, option.face, k, { on: option.selected || menu, filled, group }));
  if (option.selected && menu) {
    row = row.child(Icon.new(`${option.id}.check`, { name: "check", size: 12, color: k.text }));
  }
  if (option.trailing != null) {
    row = row.child(
      div()
        .whitespace_nowrap()
        .text_color(filled ? k.onText : k.textMuted)
        .group_hover(group, (el) => el.text_color(k.hoverText))
        .child(option.trailing),
    );
  }
  if (!closes) row = row.child(fields(dock, option, k));
  if (option.chord != null) {
    row = row.child(div().px(6).py(2).rounded(4).bg(k.on).text_color(k.keyText).child(option.chord));
  }
  return row;
}

// A choice's own fields at the far end of its row. They keep their room
// and show while the row is the current value, is under the pointer, or
// one of them is being typed in.
function fields(dock, option, k) {
  const shown = option.selected || dock.typing === option.id;
  return h_flex()
    .gap(8)
    .items_center()
    // A press in a field is typing, not a choice of the row.
    .on_mouse_down("left", (_, cx) => {
      cx.stop_propagation();
      if (dock.typing !== option.id) {
        dock.typing = option.id;
        cx.notify();
      }
    })
    .when(!shown, (el) => el.invisible().group_hover(option.id, (el) => el.visible()))
    .children(option.fields.map((field) => Field.new(field.id, { control: field.id })));
}

// A line that sets one thing: its title, and the values as the segments
// of one control at the far end.
function segmentedRow(title, options, k) {
  const segments = options.map((option) =>
    Button.new(option.id)
      .h(16)
      .px(8)
      .flex()
      .items_center()
      .rounded(4)
      .text_size(12)
      .text_color(option.selected ? k.text : k.textMuted)
      .when(option.selected, (el) => raised(el, k))
      .when(option.enabled === false, (el) => el.disabled(true).opacity(0.4))
      .when(option.enabled !== false && !option.selected, (el) => el.on_click(() => press(option.id)))
      .child(option.face?.text ?? option.label));
  return h_flex()
    .w_full()
    .h(CONTROL)
    .pl(8)
    .gap(16)
    .items_center()
    .justify_between()
    .child(div().text_color(k.textMuted).child(title))
    .child(h_flex().h(20).px(2).gap(2).items_center().rounded(6).bg(k.on).children(segments));
}

function listSection(dock, section, rows, k) {
  if (section.controls) {
    return h_flex().gap(4).items_center().children(section.controls.map((each) => dock.control(each, k, { inList: true })));
  }
  const { layout, options } = section.options;
  switch (layout.kind) {
    case "segmented":
      return segmentedRow(layout.title, options, k);
    case "row":
      return h_flex().gap(2).children(options.map((option) => choice(dock, option, null, k)));
    case "grid":
      return h_flex()
        .flex_wrap()
        .gap(2)
        .w(layout.columns * CELL + (layout.columns - 1) * 2)
        .children(options.map((option) => choice(dock, option, null, k)));
    default:
      return v_flex().gap(2).min_w(140).children(options.map((option) => choice(dock, option, rows, k)));
  }
}

// Sections one under another, with a line between blocks. Lines that
// each set one thing read as one block.
export function sections(dock, content, k) {
  const rows = rowsOf(content);
  return v_flex().gap(4).children(content.map((one, index) => {
    const next = content[index + 1];
    const joined = (segmented(one) && segmented(next)) || opensMore(next);
    return v_flex()
      .gap(4)
      .child(listSection(dock, one, rows, k))
      .when(next != null && !joined, (el) => el.child(rule(k)));
  }));
}
