// The canvas's context menu, drawn from `panels/menu`: its items in
// sections with a line between them, each with its key. The window places
// it at the point the menu opened and closes it on a press outside or on
// Escape; a pick presses the item by its name, so a paste lands where the
// menu opened.
//
// A layout hides an item by listing its id in this mod's props:
//   { "mod": "menu", "props": { "hidden": ["menu.delete"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors } from "specular";

const ROW = 24;

function item(option, k) {
  const row = Button.new(option.id)
    .h(ROW)
    .px(8)
    .gap(24)
    .flex()
    .items_center()
    .justify_between()
    .rounded(4)
    .text_size(12)
    .disabled(!option.enabled)
    .child(div().whitespace_nowrap().child(option.label))
    .when(option.chord != null, (el) =>
      el.child(div().text_color(k.textMuted).whitespace_nowrap().child(option.chord)),
    );
  if (!option.enabled) return row.opacity(0.4);
  return row
    .hover((el) => el.bg(k.hover))
    .on_click(() => run("control.press", { id: option.id }));
}

// The option lists of a menu's controls, one a section.
function sections(menu) {
  const lists = [];
  for (const control of menu.controls || []) {
    for (const section of control.content || []) {
      if (section.options) lists.push(section.options.options);
    }
  }
  return lists;
}

export default class Menu extends View {
  render() {
    const open = state("panels/menu");
    if (open == null) return div();
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    const rows = [];
    sections(open.menu)
      .map((options) => options.filter((option) => !hidden.has(option.id)))
      .filter((options) => options.length > 0)
      .forEach((options, index) => {
        if (index > 0) rows.push(div().my(4).mx(4).h(1).bg(k.divider));
        for (const option of options) rows.push(item(option, k));
      });
    return v_flex()
      .id("menu")
      .min_w(200)
      .p(4)
      .rounded(8)
      .bg(k.popup)
      .border_1()
      .border_color(k.popupBorder)
      .shadow_md()
      .text_color(k.text)
      .children(rows);
  }
}
