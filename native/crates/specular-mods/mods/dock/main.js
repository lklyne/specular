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
import { state, props, run, colors, Icon, Field, Swatch } from "specular";

// The side of a control, and of a row of a list.
const CONTROL = 24;
// A swatch's ring, and the dot inside it.
const SWATCH = 20;
const DOT = 12;
// A cell of a row or grid of glyphs.
const CELL = 32;
// The least width a field asks for (`FieldWidth`).
const WIDE = 220;

const press = (id) => run("control.press", { id });

// What a control is called, with the key that does the same.
function hint(label, chord) {
  return chord ? `${label}  ${chord}` : label;
}

// A glyph's size: back and forward are thin strokes and read small, and
// reload, with the stop it turns into, keeps in step with them.
function glyphSize(icon) {
  if (icon === "chevron-left" || icon === "chevron-right") return 18;
  if (icon === "reload" || icon === "stop") return 16;
  return 14;
}

// The fill, rim and shadow of a toggle that is on.
function raised(el, k) {
  return el
    .bg(k.raisedFill)
    .border_t_1()
    .border_color(k.raisedRimLit)
    .shadow([
      { spread: 0.5, color: k.raisedRim },
      { y: 1, blur: 2, color: k.raisedShadowNear },
      { y: 1, blur: 3, spread: -1, color: k.raisedShadowFar },
    ]);
}

