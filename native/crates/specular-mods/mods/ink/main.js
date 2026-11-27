// A pen tool and the strokes it leaves, both drawn from display lists. While
// `ink/pen` is in hand the drag arrives as the `tool` key; the mod shows the
// stroke so far with `tool.preview` and, on release, places it as an ink
// item (`item.add`, one undo step) whose props are its points. Each ink item
// is drawn again only when its props or size change. `ink/color` (the dock
// button while the pen or an ink stroke is selected) moves the selected
// strokes, or the pen, to the next colour.
import { View, div } from "gpui-kit";
import { state, run, shared, setShared } from "specular";

const COLORS = ["#1f2328", "#cf222e", "#0969da", "#1a7f37", "#8250df"];
const LINE = 3;
/** The closest two kept points are, in canvas units. */
const STEP = 2;

function thin(points) {
  const kept = [];
  for (const point of points) {
    const last = kept[kept.length - 1];
    if (!last || Math.hypot(point[0] - last[0], point[1] - last[1]) >= STEP) kept.push(point);
  }
  return kept;
}

function penColor() {
  return shared("ink/color") || COLORS[0];
}

function nextColor(color) {
  return COLORS[(COLORS.indexOf(color) + 1) % COLORS.length];
}

/** The stroke `points` as an ink item at its bounds, padded by its line. */
function place(points, color) {
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const x = Math.min(...xs) - LINE;
  const y = Math.min(...ys) - LINE;
  const width = Math.max(...xs) - x + LINE;
  const height = Math.max(...ys) - y + LINE;
  run("item.add", {
    kind: "mod",
    mod: "ink",
    x,
    y,
    width: Math.max(width, 1),
    height: Math.max(height, 1),
    props: { color, line: LINE, points: points.map((p) => [p[0] - x, p[1] - y]) },
  });
}

const drawn = new Map();
let committed = null;
let lastCall = null;

function follow(stroke) {
  if (!stroke || stroke.tool !== "ink/pen") return;
  // A stroke that ended before this script loaded (a reload) was placed.
  if (lastCall === null && stroke.phase === "up") committed = stroke.number;
  const points = thin(stroke.points);
  if (stroke.phase === "down") {
    run("tool.preview", { draws: [{ type: "path", points, stroke: penColor(), line: LINE }] });
  } else if (committed !== stroke.number) {
    committed = stroke.number;
    if (points.length > 1) place(points, penColor());
    else run("tool.preview", { draws: [] });
  }
}

function recolor(items, ids) {
  if (ids.length === 0) {
    setShared("ink/color", nextColor(penColor()));
    return;
  }
  for (const item of items.filter((item) => ids.includes(item.id))) {
    run("item.set", { id: item.id, props: { color: nextColor(item.props.color) } });
  }
}

export default class Ink extends View {
  render() {
    follow(state("tool"));
    const items = (state("canvas/mod_items") || []).filter((item) => item.mod === "ink");
    for (const item of items) {
      const key = JSON.stringify([item.width, item.height, item.props]);
      if (drawn.get(item.id) !== key) {
        drawn.set(item.id, key);
        const { points = [], color = COLORS[0], line = LINE } = item.props || {};
        run("item.draw", { id: item.id, draws: [{ type: "path", points, stroke: color, line }] });
      }
    }
    const call = state("call");
    const number = call ? call.number : 0;
    if (lastCall !== null && number > lastCall && call.command === "ink/color") {
      recolor(items, (call.args && call.args.ids) || []);
    }
    lastCall = number;
    return div();
  }
}
