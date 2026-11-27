// A canvas item that shows a column of pictures: `props.urls`, each a file
// in the space folder or an http(s) URL, under `props.title`. It draws no
// panel of its own: it runs out of sight and sends each feed item its
// display list with `item.draw`, again only when the item's props or size
// change. `feed/shuffle` (the dock button, the palette, Option+Shift+F)
// turns the pictures of the selected feeds one place round.
//
// Fill one from the CLI:
//   specular run item.set '{"id": "<item id>", "props": {"urls": ["https://..."]}}'
import { View, div } from "gpui-kit";
import { state, run } from "specular";

const PAD = 12;
const TITLE = 28;
const GAP = 8;
const INK = "#1f2328";
const MUTED = "#6e7781";
const CARD = "#ffffff";
const RIM = "#d0d7de";

/** What a feed item of `width` x `height` shows. */
function draw(item) {
  const { width, height } = item;
  const props = item.props || {};
  const urls = Array.isArray(props.urls) ? props.urls : [];
  const draws = [
    { type: "rect", x: 0, y: 0, width, height, fill: CARD, stroke: RIM, line: 1, radius: 12 },
    { type: "text", x: PAD, y: PAD, text: props.title || "Image feed", size: 15, color: INK, width: width - 2 * PAD },
  ];
  if (urls.length === 0) {
    draws.push({
      type: "text",
      x: PAD,
      y: PAD + TITLE,
      text: "No pictures yet. Give this item props.urls: files in the space or http(s) URLs.",
      size: 12,
      color: MUTED,
      width: width - 2 * PAD,
    });
    return draws;
  }
  // Two columns once there is room for them.
  const columns = width >= 300 ? 2 : 1;
  const cell = (width - 2 * PAD - (columns - 1) * GAP) / columns;
  urls.forEach((src, n) => {
    const x = PAD + (n % columns) * (cell + GAP);
    const y = PAD + TITLE + Math.floor(n / columns) * (cell + GAP);
    if (y + cell <= height - PAD) {
      draws.push({ type: "image", x, y, width: cell, height: cell, src, fit: "cover", radius: 8 });
    }
  });
  return draws;
}

const drawn = new Map();
let lastCall = null;

function shuffle(items, ids) {
  for (const item of items) {
    if (ids.length > 0 && !ids.includes(item.id)) continue;
    const urls = (item.props && item.props.urls) || [];
    if (urls.length > 1) {
      run("item.set", { id: item.id, props: { urls: [...urls.slice(1), urls[0]] } });
    }
  }
}

export default class Feed extends View {
  render() {
    const items = (state("canvas/mod_items") || []).filter((item) => item.mod === "feed");
    for (const item of items) {
      const key = JSON.stringify([item.width, item.height, item.props]);
      if (drawn.get(item.id) !== key) {
        drawn.set(item.id, key);
        run("item.draw", { id: item.id, draws: draw(item) });
      }
    }
    const call = state("call");
    const number = call ? call.number : 0;
    if (lastCall !== null && number > lastCall && call.command === "feed/shuffle") {
      shuffle(items, (call.args && call.args.ids) || []);
    }
    lastCall = number;
    return div();
  }
}
