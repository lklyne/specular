// The right panel, drawn from `panels/chat`: the canvas's agent threads,
// the transcript of the one showing and a composer. Return sends what was
// typed with `chat.send`; the rest are `chat.*` commands.
//
// A layout gives the panel its width and place like any block:
//   { "id": "chat", "mod": "chat", "size": { "px": 400 } }
import { View, div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
import { state, run, shared, colors, Field } from "specular";

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

function threads(model, k) {
  if (model.threads.length === 0) {
    return [div().p(12).text_color(k.textMuted).child("No threads yet. Write below to start one.")];
  }
  return model.threads.map((thread) =>
    Button.new(`chat.thread.${thread.id}`)
      .px(10)
      .py(8)
      .flex()
      .flex_col()
      .gap(2)
      .rounded(6)
      .when(thread.active, (el) => el.bg(k.interactive))
      .hover((el) => el.bg(k.interactiveHover))
      .on_click(() => run("chat.select", { id: thread.id }))
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
  if (out.length === 0 && t.emptyHint) out.push(div().p(12).text_color(k.textMuted).child(t.emptyHint));
  return out;
}

export default class Chat extends View {
  init() {
    this.field = "chat.composer";
    this.sent = shared(`field/${this.field}/submit`) || 0;
  }

  render() {
    const model = state("panels/chat");
    const k = colors();
    if (model == null) return div();
    const submit = shared(`field/${this.field}/submit`) || 0;
    if (submit !== this.sent) {
      this.sent = submit;
      const text = (shared(`field/${this.field}`) || "").trim();
      if (text || model.composer.canSendEmpty) run("chat.send", { text });
    }
    const head = h_flex()
      .h(36)
      .px(8)
      .gap(4)
      .items_center()
      .border_b_1()
      .border_color(k.sidebarRule)
      .when(model.canGoBack, (el) => el.child(bar("chat.back", "‹ Threads", k, () => run("chat.back"))))
      .child(div().flex_1().text_size(13).font_weight(600).child(model.title))
      .child(bar("chat.new", "New", k, () => run("chat.new-thread")))
      .when(model.composer.running, (el) => el.child(bar("chat.stop", "Stop", k, () => run("chat.stop"))));
    const body = model.transcript ? transcript(model.transcript, k) : threads(model, k);
    return v_flex()
      .id("chat")
      .size_full()
      .bg(k.sidebar)
      .border_l_1()
      .border_color(k.sidebarRule)
      .text_color(k.text)
      .child(head)
      .child(v_flex().id("chat.body").flex_1().p(8).gap(8).overflow_y_scroll().children(body))
      .child(
        v_flex()
          .p(8)
          .gap(6)
          .border_t_1()
          .border_color(k.sidebarRule)
          .child(Field.new("chat.composer", { id: this.field, placeholder: model.composer.placeholder }))
          .child(div().text_size(11).text_color(k.textMuted).child(model.composer.model.label)),
      );
  }
}
