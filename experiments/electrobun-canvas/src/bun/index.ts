import { BrowserWindow, Updater } from "electrobun/bun";

// The main (Bun) process is intentionally thin for this spike: it opens one
// window and loads the canvas view. All the interesting layering work happens
// in the renderer, because that is exactly the point we're testing — that the
// host DOM can own the canvas and the interaction model with no main-process
// view juggling (no per-page WebContentsView, no layer-stack, no input
// forwarding). There is deliberately no RPC here.

const DEV_SERVER_PORT = 5173;
const DEV_SERVER_URL = `http://localhost:${DEV_SERVER_PORT}`;

async function getMainViewUrl(): Promise<string> {
  const channel = await Updater.localInfo.channel();
  if (channel === "dev") {
    try {
      await fetch(DEV_SERVER_URL, { method: "HEAD" });
      console.log(`HMR enabled: serving canvas from ${DEV_SERVER_URL}`);
      return DEV_SERVER_URL;
    } catch {
      console.log("Vite dev server not running — run `bun run dev:hmr` for HMR.");
    }
  }
  return "views://mainview/index.html";
}

const url = await getMainViewUrl();

new BrowserWindow({
  title: "Electrobun Canvas — layering spike",
  url,
  frame: { width: 1280, height: 860, x: 120, y: 120 },
});

console.log("electrobun-canvas started");
