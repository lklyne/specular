// A star shape. `star/star` is one of the shape tool's shapes: the app
// draws it, takes the palette's colours, the fill and border styles and a
// label, and themes it, as it does a hexagon. Only the outline is ours, in
// `mod.json` for a new star and from `outline()` after that.
//
// `star/points` (a dock button beside the selected stars' style) gives them
// one more point, from 4 up to 8 and round again, with `shape.set`: one
// undo step, the new count kept in the shapes' `params`.
import { View, div } from "gpui-kit";
import { state, run } from "specular";

/** The outline of a star with `points` tips, as SVG path data in the 0 to 100 box. */
function outline(points) {
  const corners = [];
  for (let i = 0; i < points * 2; i += 1) {
    const r = i % 2 === 0 ? 50 : 22.5;
    const a = (Math.PI * i) / points - Math.PI / 2;
    corners.push(`${(50 + Math.cos(a) * r).toFixed(2)},${(50 + Math.sin(a) * r).toFixed(2)}`);
  }
  return `M${corners.join(" L")} Z`;
}

let lastCall = null;

export default class Star extends View {
  render() {
    const call = state("call");
    const number = call ? call.number : 0;
    if (lastCall !== null && number > lastCall && call.command === "star/points") {
      const ids = (call.args && call.args.ids) || [];
      const items = state("canvas/items") || [];
      const first = items.find((item) => ids.includes(item.id) && item.custom);
      const now = (first && first.custom.params.points) || 5;
      const points = Math.max((now % 8) + 1, 4);
      run("shape.set", { ids, params: { points }, path: outline(points) });
    }
    lastCall = number;
    return div();
  }
}
