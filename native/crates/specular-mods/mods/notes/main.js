// The sidebar's Notes section: everything on the active canvas that is not a
// page, front of the stack first. It is drawn under the projects
// and their canvases, in the app's own sidebar or wherever a sidebar mod
// places `SidebarSections`. It starts off: turn it on in Settings > Mods.
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, run, colors, Icon } from "specular";

const KIND = "notes";
const ROW = 26;
const INDENT = 14;

const press = (id) => run("control.press", { id });

function head(section, k) {
  return Button.new(section.id)
    .h(ROW)
    .px(14)
    .flex()
    .items_center()
    .gap(6)
    .font_weight(500)
    .on_click(() => press(section.id))
    .child(div().truncate().child(section.title))
    .child(Icon.new(`${section.id}.chevron`, {
      name: section.folded ? "chevron-right" : "chevron-down",
      size: 12,
      color: k.textMuted,
    }));
}

function rows(row, depth, k) {
  const out = [
    Button.new(row.id)
      .h(ROW)
      .pl(22 + depth * INDENT)
      .pr(16)
      .flex()
      .items_center()
      .gap(8)
      .when(row.selected, (el) => el.bg(k.interactive))
      .when(row.dimmed, (el) => el.opacity(0.5))
      .hover((el) => el.bg(k.interactiveHover))
      .on_click(() => press(row.id))
      .child(Icon.new(`${row.id}.glyph`, { name: row.icon, size: 14, color: k.textMuted }))
      .child(div().flex_1().truncate().child(row.label))
      .when(row.trailing != null, (el) => el.child(div().text_color(k.textMuted).text_size(11).child(row.trailing))),
  ];
  if (row.expanded) {
    for (const child of row.children) out.push(...rows(child, depth + 1, k));
  }
  return out;
}

export default class Notes extends View {
  render() {
    const model = state("panels/sidebar");
    const section = model && model.sections.find((each) => each.kind === KIND);
    if (section == null) return div();
    const k = colors();
    return v_flex()
      .py(4)
      .text_color(k.text)
      .child(head(section, k))
      .children(section.folded ? [] : section.rows.flatMap((row) => rows(row, 0, k)));
  }
}
