// The sidebar, drawn from `panels/sidebar`: a section for each project
// (ADR 0062), a plain folder or a repo, with its canvases under it; then
// the active canvas's notes and pages. Each section folds at its head. A
// project's head has a + for a new canvas in it and a ⋯ for its details:
// its folder, whether a repo's canvases go into git, and the sites bound
// to it. A click presses a row by its id, as the app's own sidebar does;
// a double click on a canvas asks for its new name in the `prompt` dialog.
//
// A layout hides a section or a row by listing its id in this mod's props:
//   { "mod": "sidebar", "props": { "hidden": ["sidebar.head.notes"] } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, openOverlay, Icon } from "specular";

const ROW = 26;
const INDENT = 14;

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

function itemRow(row, depth, hidden, k) {
  const out = [
    Button.new(row.id)
      .h(ROW)
      .pl(8 + depth * INDENT)
      .pr(8)
      .flex()
      .items_center()
      .gap(6)
      .rounded(6)
      .when(row.selected, (el) => el.bg(k.interactive))
      .when(row.dimmed, (el) => el.opacity(0.5))
      .hover((el) => el.bg(k.interactiveHover))
      .on_click(() => press(row.id))
      .child(Icon.new(`${row.id}.glyph`, { name: row.icon, size: 14, color: k.textMuted }))
      .child(div().flex_1().truncate().child(row.label))
      .when(row.trailing != null, (el) => el.child(div().text_color(k.textMuted).text_size(11).child(row.trailing))),
  ];
  if (row.expanded) {
    for (const child of row.children) {
      if (!hidden.has(child.id)) out.push(...itemRow(child, depth + 1, hidden, k));
    }
  }
  return out;
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
    .on_click(onPress)
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

function projectHead(section, k) {
  return h_flex()
    .h(ROW)
    .items_center()
    .gap(2)
    .child(
      Button.new(section.id)
        .flex_1()
        .h(ROW)
        .px(8)
        .flex()
        .items_center()
        .gap(6)
        .rounded(6)
        .text_size(11)
        .text_color(section.active ? k.text : k.textMuted)
        .hover((el) => el.bg(k.interactiveHover))
        .on_click(() => press(section.id))
        .child(div().truncate().child(section.title))
        .when(section.branch != null, (el) => el.child(div().text_color(k.textMuted).truncate().child(section.branch))),
    )
    .child(small(section.details, "⋯", k, () => press(section.details), section.detailsShown))
    .child(small(section.add, "+", k, () => press(section.add)));
}

function section(section, hidden, k) {
  if (section.kind === "project") {
    const out = v_flex().gap(1).child(projectHead(section, k));
    if (section.detailsShown) out.child(details(section, k));
    return out.children(section.rows.filter((row) => !hidden.has(row.id)).map((row) => canvasRow(row, k)));
  }
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
  const rows = section.folded
    ? []
    : section.rows
        .filter((row) => !hidden.has(row.id))
        .flatMap((row) => (row.canvas != null ? [canvasRow(row, k)] : itemRow(row, 0, hidden, k)));
  return v_flex().gap(1).child(head).children(rows);
}

export default class Sidebar extends View {
  render() {
    const model = state("panels/sidebar");
    const hidden = new Set((props() || {}).hidden || []);
    const k = colors();
    if (model == null) return div();
    const sections = model.sections.filter((each) => !hidden.has(each.id));
    return v_flex()
      .id("sidebar")
      .size_full()
      .p(8)
      .gap(12)
      .bg(k.sidebar)
      .border_r_1()
      .border_color(k.sidebarRule)
      .text_size(13)
      .text_color(k.text)
      .overflow_y_scroll()
      .children(sections.map((each) => section(each, hidden, k)))
      .when(!hidden.has(model.open), (el) =>
        el.child(
          Button.new(model.open)
            .h(ROW)
            .px(8)
            .flex()
            .items_center()
            .rounded(6)
            .text_color(k.textMuted)
            .hover((el) => el.bg(k.interactiveHover))
            .on_click(() => press(model.open))
            .child("+ Open project…"),
        ),
      );
  }
}
