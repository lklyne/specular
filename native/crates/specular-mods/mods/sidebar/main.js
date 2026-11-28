// The sidebar, drawn from `panels/sidebar`: a section for each project
// (ADR 0062), a plain folder or a repo, with its canvases under it; then
// the section mods (`"section": "sidebar"`), such as Notes and Pages, where
// `SidebarSections` is placed. Each section folds at its head. Under the
// pointer a project's head shows a ⋯ for its details (its folder, whether a
// repo's canvases go into git, and the sites bound to it), a + for a new
// canvas in it and, at the far right, its fold chevron. A footer has a +
// that opens a folder or a repo as a project. A click presses a row by its id, as the app's own sidebar does;
// a double click on a canvas asks for its new name in the `prompt` dialog.
//
// A layout hides a section or a row by listing its id in this mod's props:
//   { "mod": "sidebar", "props": { "hidden": ["sidebar.open"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, openOverlay, Icon, SidebarSections, WindowDrag } from "specular";

const ROW = 26;
const HOVERED = "project-head";
const TITLE = 38;
const TRAFFIC_LIGHTS = 86;

const press = (id) => run("control.press", { id });

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

function canvasRow(row, k) {
  return Button.new(row.id)
    .h(ROW)
    .px(8)
    .flex()
    .items_center()
    .justify_between()
    .rounded(6)
    .when(row.active, (el) => el.bg(k.interactive))
    .hover((el) => el.bg(k.interactiveHover))
    .on_click((event) => (event.click_count >= 2 ? rename(row) : press(row.id)))
    .child(div().truncate().child(row.label))
    .child(div().text_color(k.textMuted).text_size(11).child(String(row.count)));
}

function bindSite(section) {
  openOverlay("prompt", {
    id: "sidebar.bind",
    props: {
      title: `Bind a site to ${section.title}`,
      value: "http://localhost:3000",
      confirm: "Bind",
      command: "project.bind",
      args: { project: section.project },
      key: "origin",
    },
  });
}

function small(id, text, k, onPress, on) {
  return Button.new(id)
    .h(20)
    .px(6)
    .flex()
    .items_center()
    .rounded(4)
    .text_size(11)
    .text_color(on ? k.text : k.textMuted)
    .when(on, (el) => el.bg(k.interactive))
    .hover((el) => el.bg(k.interactiveHover))
    .on_click((event, cx) => {
      // The project head under it has a press of its own.
      if (cx) cx.stop_propagation();
      onPress();
    })
    .child(text);
}

function details(section, k) {
  const out = v_flex().gap(4).px(8).py(6).rounded(6).bg(k.interactive).text_size(11).text_color(k.textMuted);
  out.child(div().truncate().child(section.folder));
  const actions = h_flex().gap(2).child(small(section.reveal, "Show in Finder", k, () => press(section.reveal)));
  if (section.close != null) actions.child(small(section.close, "Close project", k, () => press(section.close)));
  out.child(actions);
  if (section.git != null) {
    const choice = section.git.choice;
    out.child(
      h_flex()
        .gap(2)
        .items_center()
        .child(div().flex_1().child(choice == null ? "Canvases in git? Not set" : "Canvases in git"))
        .child(small(section.git.commit, "Commit", k, () => press(section.git.commit), choice === "commit"))
        .child(small(section.git.ignore, "Ignore", k, () => press(section.git.ignore), choice === "ignore")),
    );
  }
  for (const site of section.origins) {
    out.child(
      h_flex()
        .gap(2)
        .items_center()
        .child(div().flex_1().truncate().child(site.origin))
        .child(small(site.auto, "Auto", k, () => press(site.auto), site.autoFix))
        .child(small(site.remove, "×", k, () => press(site.remove))),
    );
  }
  out.child(small(`${section.id}.bind`, "+ Bind a site…", k, () => bindSite(section)));
  return out;
}

// Shown only while the pointer is over the project's head, unless `on`.
const onHover = (el, on) => (on ? el : el.invisible().group_hover(HOVERED, (el) => el.visible()));

function projectHead(section, k) {
  return Button.new(section.id)
    .group(HOVERED)
    .h(ROW)
    .px(8)
    .flex()
    .items_center()
    .gap(2)
    .rounded(6)
    .text_size(11)
    .text_color(section.active ? k.text : k.textMuted)
    .on_click(() => press(section.id))
    .child(
      h_flex()
        .flex_1()
        .min_w_0()
        .gap(6)
        .child(div().truncate().child(section.title))
        .when(section.branch != null, (el) => el.child(div().text_color(k.textMuted).truncate().child(section.branch))),
    )
    .child(onHover(small(section.details, "⋯", k, () => press(section.details), section.detailsShown), section.detailsShown))
    .child(onHover(small(section.add, "+", k, () => press(section.add)), false))
    .child(
      onHover(
        div()
          .w(20)
          .h(20)
          .flex()
          .items_center()
          .justify_center()
          .rounded(4)
          .hover((el) => el.bg(k.interactiveHover))
          .child(Icon.new(`${section.id}.chevron`, {
            name: section.folded ? "chevron-right" : "chevron-down",
            size: 12,
            color: k.textMuted,
          })),
        false,
      ),
    );
}

function section(section, hidden, k) {
  const out = v_flex().gap(1).child(projectHead(section, k));
  if (section.detailsShown) out.child(details(section, k));
  return out.children(section.rows.filter((row) => !hidden.has(row.id)).map((row) => canvasRow(row, k)));
}

function canvases(section, hidden, k) {
  const head = Button.new(section.id)
    .h(ROW)
    .px(8)
    .flex()
    .items_center()
    .justify_between()
    .rounded(6)
    .text_size(11)
    .text_color(k.textMuted)
    .hover((el) => el.bg(k.interactiveHover))
    .on_click(() => press(section.id))
    .child(section.title.toUpperCase())
    .child(section.folded ? "+" : "−");
  const rows = section.folded ? [] : section.rows.filter((row) => !hidden.has(row.id)).map((row) => canvasRow(row, k));
  return v_flex().gap(1).child(head).children(rows);
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
      .p(8)
      .gap(16)
      .overflow_y_scroll()
      .children(sections.map((each) => (each.kind === "project" ? section(each, hidden, k) : canvases(each, hidden, k))))
      .child(SidebarSections.new("sidebar.sections", {}));
    // The foot: a + that opens a folder or a repo as a project.
    const footer = h_flex()
      .flex_shrink_0()
      .p(8)
      .border_t_1()
      .border_color(k.sidebarRule)
      .child(
        Button.new(model.open)
          .h(ROW)
          .px(8)
          .flex()
          .items_center()
          .gap(6)
          .rounded(6)
          .text_color(k.textMuted)
          .hover((el) => el.bg(k.interactiveHover))
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
                    .hover((el) => el.bg(k.interactiveHover))
                    .on_click(() => press(model.title.id))
                    .child(Icon.new(`${model.title.id}.glyph`, {
                      control: model.title.id,
                      name: model.title.icon,
                      size: 16,
                      color: k.toolbarText,
                    })),
                ),
            );
    return v_flex()
      .id("sidebar")
      .size_full()
      .bg(k.sidebar)
      .border_r_1()
      .border_color(k.sidebarRule)
      .text_size(13)
      .text_color(k.text)
      .when(title != null, (el) => el.child(title))
      .child(list)
      .when(!hidden.has(model.open), (el) => el.child(footer));
  }
}
