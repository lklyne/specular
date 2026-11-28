// The agent panel, drawn from `panels/chat`: the canvas's threads, the
// transcript of the one the panel has open and the panel's own composer
// (the `Composer` host component), acting with the `chat.*` commands. The
// buttons mods add for the open thread (`"chat": true` on a command, such
// as the thread mod's "Put this thread on the canvas") are in
// `panels/chat`'s `actions`.
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, run, notify, colors, Composer } from "specular";

function bar(id, text, k, onPress) {
  return Button.new(id)
    .h(26)
    .px(8)
    .flex()
    .items_center()
    .rounded(6)
    .text_size(12)
    .text_color(k.textMuted)
    .hover((el) => el.bg(k.hover))
    .on_click(onPress)
    .child(text);
}

/** Runs a command another mod adds, which is not there while it is off. */
function tryRun(name, args) {
  try {
    run(name, args);
  } catch (error) {
    notify(`${name}: ${error}`, "warning");
  }
}

function threadRows(list, isShown, onPick, prefix, k) {
  if (list.length === 0) {
    return [div().p(12).text_color(k.textMuted).child("No threads yet. Write below to start one.")];
  }
  return list.map((thread) =>
    Button.new(`${prefix}.thread.${thread.id}`)
      .px(10)
      .py(8)
      .flex()
      .flex_col()
      .gap(2)
      .rounded(6)
      .when(isShown(thread), (el) => el.bg(k.interactive))
      .hover((el) => el.bg(k.interactiveHover))
      .on_click(() => onPick(thread.id))
      .child(div().text_size(13).child(thread.title || "New thread"))
      .child(div().text_size(11).text_color(k.textMuted).child(thread.running ? "Working…" : thread.updatedAt)),
  );
}

function bubble(message, k) {
  const mine = message.role === "user";
  return div()
    .px(10)
    .py(8)
    .rounded(8)
    .text_size(13)
    .when(mine, (el) => el.bg(k.raisedFill).ml(32))
    .when(!mine, (el) => el.mr(16))
    .child(message.text);
}

function transcript(t, k) {
  const out = t.messages.map((message) => bubble(message, k));
  if (t.streaming) out.push(bubble({ role: "agent", text: t.streaming }, k));
  if (t.run) out.push(div().text_size(12).text_color(k.textMuted).child(t.run.label));
  if (t.error) out.push(div().text_size(12).text_color(k.error).child(t.error));
  for (const text of t.queued || []) out.push(div().text_size(12).text_color(k.textMuted).child(`Queued: ${text}`));
  if (out.length === 0 && t.emptyHint) out.push(div().p(12).text_color(k.textMuted).child(t.emptyHint));
  return out;
}

/** The right panel: the thread the panel has open, from `panels/chat`. */
function panelView(model) {
  return {
    prefix: "chat",
    title: model.title,
    back: model.canGoBack ? () => run("chat.back") : null,
    fresh: () => run("chat.new-thread"),
    stop: model.composer.running ? () => run("chat.stop") : null,
    actions: (model.actions || []).map((button) => ({
      title: button.title,
      press: () => tryRun(button.command, button.args),
    })),
    body: (k) =>
      model.transcript
        ? transcript(model.transcript, k)
        : threadRows(model.threads, (each) => each.active, (id) => run("chat.select", { id }), "chat", k),
    placeholder: model.composer.placeholder,
    thread: null,
  };
}

export default class Chat extends View {
  render() {
    const model = state("panels/chat");
    if (model == null) return div();
    const view = panelView(model);
    const k = colors();
    const head = h_flex()
      .h(36)
      .px(8)
      .gap(4)
      .items_center()
      .border_b_1()
      .border_color(k.sidebarRule)
      .when(view.back != null, (el) => el.child(bar(`${view.prefix}.back`, "‹ Threads", k, view.back)))
      .child(div().flex_1().text_size(13).font_weight(600).child(view.title))
      .children(view.actions.map((button, n) => bar(`${view.prefix}.action.${n}`, button.title, k, button.press)))
      .when(view.fresh != null, (el) => el.child(bar(`${view.prefix}.new`, "New", k, view.fresh)))
      .when(view.stop != null, (el) => el.child(bar(`${view.prefix}.stop`, "Stop", k, view.stop)));
    return v_flex()
      .id(view.prefix)
      .size_full()
      .bg(k.sidebar)
      .border_l_1()
      .border_color(k.sidebarRule)
      .text_color(k.text)
      .child(head)
      .child(v_flex().id(`${view.prefix}.body`).flex_1().p(8).gap(8).overflow_y_scroll().children(view.body(k)))
      .child(
        div()
          .p(8)
          .border_t_1()
          .border_color(k.sidebarRule)
          .child(
            // The panel's own composer: context pill, folder, model, Send
            // and pasted images, sending to the open thread.
            Composer.new(`${view.prefix}.composer`, {
              id: `${view.prefix}.composer`,
              thread: view.thread,
              placeholder: view.placeholder,
            }),
          ),
      );
  }
}
