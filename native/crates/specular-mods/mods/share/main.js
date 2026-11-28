// The Share button at the end of the tab row, and its popover, drawn from
// `panels/share`: who the link lets in (the choices open in place, under
// their row) and Copy link, with why the last share failed under them.
// A choice runs `share.access`, Copy link runs `share.copy-link`.
//
// A layout hides a piece by listing its id in this mod's props:
//   { "mod": "share", "props": { "hidden": ["share.access"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button, Popover } from "gpui-base";
import { state, props, run, colors, Icon } from "specular";

const RADIUS = 8;
const WIDTH = 300;
const ROW = 34;

// The scope `share.access` takes for an access choice's id.
function scope(id) {
  const name = id.slice(id.lastIndexOf(".") + 1);
  return name === "private" ? "none" : name;
}

function options(dropdown) {
  return dropdown.content.flatMap((section) => (section.options ? section.options.options : []));
}

export default class Share extends View {
  init() {
    this.open = false;
    this.choosing = false;
  }

  access(model, k) {
    const trigger = Button.new(model.id)
      .w_full()
      .h(ROW)
      .px(10)
      .flex()
      .items_center()
      .justify_between()
      .rounded(RADIUS)
      .border_1()
      .border_color(k.popupBorder)
      .hover((el) => el.bg(k.hover))
      .on_click((_event, cx) => {
        this.choosing = !this.choosing;
        cx.notify();
      })
      .child(div().text_sm().child(model.summary.text || ""))
      .child(Icon.new(`${model.id}.chevron`, { control: model.id, name: "chevron-down", size: 12, color: k.textMuted }));
    const choices = options(model).map((option) =>
      Button.new(option.id)
        .w_full()
        .h(ROW - 4)
        .px(10)
        .flex()
        .items_center()
        .justify_between()
        .rounded(RADIUS - 2)
        .hover((el) => el.bg(k.hover))
        .on_click((_event, cx) => {
          this.choosing = false;
          cx.notify();
          run("share.access", { scope: scope(option.id) });
        })
        .child(div().text_sm().child(option.label))
        .when(option.selected, (el) =>
          el.child(Icon.new(`${option.id}.check`, { name: "check", size: 12, color: k.textMuted })),
        ),
    );
    return v_flex()
      .gap(2)
      .child(trigger)
      .when(this.choosing, (el) => el.children(choices));
  }

  content(model, hidden, k) {
    const copy = Button.new(model.copy.id)
      .w_full()
      .h(ROW)
      .flex()
      .items_center()
      .justify_center()
      .rounded(RADIUS)
      .bg(k.primary)
      .text_color(k.primaryForeground)
      .disabled(!model.copy.enabled)
      .when(!model.copy.enabled, (el) => el.opacity(0.5))
      .when(model.copy.enabled, (el) =>
        el.hover((el) => el.bg(k.primaryHover)).on_click(() => run("share.copy-link")),
      )
      .child(div().text_sm().child(model.copy.label));
    return v_flex()
      .id("share-popover")
      .w(WIDTH)
      .p(12)
      .gap(8)
      .rounded(RADIUS + 2)
      .bg(k.popup)
      .border_1()
      .border_color(k.popupBorder)
      .shadow_md()
      .text_color(k.text)
      .when(!hidden.has(model.access.id), (el) => el.child(this.access(model.access, k)))
      .when(!hidden.has(model.copy.id), (el) => el.child(copy))
      .when(model.note != null, (el) => el.child(div().text_xs().text_color(k.textMuted).child(model.note)));
  }

  render() {
    const model = state("panels/share");
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    const trigger = Button.new(model.id)
      .h(24)
      .px(10)
      .flex()
      .items_center()
      .rounded(RADIUS)
      .bg(k.primary)
      .text_color(k.primaryForeground)
      .hover((el) => el.bg(k.primaryHover))
      .child(div().text_xs().child("Share"));
    return Popover.new("share-surface")
      .anchor("top_right")
      .open(this.open)
      .on_open_change((open, cx) => {
        this.open = open;
        this.choosing = false;
        cx.notify();
      })
      .trigger(trigger)
      .content(this.content(model, hidden, k));
  }
}
