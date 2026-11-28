// The first-run view, drawn from `panels/welcome` in place of the canvas
// while no space is open: the turning Prism, the heading and its line, the
// first two choices as buttons side by side and any others as quieter
// links below, and a warning under them when one of them needs it. A
// choice is pressed by its id.
//
// A layout hides a choice by listing its id in this mod's props:
//   { "mod": "welcome", "props": { "hidden": ["onboarding.electron"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, Prism } from "specular";

const WIDTH = 480;
const BUTTONS = 2;

const press = (id) => () => run("control.press", { id });

function button(choice, k) {
  const face = Button.new(choice.id)
    .h(40)
    .px(16)
    .flex()
    .items_center()
    .rounded(8)
    .on_click(press(choice.id))
    .child(div().text_xs().child(choice.label));
  return choice.primary
    ? face.bg(k.primary).text_color(k.primaryForeground).hover((el) => el.bg(k.primaryHover))
    : face.border_1().border_color(k.popupBorder).text_color(k.text).hover((el) => el.bg(k.hover));
}

function link(choice, k) {
  return Button.new(choice.id)
    .h(24)
    .px(8)
    .flex()
    .items_center()
    .rounded(6)
    .text_color(k.textMuted)
    .hover((el) => el.bg(k.hover))
    .on_click(press(choice.id))
    .child(div().text_xs().child(choice.label));
}

export default class Welcome extends View {
  render() {
    const model = state("panels/welcome");
    if (model == null) return div();
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    const choices = model.choices.filter((choice) => !hidden.has(choice.id));
    const buttons = choices.slice(0, BUTTONS);
    const links = choices.slice(BUTTONS);
    const body = model.body.map((line) =>
      div().text_size(15).line_height(22).text_center().text_color(k.textMuted).child(line),
    );
    return div()
      .id("welcome")
      .size_full()
      .flex()
      .items_center()
      .justify_center()
      .bg(k.backdrop)
      .text_color(k.text)
      .child(
        v_flex()
          .w(WIDTH)
          .items_center()
          .child(Prism.new("prism", {}))
          .child(div().mt(24).text_size(28).font_weight(600).text_center().child(model.title))
          .child(v_flex().mt(8).gap(8).items_center().children(body))
          .child(h_flex().mt(32).gap(12).justify_center().children(buttons.map((c) => button(c, k))))
          .child(h_flex().mt(12).gap(8).justify_center().children(links.map((c) => link(c, k))))
          .when(model.note != null, (el) =>
            el.child(
              div().mt(12).text_size(12).line_height(17).text_center().text_color(k.textMuted).child(model.note),
            ),
          ),
      );
  }
}
