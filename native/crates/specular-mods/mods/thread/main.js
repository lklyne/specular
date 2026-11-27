// A chat thread as a canvas item: each `thread` item shows one agent
// thread, `props.thread`, live in `item.html` (its transcript, the run and
// a composer). This script runs out of sight. It answers the commands:
//
// - `thread/new` starts a thread beside the panel's (`chat.new-thread` with
//   an id it picks and `open: false`) and places a block showing it.
// - `thread/place {thread?}` places a block for a thread, the panel's open
//   one by default, or a new one when none is open.
// - `thread/panel` (the dock button) opens the selected block's thread in
//   the agent panel.
//
// It also sends each block a display list: the title and the last words,
// which the .canvas file keeps as the item's snapshot and which shows
// where this mod is not installed. It is sent again when a message lands,
// not on every word streamed.
import { View, div } from "gpui-kit";
import { state, run } from "specular";

const INK = "#1f2328";
const MUTED = "#6e7781";
const CARD = "#ffffff";
const RIM = "#d0d7de";
const PAD = 14;
const LAST = 4;

/** An id for a thread nobody has, picked here so it is known at once. */
function freshThreadId() {
  const time = Date.now().toString(36);
  const salt = Math.floor(Math.random() * 1e8).toString(36);
  return `thread_${time}${salt}`;
}

function clip(text, most) {
  const flat = String(text || "").replace(/\s+/g, " ").trim();
  return flat.length > most ? `${flat.slice(0, most - 1)}…` : flat;
}

/** The snapshot of a block: its frame, the thread's title, its last words. */
function draw(item, thread) {
  const { width, height } = item;
  const inner = width - 2 * PAD;
  const draws = [
    { type: "rect", x: 0, y: 0, width, height, fill: CARD, stroke: RIM, line: 1, radius: 12 },
    { type: "text", x: PAD, y: PAD, text: (thread && thread.title) || "Chat thread", size: 15, color: INK, width: inner },
  ];
  if (!thread) {
    draws.push({ type: "text", x: PAD, y: PAD + 30, text: "No thread here yet.", size: 12, color: MUTED, width: inner });
    return draws;
  }
  let y = PAD + 34;
  for (const message of thread.messages.slice(-LAST)) {
    if (y > height - 40) break;
    const who = message.role === "user" ? "You" : "Agent";
    draws.push({ type: "text", x: PAD, y, text: who, size: 11, color: MUTED, width: inner });
    draws.push({ type: "text", x: PAD, y: y + 16, text: clip(message.text, 180), size: 12, color: INK, width: inner });
    y += 76;
  }
  return draws;
}

function place(thread) {
  run("item.add", { kind: "mod", mod: "thread", props: { thread } });
}

function startAndPlace() {
  const id = freshThreadId();
  run("chat.new-thread", { id, open: false });
  place(id);
}

function act(call, items) {
  const args = call.args || {};
  if (call.command === "thread/new") {
    startAndPlace();
  } else if (call.command === "thread/place") {
    const open = (state("chat/threads") || {}).open;
    const thread = args.thread || open;
    if (thread) place(thread);
    else startAndPlace();
  } else if (call.command === "thread/panel") {
    const ids = args.ids || [];
    const item = items.find((each) => ids.includes(each.id));
    const thread = item && item.props && item.props.thread;
    if (!thread) return;
    run("chat.select", { id: thread });
    if (!(state("settings") || {}).showChat) run("chat.toggle");
  }
}

const drawn = new Map();
let lastCall = null;

export default class ThreadBlocks extends View {
  render() {
    const items = (state("canvas/mod_items") || []).filter((item) => item.mod === "thread");
    for (const item of items) {
      const id = item.props && item.props.thread;
      const thread = id ? state(`chat/thread/${id}`) : null;
      const said = thread ? thread.messages.map((m) => [m.id, m.text]).slice(-LAST) : null;
      const key = JSON.stringify([item.width, item.height, id, thread && thread.title, said]);
      if (drawn.get(item.id) === key) continue;
      drawn.set(item.id, key);
      run("item.draw", { id: item.id, draws: draw(item, thread) });
    }
    const call = state("call");
    const number = call ? call.number : 0;
    if (lastCall !== null && number > lastCall) act(call, items);
    lastCall = number;
    return div();
  }
}
