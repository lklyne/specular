// The toolbar: the tools, the theme button and the zoom readout, drawn from
// `panels/toolbar`. Docked, it fills the strip down the canvas's left edge
// the window gives it, each button named beside it; floating, it is the
// ribbon the window centres over the top or the bottom of the canvas, or
// the column it centres down the left edge while the sidebar is hidden
// (`tools` is "left"). The options of the tool in hand open beside its
// button.
//
// A layout hides a control by listing its id in this mod's props:
//   { "mod": "toolbar", "props": { "hidden": ["tool.inspect", "zoom"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, Icon, Zoom, ToolOptions } from "specular";

const BUTTON = 28;
const GLYPH = 20;
const PAD = 4;
const GAP = 10;

// What a button is called, with the key that does the same.
function hint(label, chord) {
  return chord ? `${label}  ${chord}` : label;
}

// The fill, rim and shadow of the one that is on: a face lit along its
// top, a hairline ring that keeps to its corners, and a soft shadow under
// it, short enough to stay inside the 5 px round a tab in its row or a
// tool in its column, so nothing drawn beside them cuts it off.
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

// Whether the tools run down a column: docked, or floating on the left.
function upright(place) {
  return place === "docked" || place === "left";
}

// The name of the button the pointer is over: right of a button in a
// column, under or over one in a row. Only that one is built, so a render
// costs one label rather than one per button.
function label(text, place, k) {
  const at = div().absolute().flex().items_center();
  const placed =
    upright(place)
      ? at.left(BUTTON + GAP).top(0).h(BUTTON)
      : place === "top"
        ? at.left(0).top(BUTTON + GAP)
        : at.left(0).bottom(BUTTON + GAP);
  return placed.child(
    div()
      .px(8)
      .py(4)
      .rounded(6)
      .bg(k.popup)
      .border_1()
      .border_color(k.popupBorder)
      .shadow_md()
      .whitespace_nowrap()
      .text_color(k.text)
      .child(text),
  );
}

// The surface of the tools floating over the canvas: rounded, a hairline
// rim and a soft shadow.
function floating(el, k) {
  return el
    .id("ribbon")
    .p(PAD)
    .gap(4)
    .items_center()
    .rounded(10)
    .bg(k.toolbar)
    .border_1()
    .border_color(k.popupBorder)
    .shadow_md()
    .text_color(k.toolbarText);
}

function button(id, icon, opts, k) {
  const { active = false, tint = null, onPress, onHover } = opts;
  const face = Button.new(id)
    .size(BUTTON)
    .flex()
    .items_center()
    .justify_center()
    .rounded(6)
    .selected(active)
    .on_click(onPress)
    .on_hover(onHover);
  const shown = active ? raised(face, k) : face.hover((el) => el.bg(k.hover));
  return shown.child(
    Icon.new(`${id}.glyph`, {
      control: id,
      name: icon,
      tint,
      active,
      size: GLYPH,
      color: active ? k.toolbarTextStrong : k.toolbarText,
    }),
  );
}

function rule(place, k) {
  const across = !upright(place);
  return div()
    .m_1()
    .w(across ? 1 : 16)
    .h(across ? 16 : 1)
    .flex_shrink_0()
    .bg(k.divider);
}

export default class Toolbar extends View {
  init() {
    this.hovered = null;
    this.presses = new Map();
    this.hovers = new Map();
  }

  // One handler per control, kept across renders.
  press(id) {
    let handler = this.presses.get(id);
    if (!handler) {
      handler = () => run("control.press", { id });
      this.presses.set(id, handler);
    }
    return handler;
  }

  // The pointer entering or leaving a button: only the hovered button's
  // name is drawn, so a change of hover renders again.
  hover(id) {
    let handler = this.hovers.get(id);
    if (!handler) {
      handler = (hovered, cx) => {
        const next = hovered ? id : this.hovered === id ? null : this.hovered;
        if (next !== this.hovered) {
          this.hovered = next;
          cx.notify();
        }
      };
      this.hovers.set(id, handler);
    }
    return handler;
  }

  render() {
    const bar = state("panels/toolbar");
    const options = state("panels/tool_options");
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    const place = bar.tools;
    const docked = place === "docked";

    const groups = bar.sections
      .filter((section) => section.tools)
      .map((section) => section.tools.filter((tool) => !hidden.has(tool.id)))
      .filter((tools) => tools.length > 0);

    const items = [];
    groups.forEach((tools, index) => {
      if (index > 0) items.push(rule(place, k));
      for (const tool of tools) {
        const open = tool.active && options != null;
        items.push(
          div()
            .relative()
            .child(
              button(tool.id, tool.icon, {
                active: tool.active,
                tint: tool.tint,
                onPress: this.press(tool.id),
                onHover: this.hover(tool.id),
              }, k),
            )
            .when(!open && this.hovered === tool.id, (el) =>
              el.child(label(hint(tool.label, tool.chord), place, k)),
            )
            .when(open, (el) => el.child(ToolOptions.new("tool-options", { place }))),
        );
      }
    });

    const zoom = bar.sections.find((section) => section.zoom);
    const readout =
      zoom && !hidden.has(zoom.zoom.id)
        ? Zoom.new(zoom.zoom.id, { compact: upright(place) })
        : null;
    const theme = hidden.has(bar.theme.id)
      ? null
      : div()
          .relative()
          .child(
            button(bar.theme.id, bar.theme.icon, {
              onPress: this.press(bar.theme.id),
              onHover: this.hover(bar.theme.id),
            }, k),
          )
          .when(this.hovered === bar.theme.id, (el) => el.child(label(bar.theme.label, place, k)));

    if (docked) {
      return v_flex()
        .id("ribbon")
        .size_full()
        .py(PAD)
        .gap(4)
        .items_center()
        .bg(k.toolbar)
        .border_r_1()
        .border_color(k.toolbarBorder)
        .text_color(k.toolbarText)
        .children(items)
        .child(div().flex_1())
        .when(readout != null, (el) => el.child(div().w_full().px(2).child(readout)))
        .when(theme != null, (el) => el.child(theme));
    }

    // Floating, the window centres the panel over the canvas.
    const panel = floating(upright(place) ? v_flex() : h_flex(), k)
      .children(items)
      .when(theme != null || readout != null, (el) => el.child(rule(place, k)));
    if (upright(place)) {
      return panel
        .when(readout != null, (el) => el.child(div().w_full().child(readout)))
        .when(theme != null, (el) => el.child(theme));
    }
    return panel
      .when(theme != null, (el) => el.child(theme))
      .when(readout != null, (el) => el.child(readout));
  }
}
