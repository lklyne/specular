// A brush whose strokes are live web content: each watercolor item is
// `item.html`, which paints its stroke on a <canvas> with soft, layered
// edges, reading its props through the page bridge. The mod itself follows
// the `watercolor/brush` drag (`tool` key), previews it, places the stroke
// on release, and sends each item a plain display list: what the item shows
// where the mod is not installed, and what its snapshot keeps.
// `watercolor/color` (the dock button) moves the selected strokes, or the
// brush, to the next colour.
import { View, div } from "gpui-kit";
import { state, run, shared, setShared } from "specular";

const COLORS = ["#2f81f7", "#e5534b", "#57ab5a", "#c69026", "#986ee2"];
const SIZE = 18;
const STEP = 3;

function thin(points) {
  const kept = [];
  for (const point of points) {
    const last = kept[kept.length - 1];
    if (!last || Math.hypot(point[0] - last[0], point[1] - last[1]) >= STEP) kept.push(point);
  }
  return kept;
}

const brushColor = () => shared("watercolor/color") || COLORS[0];
const nextColor = (color) => COLORS[(COLORS.indexOf(color) + 1) % COLORS.length];
/** `#rrggbb` at about a third opacity, as a wash looks. */
const wash = (color) => `${color}55`;

function place(points, color) {
  const pad = SIZE;
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const x = Math.min(...xs) - pad;
  const y = Math.min(...ys) - pad;
  run("item.add", {
    kind: "mod",
    mod: "watercolor",
    x,
    y,
    width: Math.max(...xs) - x + pad,
    height: Math.max(...ys) - y + pad,
    props: { color, size: SIZE, points: points.map((p) => [p[0] - x, p[1] - y]) },
  });
}

const drawn = new Map();
let committed = null;
let lastCall = null;

function follow(stroke) {
  if (!stroke || stroke.tool !== "watercolor/brush") return;
  if (lastCall === null && stroke.phase === "up") committed = stroke.number;
  const points = thin(stroke.points);
  if (stroke.phase === "down") {
    run("tool.preview", { draws: [{ type: "path", points, stroke: wash(brushColor()), line: SIZE }] });
  } else if (committed !== stroke.number) {
    committed = stroke.number;
    if (points.length > 1) place(points, brushColor());
    else run("tool.preview", { draws: [] });
  }
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
    follow(state("tool"));
    const items = (state("canvas/mod_items") || []).filter((item) => item.mod === "watercolor");
    for (const item of items) {
      const key = JSON.stringify([item.width, item.height, item.props]);
      if (drawn.get(item.id) !== key) {
        drawn.set(item.id, key);
        const { points = [], color = COLORS[0], size = SIZE } = item.props || {};
        run("item.draw", { id: item.id, draws: [{ type: "path", points, stroke: wash(color), line: size }] });
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