// What a control or a choice shows: its glyph, its word, or a dot of its
// colour. `on` draws it in the full text colour and at rest in the muted
// one; `filled` is a row with the `on` fill, whose word takes `onText`;
// `group` is the row whose hover turns the word `hoverText`.
function face(id, face, k, { on = false, filled = false, group = null } = {}) {
  if (face == null) return div();
  let row = h_flex().gap(4).items_center();
  if (face.icon) {
    row = row.child(
      Icon.new(`${id}.glyph`, {
        name: face.icon,
        size: glyphSize(face.icon),
        color: on ? k.text : k.textMuted,
        tint: face.color ?? null,
        active: on,
      }),
    );
  }
  if (face.text) {
    row = row.child(
      div()
        .whitespace_nowrap()
        .text_color(filled ? k.onText : on ? k.text : k.textMuted)
        .when(face.font === "mono", (el) => el.font_family("Geist Mono"))
        .when(group != null, (el) => el.group_hover(group, (el) => el.text_color(k.hoverText)))
        .child(face.text),
    );
  }
  if (!face.icon && !face.text) {
    row = row.child(Swatch.new(`${id}.dot`, { color: face.color ?? null, palette: "vivid", role: "ink", size: 16 }));
  }
  return row;
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

const rule = (k) => div().h(1).w_full().bg(k.divider);

const segmented = (section) => section?.options?.layout?.kind === "segmented";
// A row that opens more of a list ends the rows over it.
const opensMore = (section) =>
  section?.controls != null && section.controls.length > 0 && section.controls.every((c) => c.kind === "dropdown");

// How a list's rows are drawn: with far-end text or a key, the current row
// filled and the others muted ("presets"); else every row in the text
// colour and the current one checked ("menu").
function rowsOf(content) {
  const detailed = content.some((section) =>
    (section.options?.options || []).some((option) => option.trailing != null || option.chord != null));
  return detailed ? "presets" : "menu";
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
      .content(surface(k).child(this.sections(control.content, k)));
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
      .content(surface(k).child(this.sections(control.content, k)));
  }

  // One choice: a row of a list (`rows` is "menu" or "presets"), or a cell
  // of a row or grid of glyphs (`rows` is null).
  option(option, rows, k) {
    const menu = rows === "menu";
    const filled = option.selected && !menu;
    const closes = (option.fields || []).length === 0;
    const height = rows == null ? CELL : option.face?.font ? 28 : CONTROL;
    const group = option.id;
    let row = Button.new(option.id)
      .group(group)
      .flex()
      .items_center()
      .justify_between()
      .gap(16)
      .h(height)
      .rounded(6)
      .text_size(12)
      .when(rows != null, (el) => el.w_full().px(8))
      .when(rows == null, (el) => el.w(CELL).justify_center())
      .when(!closes, (el) => el.pr(3))
      .when(filled, (el) => el.bg(k.on))
      .when(option.enabled === false, (el) => el.disabled(true).opacity(0.4))
      .when(option.enabled !== false, (el) =>
        el.hover((el) => el.bg(k.hover)).on_click((_, cx) => {
          if (closes) this.close();
          press(option.id);
          cx.notify();
        }))
      .child(face(option.id, option.face, k, { on: option.selected || menu, filled, group }));
    if (option.selected && menu) {
      row = row.child(Icon.new(`${option.id}.check`, { name: "check", size: 12, color: k.text }));
    }
    if (option.trailing != null) {
      row = row.child(
        div()
          .whitespace_nowrap()
          .text_color(filled ? k.onText : k.textMuted)
          .group_hover(group, (el) => el.text_color(k.hoverText))
          .child(option.trailing),
      );
    }
    if (!closes) row = row.child(this.fields(option, k));
    if (option.chord != null) {
      row = row.child(div().px(6).py(2).rounded(4).bg(k.on).text_color(k.keyText).child(option.chord));
    }
    return row;
  }

  // A choice's own fields at the far end of its row. They keep their room
  // and show while the row is the current value, is under the pointer, or
  // one of them is being typed in.
  fields(option, k) {
    const shown = option.selected || this.typing === option.id;
    return h_flex()
      .gap(8)
      .items_center()
      // A press in a field is typing, not a choice of the row.
      .on_mouse_down("left", (_, cx) => {
        cx.stop_propagation();
        if (this.typing !== option.id) {
          this.typing = option.id;
          cx.notify();
        }
      })
      .when(!shown, (el) => el.invisible().group_hover(option.id, (el) => el.visible()))
      .children(option.fields.map((field) => Field.new(field.id, { control: field.id })));
  }

  // A line that sets one thing: its title, and the values as the segments
  // of one control at the far end.
  segmentedRow(title, options, k) {
    const segments = options.map((option) =>
      Button.new(option.id)
        .h(16)
        .px(8)
        .flex()
        .items_center()
        .rounded(4)
        .text_size(12)
        .text_color(option.selected ? k.text : k.textMuted)
        .when(option.selected, (el) => raised(el, k))
        .when(option.enabled === false, (el) => el.disabled(true).opacity(0.4))
        .when(option.enabled !== false && !option.selected, (el) => el.on_click(() => press(option.id)))
        .child(option.face?.text ?? option.label));
    return h_flex()
      .w_full()
      .h(CONTROL)
      .pl(8)
      .gap(16)
      .items_center()
      .justify_between()
      .child(div().text_color(k.textMuted).child(title))
      .child(h_flex().h(20).px(2).gap(2).items_center().rounded(6).bg(k.on).children(segments));
  }

  section(section, rows, k) {
    if (section.controls) {
      return h_flex().gap(4).items_center().children(section.controls.map((each) => this.control(each, k, { inList: true })));
    }
    const { layout, options } = section.options;
    switch (layout.kind) {
      case "segmented":
        return this.segmentedRow(layout.title, options, k);
      case "row":
        return h_flex().gap(2).children(options.map((option) => this.option(option, null, k)));
      case "grid":
        return h_flex()
          .flex_wrap()
          .gap(2)
          .w(layout.columns * CELL + (layout.columns - 1) * 2)
          .children(options.map((option) => this.option(option, null, k)));
      default:
        return v_flex().gap(2).min_w(140).children(options.map((option) => this.option(option, rows, k)));
    }
  }

  // Sections one under another, with a line between blocks. Lines that
  // each set one thing read as one block.
  sections(content, k) {
    const rows = rowsOf(content);
    return v_flex().gap(4).children(content.map((one, index) => {
      const next = content[index + 1];
      const joined = (segmented(one) && segmented(next)) || opensMore(next);
      return v_flex()
        .gap(4)
        .child(this.section(one, rows, k))
        .when(next != null && !joined, (el) => el.child(rule(k)));
    }));
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
