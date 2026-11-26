// The toolbar: the tools, the theme button and the zoom readout, drawn from
// `panels/toolbar`. Docked, it fills the strip down the canvas's left edge
// the window gives it, each button named beside it; floating, it is the
// ribbon the window centres over the top or the bottom of the canvas. The
// options of the tool in hand open beside its button.
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

// The fill, rim and shadow of the button that is on.
function raised(el, k) {
  return el
    .bg(k.raisedFill)
    .border_1()
    .border_color(k.raisedRim)
    .shadow([
      { y: 1, blur: 2, spread: -1, color: k.raisedShadowNear },
      { y: 2, blur: 4, spread: -2, color: k.raisedShadowFar },
    ]);
}

// A name shown while the pointer is over `group`: right of a docked button,
// under or over a floating one.
function label(group, text, place, k) {
  const at = div().absolute().flex().items_center().opacity(0);
  const placed =
    place === "docked"
      ? at.left(BUTTON + GAP).top(0).h(BUTTON)
      : place === "top"
        ? at.left(0).top(BUTTON + GAP)
        : at.left(0).bottom(BUTTON + GAP);
  return placed.group_hover(group, (el) => el.opacity(1)).child(
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

function button(id, icon, opts, k) {
  const { active = false, tint = null, onPress } = opts;
  const face = Button.new(id)
    .size(BUTTON)
    .flex()
    .items_center()
    .justify_center()
    .rounded(6)
    .selected(active)
    .on_click(onPress);
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
  const upright = place !== "docked";
  return div()
    .m_1()
    .w(upright ? 1 : 16)
    .h(upright ? 16 : 1)
    .flex_shrink_0()
    .bg(k.divider);
}

export default class Toolbar extends View {
  render() {
    const bar = state("panels/toolbar");
    const options = state("panels/tool_options");
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    const place = bar.tools;
    const docked = place === "docked";

    const press = (id) => () => run("control.press", { id });
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
            .group(tool.id)
            .relative()
            .child(
              button(tool.id, tool.icon, {
                active: tool.active,
                tint: tool.tint,
                onPress: press(tool.id),
              }, k),
            )
            .when(!open, (el) => el.child(label(tool.id, hint(tool.label, tool.chord), place, k)))
            .when(open, (el) => el.child(ToolOptions.new("tool-options", { place }))),
        );
      }
    });

    const zoom = bar.sections.find((section) => section.zoom);
    const readout =
      zoom && !hidden.has(zoom.zoom.id)
        ? Zoom.new(zoom.zoom.id, { compact: docked })
        : null;
    const theme = hidden.has(bar.theme.id)
      ? null
      : div()
          .group(bar.theme.id)
          .relative()
          .child(button(bar.theme.id, bar.theme.icon, { onPress: press(bar.theme.id) }, k))
          .child(label(bar.theme.id, bar.theme.label, place, k));

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

    // Floating, the window centres the ribbon over the canvas.
    return h_flex()
      .id("ribbon")
      .p(PAD)
      .gap(4)
      .items_center()
      .rounded(10)
      .bg(k.toolbar)
      .border_1()
      .border_color(k.popupBorder)
      .shadow_md()
      .text_color(k.toolbarText)
      .children(items)
      .when(theme != null || readout != null, (el) => el.child(rule(place, k)))
      .when(theme != null, (el) => el.child(theme))
      .when(readout != null, (el) => el.child(readout));
  }
}
