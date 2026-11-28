// The sidebar, drawn from `panels/sidebar`: a section for each project
// (ADR 0062), a plain folder or a repo, with its canvases under it; then
// the section mods (`"section": "sidebar"`), such as Notes and Pages, where
// `SidebarSections` is placed. Each section folds at its head. Under the
// pointer a project's head shows a + for a new canvas in it and, at the far
// right, its fold chevron. A footer has a + that opens a folder or a repo
// as a project. A click presses a row by its id, as the app's own sidebar
// does; a double click on a canvas asks for its new name in the `prompt`
// dialog.
// A right press on a canvas opens its menu at the pointer (`menu.open`),
// whose Rename types the name in place; on a project's head it opens the
// project's card (the `project` mod) beside the sidebar.
//
// A layout hides a section or a row by listing its id in this mod's props:
//   { "mod": "sidebar", "props": { "hidden": ["sidebar.open"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, openOverlay, Field, Icon, SidebarSections, WindowDrag } from "specular";

// The sizes are the app's own sidebar's (`kit/view/sidebar.rs`).
const ROW = 28;
const ROW_LEFT = 22;
const ROW_RIGHT = 16;
const HEAD = 36;
const HEAD_PAD = 12;
const HEAD_BUTTON = 22;
const ADD = 26;
const ICON = 14;
const FOLD = 12;
const HOVERED = "project-head";
const TITLE = 38;
const TRAFFIC_LIGHTS = 86;

const press = (id) => run("control.press", { id });

// Opens the app's menu for a canvas where the pointer is.
const canvasMenu = (row) => (event) =>
  run("menu.open", { target: "canvas", id: row.canvas, at: [event.position.x, event.position.y] });

// Opens a project's card beside its head, past the sidebar's right edge.
const CARD_GAP = 8;
const projectCard = (section) => (event) => {
  const head = event.bounds;
  const at = head
    ? [head.x + head.width + CARD_GAP, head.y]
    : [event.position.x + CARD_GAP, event.position.y];
  openOverlay("project", { kind: "popup", at, width: 300, props: { project: section.project } });
};

function rename(row) {
  openOverlay("prompt", {
    id: "sidebar.rename",
    props: {
      title: "Rename canvas",
      value: row.label,
      confirm: "Rename",
      command: "canvas.rename",
      args: { canvas: row.canvas },
      key: "name",
    },
  });
}

// A canvas: its glyph, its name, and a check while it is the one shown.
function canvasRow(row, k) {
  return Button.new(row.id)
    .h(ROW)
    .flex_shrink_0()
    .pl(ROW_LEFT)
    .pr(ROW_RIGHT)
    .flex()
    .items_center()
    .gap(8)
    .hover((el) => el.bg(k.interactiveHover))
    .on_click((event) => {
      if (row.renaming) return;
      if (event.click_count >= 2) rename(row);
      else press(row.id);
    })
    .on_mouse_down("right", canvasMenu(row))
    .child(Icon.new(`${row.id}.glyph`, { name: "file", size: ICON, color: k.textMuted }))
    .child(
      row.renaming
        ? div().flex_1().min_w(0).child(Field.new(row.rename, { control: row.rename }))
        : div().flex_1().min_w(0).truncate().child(row.label),
    )
    .when(row.active && !row.renaming, (el) =>
      el.child(Icon.new(`${row.id}.check`, { name: "check", size: ICON, color: k.text })),
    );
}

// A button on a project's head: a glyph in a box filled under the pointer.
// With no `onPress` (the chevron) a press on it is the head's, which folds.
function headButton(id, icon, size, k, onPress) {
  const button = Button.new(id)
    .size(HEAD_BUTTON)
    .flex()
    .flex_shrink_0()
    .items_center()
    .justify_center()
    .rounded(6)
    .hover((el) => el.bg(k.interactive))
    .child(Icon.new(`${id}.glyph`, { name: icon, size, color: k.textMuted }));
  if (onPress == null) return button;
  return button.on_click((event, cx) => {
    // The project head under it has a press of its own.
    if (cx) cx.stop_propagation();
    onPress();
  });
}

// A project's head: its name and branch, which fold it. Under the pointer
// it shows a + for a new canvas in it and, at the far right, the chevron.
function projectHead(section, k) {
  const title = section.branch != null ? `${section.title} \u00b7 ${section.branch}` : section.title;
  return Button.new(section.id)
    .group(HOVERED)
    .h(ROW)
    .flex_shrink_0()
    .px(HEAD_PAD)
    .flex()
    .items_center()
    .gap(4)
    .text_color(section.active ? k.text : k.textMuted)
    .on_click(() => press(section.id))
    .on_mouse_down("right", projectCard(section))
    .child(div().flex_1().min_w(0).truncate().child(title))
    .child(
      h_flex()
        .flex_shrink_0()
        .gap(2)
        .invisible()
        .group_hover(HOVERED, (el) => el.visible())
        .child(headButton(section.add, "plus", ICON, k, () => press(section.add)))
        .child(headButton(`${section.id}.fold`, section.folded ? "chevron-right" : "chevron-down", FOLD, k)),
    );
}

