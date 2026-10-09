import type { ElectrobunConfig } from "electrobun";

// Self-contained Electrobun app. Vite builds the renderer to ./dist, and the
// `copy` map below lifts that output into the Electrobun bundle's views/ tree
// so the main process can load it via `views://mainview/index.html`.
export default {
  app: {
    name: "electrobun-canvas",
    identifier: "canvas.electrobun.experiments.specular",
    version: "0.0.1",
  },
  build: {
    copy: {
      "dist/index.html": "views/mainview/index.html",
      "dist/assets": "views/mainview/assets",
    },
    // Vite owns view rebuilds (HMR); don't let Electrobun's watcher fight it.
    watchIgnore: ["dist/**"],
    // macOS WebKit (WKWebView). Mask selectors + passthrough are fully
    // supported here — which is the whole point of the spike. CEF would only
    // be needed for the same behavior on Windows.
    mac: { bundleCEF: false },
  },
} satisfies ElectrobunConfig;
