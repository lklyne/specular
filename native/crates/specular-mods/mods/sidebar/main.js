// The sidebar, drawn from `panels/sidebar`: the space's canvases, then the
// active canvas's notes and pages, each section folding at its head. A
// click presses a row by its id, as the app's own sidebar does; a double
// click on a canvas asks for its new name in the `prompt` dialog.
//
// A layout hides a section or a row by listing its id in this mod's props:
//   { "mod": "sidebar", "props": { "hidden": ["sidebar.head.notes"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, openOverlay, Icon } from "specular";

const ROW = 26;
const INDENT = 14;

const press = (id) => run("control.press", { id });

function rename(row) {
  openOverlay("prompt", {
    id: "sidebar.rename",
    props: {
      title: "Rename canvas",
      value: row.label,
      confirm: "Rename",
      command: "canvas.rename",
      args: { canvas: row.canvas },
      key: "name",
    },
  });
}

function canvasRow(row, k) {
  return Button.new(row.id)
    .h(ROW)
    .px(8)
    .flex()
    .items_center()
    .justify_between()
    .rounded(6)
    .when(row.active, (el) => el.bg(k.interactive))
    .hover((el) => el.bg(k.interactiveHover))
    .on_click((event) => (event.click_count >= 2 ? rename(row) : press(row.id)))
    .child(div().truncate().child(row.label))
    .child(div().text_color(k.textMuted).text_size(11).child(String(row.count)));
}

function itemRow(row, depth, hidden, k) {
  const out = [
    Button.new(row.id)
      .h(ROW)
      .pl(8 + depth * INDENT)
      .pr(8)
      .flex()
      .items_center()
      .gap(6)
      .rounded(6)
      .when(row.selected, (el) => el.bg(k.interactive))
      .when(row.dimmed, (el) => el.opacity(0.5))
      .hover((el) => el.bg(k.interactiveHover))
      .on_click(() => press(row.id))
      .child(Icon.new(`${row.id}.glyph`, { name: row.icon, size: 14, color: k.textMuted }))
      .child(div().flex_1().truncate().child(row.label))
      .when(row.trailing != null, (el) => el.child(div().text_color(k.textMuted).text_size(11).child(row.trailing))),
  ];
  if (row.expanded) {
    for (const child of row.children) {
      if (!hidden.has(child.id)) out.push(...itemRow(child, depth + 1, hidden, k));
    }
  }
  return out;
}

function section(section, hidden, k) {
  const head = Button.new(section.id)
    .h(ROW)
    .px(8)
    .flex()
    .items_center()
    .justify_between()
    .rounded(6)
    .text_size(11)
    .text_color(k.textMuted)
    .hover((el) => el.bg(k.interactiveHover))
    .on_click(() => press(section.id))
    .child(section.title.toUpperCase())
    .child(section.folded ? "+" : "−");
  const rows = section.folded
    ? []
    : section.rows
        .filter((row) => !hidden.has(row.id))
        .flatMap((row) => (row.canvas != null ? [canvasRow(row, k)] : itemRow(row, 0, hidden, k)));
  return v_flex().gap(1).child(head).children(rows);
}

export default class Sidebar extends View {
  render() {
    const model = state("panels/sidebar");
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    if (model == null) return div();
    const sections = model.sections.filter((each) => !hidden.has(each.id));
    return v_flex()
      .id("sidebar")
      .size_full()
      .p(8)
      .gap(12)
      .bg(k.sidebar)
      .border_r_1()
      .border_color(k.sidebarRule)
      .text_size(13)
      .text_color(k.text)
      .overflow_y_scroll()
      .children(sections.map((each) => section(each, hidden, k)))
      .when(!hidden.has(model.add), (el) =>
        el.child(
          Button.new(model.add)
            .h(ROW)
            .px(8)
            .flex()
            .items_center()
            .rounded(6)
            .text_color(k.textMuted)
            .hover((el) => el.bg(k.interactiveHover))
            .on_click(() => press(model.add))
            .child("+ New canvas"),
        ),
      );
  }
}
