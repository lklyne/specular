// A dialog that asks for a line of text, opened as an overlay:
//   openOverlay("prompt", { props: {
//     title: "Rename canvas", value: "Canvas 1", confirm: "Rename",
//     command: "canvas.rename", args: { canvas: "tab_1" }, key: "name" } })
// Return or the confirm button runs `command` with `args` and the text
// under `key` (`text` when not given), then closes the dialog.
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { props, run, shared, colors, closeOverlay, Field } from "specular";

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

export default class Prompt extends View {
  init() {
    this.field = `prompt-${Date.now()}`;
    this.submitted = 0;
  }

  submit(p) {
    const text = shared(`field/${this.field}`) || "";
    closeOverlay(p.overlay);
    if (p.command) run(p.command, { ...(p.args || {}), [p.key || "text"]: text });
  }

  render() {
    const p = props() || {};
    const k = colors();
    const submit = shared(`field/${this.field}/submit`) || 0;
    if (submit !== this.submitted) {
      this.submitted = submit;
      this.submit(p);
    }
    return v_flex()
      .id("prompt")
      .w(380)
      .p(16)
      .gap(10)
      .text_color(k.text)
      .child(div().text_size(15).font_weight(600).child(p.title || "Enter a value"))
      .child(Field.new("prompt.field", { id: this.field, value: p.value || "", placeholder: p.placeholder || "", autofocus: true }))
      .child(
        h_flex()
          .gap(8)
          .justify_end()
          .child(button("prompt.cancel", p.cancel || "Cancel", false, k, () => closeOverlay(p.overlay)))
          .child(button("prompt.ok", p.confirm || "OK", true, k, () => this.submit(p))),
      );
  }
}
