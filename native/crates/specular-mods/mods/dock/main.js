// The dock, drawn from `panels/dock`: the controls of the selection in a
// bar under the tab row, with the tab's lens from `panels/toolbar` at its
// far end. A dropdown opens its sections in a popover under its button,
// which a press outside, Escape or a new selection closes; a field is the
// window's own (`Field`), so typing in it keeps the keys.
//
// A layout hides a control by listing its id in this mod's props:
//   "dock": { "mod": "dock", "props": { "hidden": ["item.annotate"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button, Popover } from "gpui-base";
import { state, props, colors, Icon, Field, Swatch } from "specular";
import { CONTROL, face, press, raised, sections } from "./list.js";

// A swatch's ring, and the dot inside it.
const SWATCH = 20;
const DOT = 12;
// The least width a field asks for (`FieldWidth`).
const WIDE = 220;

// What a control is called, with the key that does the same.
function hint(label, chord) {
  return chord ? `${label}  ${chord}` : label;
}

// The surface every list that opens has.
function surface(k) {
  return v_flex()
    .p(4)
    .gap(4)
    .rounded(10)
    .bg(k.popup)
    .border_1()
    .border_color(k.popupBorder)
    .shadow_md()
    .text_size(12)
    .text_color(k.text);
}

export default class Dock extends View {
  init() {
    // The dropdown of the bar whose list is open, the dropdown inside it
    // whose list is open, the selection it was opened for, and the row
    // whose fields are being typed in.
    this.open = null;
    this.more = null;
    this.openedFor = null;
    this.typing = null;
  }

  close() {
    this.open = null;
    this.more = null;
    this.typing = null;
  }

  // A button, a toggle or a dropdown's trigger.
  plain(id, k, { enabled = true, on = false, fills = false, label = null, onPress = null } = {}) {
    const button = Button.new(id)
      .h(CONTROL)
      .min_w(CONTROL)
      .px(4)
      .flex()
      .items_center()
      .justify_center()
      .gap(4)
      .rounded(6)
      .text_size(12)
      .when(fills, (el) => el.flex_1())
      .when(label != null, (el) => el.tooltip(label));
    if (!enabled) return button.disabled(true).opacity(0.5);
    const shown = on ? raised(button, k) : button.hover((el) => el.bg(k.hover));
    return onPress ? shown.on_click(onPress) : shown;
  }

  control(control, k, { inList = false } = {}) {
    const labelled = (f) => f?.icon != null && f?.text != null;
    switch (control.kind) {
      case "button":
        return this.plain(control.id, k, {
          enabled: control.enabled !== false,
          fills: inList && labelled(control.face),
          label: hint(control.label, control.chord),
          onPress: () => press(control.id),
        }).child(face(control.id, control.face, k));
      case "toggle":
        return this.plain(control.id, k, {
          enabled: control.enabled !== false,
          on: control.on && control.enabled !== false,
          fills: inList && labelled(control.face),
          label: hint(control.label, control.chord),
          onPress: () => press(control.id),
        }).child(face(control.id, control.face, k, { on: control.on }));
      case "dropdown":
        return inList ? this.dropdownRow(control, k) : this.dropdown(control, k);
      case "swatches":
        return h_flex().gap(4).children(control.options.map((option) => this.swatch(option, control, k)));
      case "stepper":
        return h_flex()
          .gap(2)
          .items_center()
          .child(this.step(`${control.id}.dec`, "−", control.can_decrement, k))
          .child(div().min_w(28).text_center().text_size(12).child(control.value))
          .child(this.step(`${control.id}.inc`, "+", control.can_increment, k));
      case "field":
        // An address takes its least width and grows into the room its row
        // has left; a number or a label keeps the width it asks for.
        return control.width === "wide"
          ? div().flex_1().min_w(WIDE).child(Field.new(control.id, { control: control.id }))
          : div().flex_shrink_0().child(Field.new(control.id, { control: control.id }));
      case "separator":
        return div().mx(4).w(1).h(16).flex_shrink_0().bg(k.divider);
      default:
        return div();
    }
  }

