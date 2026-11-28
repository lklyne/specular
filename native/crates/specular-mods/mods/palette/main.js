// The command palette, drawn from `panels/palette` in an overlay near the
// top of the window (Command+K opens it when a layout names this mod for
// its `palette` overlay). Typing filters by label, group and keywords; the
// arrows move the pick, Return runs it, Escape closes the palette.
//
// A layout hides a command by listing its name in this mod's props:
//   "palette": { "mod": "palette", "props": { "hidden": ["app.quit"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, shared, colors, closeOverlay, Field } from "specular";

const ROW = 30;
const SHOWN = 12;

// Whether every word of `query` is in the item's words.
function matches(item, words) {
  const text = [item.label, item.group, ...(item.keywords || [])].join(" ").toLowerCase();
  return words.every((word) => text.includes(word));
}

export default class Palette extends View {
  init() {
    // An overlay is a new instance each time it opens, so this runs then.
    this.field = `palette-${Date.now()}`;
    this.pick = 0;
    this.submitted = shared(`field/${this.field}/submit`) || 0;
  }

  items() {
    const hidden = new Set((props() || {}).hidden || []);
    const query = (shared(`field/${this.field}`) || "").trim().toLowerCase();
    const words = query.split(/\s+/).filter(Boolean);
    return (state("panels/palette") || [])
      .filter((item) => item.enabled && item.command && !hidden.has(item.command))
      .filter((item) => matches(item, words));
  }

  choose(item) {
    closeOverlay(props().overlay);
    run(item.command, item.args || {});
  }

  render() {
    const k = colors();
    const items = this.items();
    this.pick = Math.min(this.pick, Math.max(items.length - 1, 0));
    const submit = shared(`field/${this.field}/submit`) || 0;
    if (submit !== this.submitted) {
      this.submitted = submit;
      if (items[this.pick]) this.choose(items[this.pick]);
    }
    const rows = items.slice(0, SHOWN).map((item, index) =>
      Button.new(`palette.${index}`)
        .h(ROW)
        .px(10)
        .flex()
        .items_center()
        .justify_between()
        .rounded(6)
        .text_size(13)
        .when(index === this.pick, (el) => el.bg(k.hover))
        .hover((el) => el.bg(k.hover))
        .on_click(() => this.choose(item))
        .child(
          h_flex()
            .gap(8)
            .child(div().text_color(k.textMuted).child(item.group))
            .child(div().text_color(k.text).child(item.label)),
        )
        .when(item.chord != null, (el) => el.child(div().text_color(k.textMuted).child(item.chord))),
    );
    return v_flex()
      .id("palette")
      .w_full()
      .p(6)
      .gap(4)
      .text_color(k.text)
      .on_key_down((event, cx) => {
        if (event.key === "down") this.pick = Math.min(this.pick + 1, items.length - 1);
        else if (event.key === "up") this.pick = Math.max(this.pick - 1, 0);
        else return;
        cx.notify();
      })
      .child(Field.new("palette.query", { id: this.field, placeholder: "Type a command", autofocus: true }))
      .child(div().h(1).bg(k.divider))
      .children(rows.length > 0 ? rows : [div().p(10).text_color(k.textMuted).child("No command matches")]);
  }
}
