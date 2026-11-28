// The notices, drawn from `notices` in the canvas's lower right corner: a
// page that stopped or answered an HTTP error, a share that failed. Each
// card offers what its notice offers (Reload, Try again) and, when the
// notice can be dismissed, a close button that runs `notice.dismiss`.
//
// A layout hides a notice by listing its id in this mod's props:
//   { "mod": "toast", "props": { "hidden": ["share.failed"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, Icon } from "specular";

const WIDTH = 320;

function card(notice, k) {
  const tone = notice.level === "error" ? k.error : notice.level === "warning" ? k.warning : k.textMuted;
  const action = notice.action
    ? Button.new(`${notice.id}.action`)
        .h(24)
        .px(10)
        .flex()
        .items_center()
        .rounded(6)
        .border_1()
        .border_color(k.popupBorder)
        .hover((el) => el.bg(k.hover))
        .on_click(() => run(notice.action.run, notice.action.args))
        .child(div().text_xs().child(notice.action.label))
    : null;
  const dismiss = notice.dismiss
    ? Button.new(`${notice.id}.dismiss`)
        .size(20)
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(4)
        .hover((el) => el.bg(k.hover))
        .on_click(() => run("notice.dismiss", { id: notice.id }))
        .child(Icon.new(`${notice.id}.dismiss.glyph`, { name: "close", size: 12, color: k.textMuted }))
    : null;
  return h_flex()
    .id(notice.id)
    .w(WIDTH)
    .p(10)
    .gap(10)
    .items_start()
    .rounded(10)
    .bg(k.popup)
    .border_1()
    .border_color(k.popupBorder)
    .shadow_md()
    .text_color(k.text)
    .child(div().mt(4).size(8).flex_shrink_0().rounded(4).bg(tone))
    .child(
      v_flex()
        .flex_1()
        .min_w(0)
        .gap(8)
        .child(div().text_sm().child(notice.text))
        .when(action != null, (el) => el.child(h_flex().child(action))),
    )
    .when(dismiss != null, (el) => el.child(dismiss));
}

export default class Toast extends View {
  render() {
    const hidden = new Set((props() || {}).hidden || []);
    const notices = (state("notices") || []).filter((notice) => !hidden.has(notice.id));
    const k = colors();
    return v_flex()
      .id("notices")
      .gap(8)
      .items_end()
      .children(notices.map((notice) => card(notice, k)));
  }
}
