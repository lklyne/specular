// The tab row, drawn from `panels/strip`: the window's title bar, with the
// traffic lights in its left padding, then a tab for the canvas and one for
// each page and Document, the add button, bare title bar to drag the window
// by, and at the far end the zoom readout (while the tools are the app's
// own), the Share button and the right panel's toggle.
//
// A layout hides a control by listing its id in this mod's props:
//   { "mod": "tabs", "props": { "hidden": ["view.add", "chat.toggle"] } }
import { View, div } from "gpui-kit";
import { h_flex, Button } from "gpui-base";
import { state, props, run, colors, Icon, Favicon, Zoom, Share, WindowDrag } from "specular";

const ROW = 38;
const TRAFFIC_LIGHTS = 86;
const TAB_WIDTH = 180;
const TAB_LEAST = 28;
const TAB = 28;
const GLYPH = 14;
const CLOSE = 18;
const CLOSE_GLYPH = 12;
const BARE = 80;
const PANEL_GLYPH = 16;

const press = (id) => (_event, cx) => {
  if (cx) cx.stop_propagation();
  run("control.press", { id });
};

// The fill, rim and shadow of the tab that is showing.
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

// The button at a tab's far end that closes it: always there on the tab
// showing, and on another while the pointer is over it.
function close(tab, k) {
  const id = `${tab.id}.close`;
  const face = Button.new(id)
    .absolute()
    .right(5)
    .top((TAB - CLOSE) / 2)
    .size(CLOSE)
    .flex()
    .items_center()
    .justify_center()
    .rounded(4)
    .bg(tab.active ? k.raisedFill : k.tabHover)
    .hover((el) => el.bg(tab.active ? k.tabHover : k.raisedFill))
    .on_click(press(id))
    .child(Icon.new(`${id}.glyph`, { control: id, name: "close", size: CLOSE_GLYPH, color: k.toolbarText }));
  return tab.active ? face : face.opacity(0).group_hover(tab.id, (el) => el.opacity(1));
}

function tab(model, k) {
  const text = model.active ? k.toolbarTextStrong : k.toolbarText;
  const face = h_flex()
    .size_full()
    .px(6)
    .gap(6)
    .items_center()
    .overflow_hidden()
    .child(Favicon.new(`${model.id}.icon`, {
      tab: model.id,
      icon: model.icon,
      size: GLYPH,
      color: text,
      control: model.id,
    }))
    .child(div().flex_1().min_w(0).overflow_hidden().whitespace_nowrap().text_ellipsis().child(model.label));
  const shell = Button.new(model.id)
    .group(model.id)
    .relative()
    .w(TAB_WIDTH)
    .min_w(TAB_LEAST)
    .h(TAB)
    .flex_shrink_1()
    .rounded(6)
    .text_color(text)
    .on_click(press(model.id))
    .child(face);
  const shown = model.active ? raised(shell, k) : shell.hover((el) => el.bg(k.tabHover));
  return model.id === "view.canvas" ? shown : shown.child(close(model, k));
}

function add(model, k) {
  return Button.new(model.id)
    .size(TAB)
    .flex_shrink_0()
    .flex()
    .items_center()
    .justify_center()
    .rounded(6)
    .hover((el) => el.bg(k.tabHover))
    .on_click(press(model.id))
    .child(Icon.new(`${model.id}.glyph`, {
      control: model.id,
      name: model.face.icon,
      size: GLYPH,
      color: k.toolbarText,
    }));
}

function panelToggle(model, k) {
  return Button.new(model.id)
    .size(TAB)
    .flex()
    .items_center()
    .justify_center()
    .rounded(8)
    .hover((el) => el.bg(k.tabHover))
    .on_click(press(model.id))
    .child(
      div()
        .when(!model.open, (el) => el.opacity(0.6))
        .child(Icon.new(`${model.id}.glyph`, {
          control: model.id,
          name: model.icon,
          size: PANEL_GLYPH,
          color: k.toolbarText,
        })),
    );
}

export default class Tabs extends View {
  render() {
    const strip = state("panels/strip");
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    const shown = (control) => control != null && !hidden.has(control.id);

    const tabs = strip.tabs.filter(shown).map((model) => tab(model, k));
    return h_flex()
      .id("tab-row")
      .size_full()
      .h(ROW)
      .pl(TRAFFIC_LIGHTS)
      .items_center()
      .bg(k.toolbar)
      .child(
        h_flex()
          .min_w(0)
          .flex_shrink_1()
          .gap(4)
          .items_center()
          .overflow_hidden()
          .children(tabs),
      )
      .when(shown(strip.add), (el) => el.child(div().pl(4).flex_shrink_0().child(add(strip.add, k))))
      .child(
        div()
          .flex_1()
          .flex_shrink_0()
          .min_w(BARE)
          .h_full()
          .child(WindowDrag.new("tabs-bare", {})),
      )
      .child(
        h_flex()
          .pr(12)
          .gap(4)
          .flex_shrink_0()
          .items_center()
          .text_color(k.toolbarText)
          .when(shown(strip.zoom), (el) => el.child(Zoom.new(strip.zoom.id, { compact: false })))
          .when(!hidden.has("share"), (el) => el.child(Share.new("share", {})))
          .when(shown(strip.chat), (el) => el.child(panelToggle(strip.chat, k))),
      );
  }
}
