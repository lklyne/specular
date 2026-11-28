// The tab row, drawn from `panels/strip`: the window's title bar, with the
// traffic lights in its left padding (unless the sidebar is shown up to the
// window's top and holds them: `lights` is false) and the sidebar's toggle
// (`sidebar`, null while the sidebar holds it), then a tab for the canvas and one for
// each page and Document, the add button, bare title bar to drag the window
// by, and at the far end the zoom readout (while the tools are the app's
// own), the agents on the canvas, the Share button and the right panel's
// toggle.
//
// The agents are badges drawn from the `agents` key, up to three overlapping
// and a count for the rest. A click lists them all; picking one runs
// `view.follow`, which keeps the camera on its cursor until the person moves
// the camera or presses Escape.
//
// A layout hides a control by listing its id in this mod's props:
//   { "mod": "tabs", "props": { "hidden": ["view.add", "chat.toggle", "sidebar.toggle", "agents"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button, Popover } from "gpui-base";
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
const PANEL_GLYPH = 14;
const BADGE = 22;
const BADGE_STEP = 14;
const BADGES = 3;
const LIST_WIDTH = 240;
const LIST_ROW = 30;

const press = (id) => (_event, cx) => {
  if (cx) cx.stop_propagation();
  run("control.press", { id });
};

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

// What a badge shows: the number of an "Agent 2", else the first letter.
function initial(name) {
  const number = /^Agent (\d+)$/.exec(name);
  return number ? number[1] : (name.trim()[0] || "?").toUpperCase();
}

function badge(agent, k) {
  return div()
    .size(BADGE)
    .flex()
    .items_center()
    .justify_center()
    .rounded(BADGE / 2)
    .border_1()
    .border_color(k.toolbar)
    .bg(agent.color)
    .text_color("#ffffff")
    .text_xs()
    .child(initial(agent.name));
}

function agentRow(agent, following, close, k) {
  const followed = agent.id === following;
  return Button.new(`agents.${agent.id}`)
    .w_full()
    .h(LIST_ROW)
    .px(8)
    .flex()
    .items_center()
    .gap(8)
    .rounded(6)
    .hover((el) => el.bg(k.hover))
    .on_click((_event, cx) => {
      close(cx);
      run("view.follow", followed ? {} : { agent: agent.id });
    })
    .child(div().size(10).flex_shrink_0().rounded(5).bg(agent.color))
    .child(div().flex_1().min_w(0).truncate().text_sm().child(agent.name))
    .child(div().flex_shrink_0().text_xs().text_color(k.textMuted).child(followed ? "following" : agent.label));
}

export default class Tabs extends View {
  init() {
    this.agentsOpen = false;
  }

  // The badges, overlapping, and the list a click opens.
  agents(roster, k) {
    const shown = roster.agents.slice(0, BADGES);
    const more = roster.agents.length - shown.length;
    const width = BADGE + (shown.length - 1) * BADGE_STEP;
    const stack = div()
      .relative()
      .w(width)
      .h(BADGE)
      .children(shown.map((agent, index) => div().absolute().top(0).left(index * BADGE_STEP).child(badge(agent, k))));
    const trigger = Button.new("agents")
      .h(BADGE + 4)
      .px(2)
      .flex()
      .items_center()
      .gap(4)
      .rounded(BADGE / 2 + 2)
      .hover((el) => el.bg(k.hover))
      .child(stack)
      .when(more > 0, (el) => el.child(div().pr(4).text_xs().text_color(k.toolbarText).child(`+${more}`)));
    const close = (cx) => {
      this.agentsOpen = false;
      cx.notify();
    };
    const list = v_flex()
      .id("agents-list")
      .w(LIST_WIDTH)
      .p(6)
      .gap(2)
      .rounded(10)
      .bg(k.popup)
      .border_1()
      .border_color(k.popupBorder)
      .shadow_md()
      .text_color(k.text)
      .child(div().px(8).py(4).text_xs().text_color(k.textMuted).child("Agents on this canvas"))
      .children(roster.agents.map((agent) => agentRow(agent, roster.following, close, k)));
    return Popover.new("agents-surface")
      .anchor("top_right")
      .open(this.agentsOpen)
      .on_open_change((open, cx) => {
        this.agentsOpen = open;
        cx.notify();
      })
      .trigger(trigger)
      .content(list);
  }

  render() {
    const strip = state("panels/strip");
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    const shown = (control) => control != null && !hidden.has(control.id);

    const tabs = strip.tabs.filter(shown).map((model) => tab(model, k));
    const roster = state("agents");
    const agents = !hidden.has("agents") && roster != null && roster.agents.length > 0;
    return h_flex()
      .id("tab-row")
      .size_full()
      .h(ROW)
      .pl(strip.lights === false ? 8 : TRAFFIC_LIGHTS - 4)
      .items_center()
      .bg(k.toolbar)
      .when(shown(strip.sidebar), (el) => el.child(div().pl(4).pr(4).flex_shrink_0().child(panelToggle(strip.sidebar, k))))
      .child(
        // The tabs are cut off past the row's room, but not their shadows:
        // the box they are cut to reaches 5 px round each.
        h_flex()
          .min_w(0)
          .flex_shrink_1()
          .px(4)
          .py(5)
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
          .when(agents, (el) => el.child(this.agents(roster, k)))
          .when(!hidden.has("share"), (el) => el.child(Share.new("share", {})))
          .when(shown(strip.chat), (el) => el.child(panelToggle(strip.chat, k))),
      );
  }
}
