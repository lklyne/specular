// A project's details as a card, opened as a popup overlay beside its head
// in the sidebar (a right press there):
//   openOverlay("project", { kind: "popup", at: [x, y], width: 300,
//     props: { project: "<project id>" } })
// Drawn from the project's section of `panels/sidebar`: its name, its
// folder, Show in Finder and Close project, whether a repo's canvases go
// into git, the sites bound to it, and Bind a site. A press outside it or
// Escape closes it.
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, colors, openOverlay, closeOverlay } from "specular";

const FOLDER = 44;

const press = (id) => run("control.press", { id });

// `text` with its middle cut out when it is longer than `most` characters,
// so a path keeps its root and its folder's name.
function middle(text, most) {
  if (text.length <= most) return text;
  const half = Math.floor((most - 1) / 2);
  return `${text.slice(0, half)}…${text.slice(text.length - half)}`;
}

function button(id, text, k, onPress, on) {
  return Button.new(id)
    .h(22)
    .px(8)
    .flex()
    .items_center()
    .rounded(5)
    .text_size(12)
    .text_color(on ? k.text : k.textMuted)
    .when(on, (el) => el.bg(k.interactive))
    .hover((el) => el.bg(k.interactiveHover))
    .on_click(onPress)
    .child(text);
}

function bindSite(section, overlay) {
  closeOverlay(overlay);
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

const rule = (k) => div().h(1).bg(k.divider);

export default class Project extends View {
  render() {
    const p = props() || {};
    const k = colors();
    const sidebar = state("panels/sidebar");
    const section = ((sidebar && sidebar.sections) || []).find(
      (each) => each.kind === "project" && each.project === p.project,
    );
    if (section == null) return div();
    const out = v_flex().id("project").p(12).gap(8).text_size(12).text_color(k.text);
    out.child(
      v_flex()
        .gap(2)
        .child(div().text_size(13).font_weight(600).truncate().child(section.title))
        .child(div().text_color(k.textMuted).whitespace_nowrap().child(middle(section.folder, FOLDER))),
    );
    const folder = h_flex().gap(2).child(button(section.reveal, "Show in Finder", k, () => press(section.reveal)));
    if (section.close != null) {
      folder.child(
        button(section.close, "Close project", k, () => {
          closeOverlay(p.overlay);
          press(section.close);
        }),
      );
    }
    out.child(folder);
    if (section.git != null) {
      const choice = section.git.choice;
      out.child(rule(k)).child(
        h_flex()
          .gap(2)
          .items_center()
          .child(div().flex_1().text_color(k.textMuted).child(choice == null ? "Canvases in git? Not set" : "Canvases in git"))
          .child(button(section.git.commit, "Commit", k, () => press(section.git.commit), choice === "commit"))
          .child(button(section.git.ignore, "Ignore", k, () => press(section.git.ignore), choice === "ignore")),
      );
    }
    out.child(rule(k));
    for (const site of section.origins) {
      out.child(
        h_flex()
          .gap(2)
          .items_center()
          .child(div().flex_1().min_w(0).truncate().child(site.origin))
          .child(button(site.auto, "Auto", k, () => press(site.auto), site.autoFix))
          .child(button(site.remove, "×", k, () => press(site.remove))),
      );
    }
    out.child(
      h_flex().child(button(`${section.id}.bind`, "Bind a site…", k, () => bindSite(section, p.overlay))),
    );
    return out;
  }
}
