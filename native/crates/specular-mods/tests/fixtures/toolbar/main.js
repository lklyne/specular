// The phase 0 spike: a toolbar drawn from `panels/toolbar`, the shape the
// shipped toolbar mod reads. Only the tool buttons and the theme label.
import { View, div } from "gpui-kit";
import { h_flex, Button } from "gpui-base";
import { state, props, run } from "specular";

export default class Toolbar extends View {
  render(cx) {
    const t = state("panels/toolbar");
    const c = cx.theme().colors;
    const hidden = new Set((props() || {}).hidden || []);
    const groups = t.sections.map((section) =>
      h_flex().gap(2).children(
        section.tools.filter((b) => !hidden.has(b.id)).map((b) =>
          Button.new(b.id).size(32).rounded(8).items_center().justify_center()
            .bg(b.active ? c.primary : c.background)
            .text_color(b.active ? c.primary_foreground : c.foreground)
            .on_click(() => run("control.press", { id: b.id }))
            .child(b.label.slice(0, 1))
        )
      )
    );
    return h_flex().h(44).px(6).gap(8).items_center().rounded(12).bg(c.background)
      .children(groups)
      .child(div().text_sm().text_color(c.muted_foreground).child(t.theme.label));
  }
}
