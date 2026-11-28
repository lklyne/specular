// A status line for a layout's bottom block, drawn from `canvas/info`,
// `selection`, `layouts` and `tool`: the canvas and its counts, what is
// selected, and the layout in force, which a click switches to the next.
//   { "id": "status", "mod": "status", "size": { "px": 24 } }
import { View, div } from "gpui-kit";
import { h_flex, Button } from "gpui-base";
import { state, run, colors } from "specular";

function plural(n, word) {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

export default class Status extends View {
  render() {
    const k = colors();
    const info = state("canvas/info") || {};
    const selection = state("selection") || { ids: [], edges: [] };
    const layout = (state("layouts") || []).find((each) => each.active);
    const picked = selection.ids.length + selection.edges.length;
    return h_flex()
      .id("status")
      .size_full()
      .px(10)
      .gap(16)
      .items_center()
      .text_size(11)
      .bg(k.toolbar)
      .border_t_1()
      .border_color(k.toolbarBorder)
      .text_color(k.textMuted)
      .child(div().text_color(k.text).child(info.name || ""))
      .child(plural(info.items || 0, "item"))
      .child(picked > 0 ? `${picked} selected` : "")
      .child(div().flex_1())
      .child(
        Button.new("status.layout")
          .px(6)
          .rounded(4)
          .hover((el) => el.bg(k.hover))
          .on_click(() => run("layout.switch"))
          .child(layout ? `Layout: ${layout.title}` : ""),
      );
  }
}
