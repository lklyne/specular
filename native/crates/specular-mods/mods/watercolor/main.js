// A brush the app draws while it is in hand: `stroke` in mod.json is the
// brush, three tapered washes that thin where the pointer moves fast, drawn
// over the live stroke every frame with no word from this script. The
// script acts on release: it places the stroke as a watercolor item
// (`item.add`, one undo step) and draws each item with the same washes.
// `watercolor/color` (the dock button) moves the selected strokes, or the
// brush, to the next colour, the brush through `tool.stroke`.
import { View, div } from "gpui-kit";
import { state, run, shared, setShared } from "specular";

const COLORS = ["#2f81f7", "#e5534b", "#57ab5a", "#c69026", "#986ee2"];
/** The washes, widest and faintest first: [alpha, line, speed]. */
const WASHES = [["2e", 28, 0], ["40", 18, 0.5], ["55", 8, 0.8]];
const PAD = 14;

const brushColor = () => shared("watercolor/color") || COLORS[0];
const nextColor = (color) => COLORS[(COLORS.indexOf(color) + 1) % COLORS.length];

/** The washes in `color` along `points`. */
function washes(color, points = []) {
  return WASHES.map(([alpha, line, speed]) =>
    ({ type: "path", points, stroke: color + alpha, line, taper: true, speed }));
}

function place(points, color) {
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const x = Math.min(...xs) - PAD;
  const y = Math.min(...ys) - PAD;
  run("item.add", {
    kind: "mod",
    mod: "watercolor",
    x,
    y,
    width: Math.max(...xs) - x + PAD,
    height: Math.max(...ys) - y + PAD,
    props: { color, points: points.map((p) => [p[0] - x, p[1] - y]) },
  });
}

const drawn = new Map();
let brushed = null;
let committed = null;
let lastCall = null;

function follow(stroke) {
  if (!stroke || stroke.tool !== "watercolor/brush" || stroke.phase !== "up") return;
  // A stroke that ended before this script loaded (a reload) was placed.
  if (lastCall === null) committed = stroke.number;
  if (committed === stroke.number) return;
  committed = stroke.number;
  if (stroke.points.length > 1) place(stroke.points, brushColor());
  else run("tool.preview", { draws: [] });
}

function recolor(items, ids) {
  if (ids.length === 0) {
    setShared("watercolor/color", nextColor(brushColor()));
    return;
  }
  for (const item of items.filter((item) => ids.includes(item.id))) {
    run("item.set", { id: item.id, props: { color: nextColor(item.props.color) } });
  }
}

export default class Watercolor extends View {
  render() {
    if (brushed !== brushColor()) {
      brushed = brushColor();
      run("tool.stroke", { tool: "watercolor/brush", draws: washes(brushed) });
    }
    follow(state("tool"));
    const items = (state("canvas/mod_items") || []).filter((item) => item.mod === "watercolor");
    for (const item of items) {
      const key = JSON.stringify([item.width, item.height, item.props]);
      if (drawn.get(item.id) !== key) {
        drawn.set(item.id, key);
        const { points = [], color = COLORS[0] } = item.props || {};
        run("item.draw", { id: item.id, draws: washes(color, points) });
      }
    }
    const call = state("call");
    const number = call ? call.number : 0;
    if (lastCall !== null && number > lastCall && call.command === "watercolor/color") {
      recolor(items, (call.args && call.args.ids) || []);
    }
    lastCall = number;
    return div();
  }
}