  step(id, sign, enabled, k) {
    return Button.new(id)
      .size(20)
      .flex()
      .items_center()
      .justify_center()
      .rounded(6)
      .text_size(14)
      .text_color(k.textMuted)
      .when(enabled === false, (el) => el.disabled(true).opacity(0.5))
      .when(enabled !== false, (el) => el.hover((el) => el.bg(k.hover)).on_click(() => press(id)))
      .child(sign);
  }

  swatch(option, control, k) {
    const enabled = control.enabled !== false;
    return Button.new(option.id)
      .size(SWATCH)
      .flex()
      .items_center()
      .justify_center()
      .rounded_full()
      .tooltip(option.label)
      .when(option.selected, (el) => el.border_2().border_color(k.textMuted))
      .when(!enabled, (el) => el.disabled(true).opacity(0.3))
      .when(enabled, (el) => el.on_click(() => press(option.id)))
      .child(Swatch.new(`${option.id}.dot`, { color: option.color ?? null, palette: control.palette, role: control.role, size: DOT }));
  }

  // A dropdown of the bar: its value and a caret, opening its list under it.
  dropdown(control, k) {
    const open = this.open === control.id;
    const trigger = this.plain(control.id, k, { on: false })
      .when(open, (el) => el.bg(k.on))
      .child(face(control.id, control.summary, k))
      .child(Icon.new(`${control.id}.caret`, { name: "chevron-down", size: 12, color: k.textMuted }));
    return Popover.new(`${control.id}.popover`)
      .open(open)
      .on_open_change((opened, cx) => {
        this.close();
        if (opened) {
          this.open = control.id;
          this.openedFor = this.selection;
        }
        cx.notify();
      })
      .trigger(trigger)
      .content(surface(k).child(sections(this, control.content, k)));
  }

  // A dropdown as a row of the list it is in: its value at the start and a
  // chevron at the far end, its own list opening from it.
  dropdownRow(control, k) {
    const open = this.more === control.id;
    const group = `${control.id}.row`;
    const trigger = Button.new(control.id)
      .group(group)
      .h(CONTROL)
      .w_full()
      .px(8)
      .flex()
      .items_center()
      .justify_between()
      .rounded(6)
      .text_size(12)
      .when(open, (el) => el.bg(k.on))
      .hover((el) => el.bg(k.hover))
      .child(face(control.id, control.summary, k, { filled: open, group }))
      .child(Icon.new(`${control.id}.chevron`, { name: "chevron-right", size: 12, color: open ? k.onText : k.textMuted }));
    return Popover.new(`${control.id}.popover`)
      .w_full()
      .open(open)
      .on_open_change((opened, cx) => {
        this.more = opened ? control.id : null;
        cx.notify();
      })
      .trigger(trigger)
      .content(surface(k).child(sections(this, control.content, k)));
  }

  render() {
    const k = colors();
    const hidden = new Set((props() || {}).hidden || []);
    const model = state("panels/dock");
    const lens = (state("panels/toolbar") || {}).view || [];
    this.selection = JSON.stringify(state("selection"));
    const controls = ((model || {}).controls || []).filter((control) => !hidden.has(control.id));
    // A list closes with its dropdown, and when the selection it was
    // opened for changes.
    const open = controls.find((control) => control.id === this.open && control.kind === "dropdown");
    if (open == null || this.openedFor !== this.selection) this.close();
    return h_flex()
      .id("dock")
      .size_full()
      .pl(8)
      .pr(12)
      .gap(4)
      .items_center()
      .bg(k.toolbar)
      .border_b_1()
      .border_color(k.toolbarBorder)
      .text_color(k.toolbarText)
      .child(
        h_flex()
          .flex_1()
          .min_w_0()
          .h_full()
          .px(4)
          .gap(4)
          .items_center()
          .overflow_hidden()
          .children(controls.map((each) => this.control(each, k))),
      )
      .child(h_flex().flex_shrink_0().gap(4).items_center().children(lens.map((each) => this.control(each, k))));
  }
}
