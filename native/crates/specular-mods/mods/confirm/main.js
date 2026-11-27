// A yes-or-no dialog, opened as an overlay by any mod or agent:
//   openOverlay("confirm", { props: {
//     title: "Delete this canvas?", message: "It cannot be undone.",
//     confirm: "Delete", command: "canvas.delete", args: {} } })
// The confirm button runs `command` with `args` and closes the dialog;
// Cancel and Escape close it.
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { props, run, colors, closeOverlay } from "specular";

function button(id, text, primary, k, onPress) {
  return Button.new(id)
    .h(28)
    .px(12)
    .flex()
    .items_center()
    .rounded(6)
    .text_size(13)
    .bg(primary ? k.primary : k.raisedFill)
    .text_color(primary ? k.primaryForeground : k.text)
    .border_1()
    .border_color(primary ? k.primary : k.popupBorder)
    .hover((el) => el.bg(primary ? k.primaryHover : k.hover))
    .on_click(onPress)
    .child(text);
}

export default class Confirm extends View {
  render() {
    const p = props() || {};
    const k = colors();
    const close = () => closeOverlay(p.overlay);
    return v_flex()
      .id("confirm")
      .w(360)
      .p(16)
      .gap(8)
      .text_color(k.text)
      .child(div().text_size(15).font_weight(600).child(p.title || "Are you sure?"))
      .when(p.message != null, (el) => el.child(div().text_size(13).text_color(k.textMuted).child(p.message)))
      .child(
        h_flex()
          .pt(8)
          .gap(8)
          .justify_end()
          .child(button("confirm.cancel", p.cancel || "Cancel", false, k, close))
          .child(
            button("confirm.ok", p.confirm || "OK", true, k, () => {
              close();
              if (p.command) run(p.command, p.args || {});
            }),
          ),
      );
  }
}
