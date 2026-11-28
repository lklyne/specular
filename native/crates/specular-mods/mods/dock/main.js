// The dock, drawn from `panels/dock`: the controls of the selection, or of
// the tool in hand, in a bar under the tab row, with the tab's lens at its
// far end. A dropdown opens its choices under the bar; a field is the
// window's own (`Field`), so typing in it keeps the keys.
//
// A layout hides a control by listing its id in this mod's props:
//   "dock": { "mod": "dock", "props": { "hidden": ["comment.annotate"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, Icon, Field, Swatch, Popover } from "specular";

const H = 28;

const press = (id) => run("control.press", { id });

// What a closed control or a choice shows: its glyph, its word, or a dot.
function face(id, face, k) {
  if (face == null) return div();
  if (face.icon) return Icon.new(`${id}.glyph`, { name: face.icon, size: 16, color: k.toolbarText });
  if (face.text) return div().text_size(12).child(face.text);
  return Swatch.new(`${id}.dot`, { color: face.color ?? null, size: 12 });
}

function plain(id, child, on, k, onPress) {
  return Button.new(id)
    .h(H)
    .min_w(H)
    .px(6)
    .flex()
    .items_center()
    .justify_center()
    .gap(4)
    .rounded(6)
    .when(on, (el) => el.bg(k.on))
    .hover((el) => el.bg(k.hover))
    .on_click(onPress)
    .child(child);
}

export default class Dock extends View {
  init() {
    this.open = null;
  }

  control(control, k) {
    switch (control.kind) {
      case "button":
        return plain(control.id, face(control.id, control.face, k), false, k, () => press(control.id))
          .disabled(control.enabled === false);
      case "toggle":
        return plain(control.id, face(control.id, control.face, k), control.on, k, () => press(control.id));
      case "dropdown":
        return plain(control.id, face(control.id, control.summary, k), this.open === control.id, k, (_, cx) => {
          this.open = this.open === control.id ? null : control.id;
          cx.notify();
        });
      case "swatches":
        return h_flex().gap(2).children(control.options.map((option) =>
          plain(option.id, Swatch.new(`${option.id}.dot`, { color: option.color ?? null, palette: control.palette, role: control.role }), option.selected, k, () => press(option.id))));
      case "stepper":
        return h_flex()
          .gap(2)
          .child(plain(`${control.id}.dec`, "−", false, k, () => press(`${control.id}.dec`)))
          .child(div().text_size(12).min_w(24).child(control.value))
          .child(plain(`${control.id}.inc`, "+", false, k, () => press(`${control.id}.inc`)));
      case "field":
        return div().w(control.width === "wide" ? 220 : 96).child(Field.new(control.id, { control: control.id }));
      case "separator":
        return div().w(1).h(18).mx(4).bg(k.divider);
      default:
        return div();
    }
  }

  // The choices of the open dropdown, under the bar.
  choices(control, k) {
    const rows = [];
    for (const section of control.content) {
      if (section.options) {
        for (const option of section.options.options) {
          rows.push(
            Button.new(option.id)
              .h(26)
              .px(8)
              .flex()
              .items_center()
              .gap(8)
              .rounded(4)
              .text_size(12)
              .when(option.selected, (el) => el.bg(k.on))
              .hover((el) => el.bg(k.hover))
              .on_click((_, cx) => {
                this.open = null;
                press(option.id);
                cx.notify();
              })
              .child(face(option.id, option.face, k))
              .child(option.label),
          );
        }
      } else if (section.controls) {
        rows.push(h_flex().gap(2).p(4).children(section.controls.map((each) => this.control(each, k))));
      }
    }
    const list = v_flex()
      .min_w(180)
      .p(4)
      .rounded(8)
      .bg(k.popup)
      .border_1()
      .border_color(k.popupBorder)
      .shadow_md()
      .children(rows);
    // A press on the list must not reach the canvas under it.
    return Popover.new("dock.choices", { top: H + 8 }).child(list);
  }

  render() {
    const k = colors();
    const hidden = new Set((props() || {}).hidden || []);
    const model = state("panels/dock") || state("panels/tool_options");
    const lens = (state("panels/toolbar") || {}).view || [];
    const controls = ((model || {}).controls || []).filter((control) => !hidden.has(control.id));
    const open = controls.find((control) => control.id === this.open && control.kind === "dropdown");
    if (open == null) this.open = null;
    return h_flex()
      .id("dock")
      .relative()
      .size_full()
      .px(8)
      .gap(2)
      .items_center()
      .bg(k.toolbar)
      .border_b_1()
      .border_color(k.toolbarBorder)
      .text_color(k.toolbarText)
      .child(h_flex().flex_1().gap(2).items_center().children(controls.map((each) => this.control(each, k))))
      .child(h_flex().gap(2).children(lens.map((each) => this.control(each, k))))
      .when(open != null, (el) => el.child(this.choices(open, k)));
  }
}
