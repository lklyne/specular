// The agent panel. Without props it is the right panel, drawn from
// `panels/chat`: the canvas's threads, the transcript of the one the panel
// has open and a composer, acting with the `chat.*` commands.
//
// With `props.pane` it is a pane of its own, beside the panel and other
// panes: it keeps the thread it shows in `shared("chat/pane/<pane>")`,
// lists threads from `chat/threads`, draws the thread from
// `chat/thread/<id>` and sends with `chat.send {thread}`, so several panes
// show several threads side by side. Each draws its composer with the
// `Composer` host component, the panel's own. `props.thread` pins a pane to one
// thread. The `chats` layout puts two panes beside the canvas:
//   { "id": "chat.a", "type": "block", "content": { "mod": "chat", "props": { "pane": "a" } } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, props, run, shared, setShared, notify, colors, Composer } from "specular";

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

/** Threads this mod started, which read as null until the app has them. */
const started = new Set();

/** Each pane's id for the thread its composer starts, before it has one. */
const pending = new Map();

function freshThreadId() {
  const id = `thread_${Date.now().toString(36)}${Math.floor(Math.random() * 1e8).toString(36)}`;
  started.add(id);
  return id;
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
  const thread = model.threads.find((each) => each.active);
  return {
    prefix: "chat",
    title: model.title,
    back: model.canGoBack ? () => run("chat.back") : null,
    fresh: () => run("chat.new-thread"),
    stop: model.composer.running ? () => run("chat.stop") : null,
    place: thread ? () => tryRun("thread/place", { thread: thread.id }) : null,
    body: (k) =>
      model.transcript
        ? transcript(model.transcript, k)
        : threadRows(model.threads, (each) => each.active, (id) => run("chat.select", { id }), "chat", k),
    placeholder: model.composer.placeholder,
    thread: null,
  };
}

/** A pane: a thread of its own, from `chat/threads` and `chat/thread/<id>`. */
function paneView(pane, pinned) {
  const slot = `chat/pane/${pane}`;
  const list = state("chat/threads") || { threads: [], model: { label: "" } };
  const chosen = pinned || shared(slot);
  const thread = chosen ? state(`chat/thread/${chosen}`) : null;
  // A thread closed, or left on another canvas, gives the list back.
  const id = thread || pinned || started.has(chosen) ? chosen : null;
  const choose = (next) => setShared(slot, next);
  // With no thread the composer sends to a fresh id, which `chat.send`
  // starts; once the app lists it the pane shows it.
  let next = null;
  if (!id) {
    next = pending.get(pane) || freshThreadId();
    pending.set(pane, next);
    if (list.threads.some((each) => each.id === next)) {
      pending.delete(pane);
      choose(next);
    }
  }
  const start = () => {
    const fresh = freshThreadId();
    run("chat.new-thread", { id: fresh, open: false });
    choose(fresh);
    return fresh;
  };
  return {
    prefix: `chat.${pane}`,
    title: id ? (thread && thread.title) || "New thread" : "Threads",
    back: id && !pinned ? () => choose(null) : null,
    fresh: pinned ? null : start,
    stop: thread && thread.running ? () => run("chat.stop", { thread: id }) : null,
    place: id ? () => tryRun("thread/place", { thread: id }) : null,
    body: (k) => {
      if (thread) return transcript(thread, k);
      if (id) return [div().p(12).text_color(k.textMuted).child("A new thread. Write below to start it.")];
      return threadRows(list.threads, () => false, choose, `chat.${pane}`, k);
    },
    placeholder: (thread && thread.placeholder) || "Write to the agent…",
    thread: id || next,
  };
}

export default class Chat extends View {
  render() {
    const given = props() || {};
    const pane = given.pane || (given.thread ? `thread.${given.thread}` : null);
    const model = pane ? null : state("panels/chat");
    if (!pane && model == null) return div();
    const view = pane ? paneView(pane, given.thread) : panelView(model);
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
      .when(view.place != null, (el) => el.child(bar(`${view.prefix}.place`, "To canvas", k, view.place)))
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
            // and pasted images, sending to the pane's thread.
            Composer.new(`${view.prefix}.composer`, {
              id: `${view.prefix}.composer`,
              thread: view.thread,
              placeholder: view.placeholder,
            }),
          ),
      );
  }
}
