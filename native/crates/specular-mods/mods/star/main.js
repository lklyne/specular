// A star shape. `star/star` sits among the shape tool's shapes
// (`"group": "shape"`): a drag draws a star in the box it spans (a click
// places one at the size `items.size` gives), previewed with
// `tool.preview` and placed with `item.add` as one undo step. Each star is
// drawn from its props, `points` and `color`, again only when they or its
// size change. `star/points` (a dock button while a star is selected) gives
// the selected stars one more point, from 4 up to 8 and round again.
import { View, div } from "gpui-kit";
import { state, run } from "specular";

const COLOR = "#f5b301";
const EDGE = "#b07d00";
const SIZE = 120;

/** The star's outline in a `width` by `height` box: `points` tips. */
function outline(width, height, points) {
  const out = [];
  const [cx, cy] = [width / 2, height / 2];
  for (let i = 0; i < points * 2; i += 1) {
    const r = i % 2 === 0 ? 1 : 0.45;
    const a = (Math.PI * i) / points - Math.PI / 2;
    out.push([cx + Math.cos(a) * r * cx, cy + Math.sin(a) * r * cy]);
  }
  return out;
}

function star(x, y, width, height, points, color) {
  const shape = outline(width, height, points).map(([px, py]) => [x + px, y + py]);
  return { type: "path", points: shape, closed: true, fill: color, stroke: EDGE, line: 1.5 };
}

/** The box a drag from `a` to `b` spans; a click is a star of the default size. */
function box(a, b) {
  const [w, h] = [Math.abs(b[0] - a[0]), Math.abs(b[1] - a[1])];
  if (w < 4 && h < 4) return [a[0] - SIZE / 2, a[1] - SIZE / 2, SIZE, SIZE];
  return [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.max(w, 8), Math.max(h, 8)];
}

const drawn = new Map();
let placed = null;
let lastCall = null;

function follow(stroke) {
  if (!stroke || stroke.tool !== "star/star") return;
  if (lastCall === null && stroke.phase === "up") placed = stroke.number;
  const first = stroke.points[0];
  const last = stroke.points[stroke.points.length - 1];
  const [x, y, width, height] = box(first, last);
  if (stroke.phase === "down") {
    run("tool.preview", { draws: [star(x, y, width, height, 5, COLOR)] });
  } else if (placed !== stroke.number) {
    placed = stroke.number;
    run("item.add", { kind: "mod", mod: "star", x, y, width, height, props: { points: 5, color: COLOR } });
  }
}

export default class Star extends View {
  render() {
    follow(state("tool"));
    const items = (state("canvas/mod_items") || []).filter((item) => item.mod === "star");
    for (const item of items) {
      const key = JSON.stringify([item.width, item.height, item.props]);
      if (drawn.get(item.id) === key) continue;
      drawn.set(item.id, key);
      const { points = 5, color = COLOR } = item.props || {};
      run("item.draw", { id: item.id, draws: [star(0, 0, item.width, item.height, points, color)] });
    }
    const call = state("call");
    const number = call ? call.number : 0;
    if (lastCall !== null && number > lastCall && call.command === "star/points") {
      for (const id of (call.args && call.args.ids) || []) {
        const item = items.find((each) => each.id === id);
        const points = item ? ((item.props.points || 5) % 8) + 1 : 5;
        run("item.set", { id, props: { points: Math.max(points, 4) } });
      }
    }
    lastCall = number;
    return div();
  }
}