function section(section, hidden, n, k) {
  const out = v_flex().flex_shrink_0().when(n > 0, (el) => el.pt(12)).child(projectHead(section, k));
  return out.children(section.rows.filter((row) => !hidden.has(row.id)).map((row) => canvasRow(row, k)));
}

// Canvases in no project: a head with a + for a new one, then the rows.
function canvases(section, model, hidden, k) {
  const head = h_flex()
    .h(HEAD)
    .flex_shrink_0()
    .px(HEAD_PAD)
    .gap(4)
    .items_center()
    .child(
      Button.new(section.id)
        .h(HEAD)
        .flex_1()
        .min_w(0)
        .flex()
        .items_center()
        .gap(6)
        .font_weight(500)
        .on_click(() => press(section.id))
        .child(div().min_w(0).truncate().child(section.title))
        .child(Icon.new(`${section.id}.glyph`, { name: section.folded ? "chevron-right" : "chevron-down", size: FOLD, color: k.text })),
    )
    .when(!hidden.has(model.add), (el) =>
      el.child(
        Button.new(model.add)
          .size(ADD)
          .flex()
          .flex_shrink_0()
          .items_center()
          .justify_center()
          .rounded(8)
          .hover((el) => el.bg(k.interactive))
          .on_click(() => press(model.add))
          .child(Icon.new(`${model.add}.glyph`, { name: "plus", size: ICON, color: k.text })),
      ),
    );
  const rows = section.folded ? [] : section.rows.filter((row) => !hidden.has(row.id)).map((row) => canvasRow(row, k));
  return v_flex().flex_shrink_0().child(head).children(rows);
}

export default class Sidebar extends View {
  render() {
    const model = state("panels/sidebar");
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    if (model == null) return div();
    // The notes and pages are the `notes` and `pages` section mods' to draw.
    const sections = model.sections.filter(
      (each) => !hidden.has(each.id) && each.kind !== "notes" && each.kind !== "pages",
    );
    const list = v_flex()
      .id("sidebar.list")
      .flex_1()
      .min_h_0()
      .overflow_y_scroll()
      .child(
        v_flex()
          .flex_shrink_0()
          .pt(8)
          .pb(8)
          .children(
            sections.map((each, n) => (each.kind === "project" ? section(each, hidden, n, k) : canvases(each, model, hidden, k))),
          ),
      )
      .child(SidebarSections.new("sidebar.sections", {}));
    // The foot: a + that opens a folder or a repo as a project.
    const footer = h_flex()
      .h(HEAD)
      .flex_shrink_0()
      .px(HEAD_PAD)
      .items_center()
      .border_t_1()
      .border_color(k.sidebarRule)
      .child(
        Button.new(model.open)
          .h(ADD)
          .px(6)
          .flex()
          .items_center()
          .gap(6)
          .rounded(8)
          .text_color(k.textMuted)
          .hover((el) => el.bg(k.interactive))
          .on_click(() => press(model.open))
          .child(Icon.new(`${model.open}.glyph`, { name: "plus", size: 14, color: k.textMuted }))
          .child("New project"),
      );
    // While the sidebar reaches the window's top it is the title bar there:
    // the traffic lights, then its own toggle, over bare bar to drag by.
    const title =
      model.title == null
        ? null
        : div()
            .relative()
            .flex_shrink_0()
            .h(TITLE)
            .child(div().absolute().top(0).left(0).size_full().child(WindowDrag.new("sidebar.drag", {})))
            .child(
              h_flex()
                .absolute()
                .top(0)
                .left(TRAFFIC_LIGHTS)
                .h(TITLE)
                .items_center()
                .child(
                  Button.new(model.title.id)
                    .size(28)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(8)
                    .hover((el) => el.bg(k.interactive))
                    .on_click((_event, cx) => {
                      // The bar to drag the window by is under it: a second
                      // press must not reach it as a double click, which
                      // zooms the window.
                      if (cx) cx.stop_propagation();
                      press(model.title.id);
                    })
                    .child(Icon.new(`${model.title.id}.glyph`, {
                      control: model.title.id,
                      name: model.title.icon,
                      size: 14,
                      color: k.toolbarText,
                    })),
                ),
            );
    return v_flex()
      .id("sidebar")
      .size_full()
      .bg(k.sidebar)
      .border_r_1()
      .border_color(k.popupBorder)
      .text_color(k.text)
      .when(title != null, (el) => el.child(title))
      .child(list)
      .when(!hidden.has(model.open), (el) => el.child(footer));
  }
}
