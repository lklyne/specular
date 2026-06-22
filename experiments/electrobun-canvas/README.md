# electrobun-canvas — layering spike

A small, self-contained Electrobun + Vite + React app that recreates Specular's
spatial canvas to test **one question**:

> Can a sticky note sit **in front of** one live web page while staying **behind**
> another, at the same time — the cross-surface stacking that ADR 0014 calls
> "architecturally impossible" on Electron's `WebContentsView` model?

Background and the full Electron-vs-Electrobun comparison live in
[`docs/research/electrobun-assessment.md`](../../docs/research/electrobun-assessment.md).

This is a throwaway experiment. It is **not** wired into the main app, the pnpm
workspace, or the root typecheck/test commands.

## The idea in one paragraph

The **host DOM is the canvas.** Pages are `<electrobun-webview>` tags; stickies
are plain host-DOM elements. A native webview always paints above the host DOM, so
a sticky is normally hidden behind every page. The escape hatch is **mask
selectors**: for each page we tell its webview to punch holes wherever specific
host-DOM elements land. Because the mask set is **per page**, a sticky can punch
through page A (appearing in front of it) while page B has no such hole (so it
covers the sticky). One shared z-order drives which stickies are in each page's
mask set — see [`src/mainview/core/layering.ts`](src/mainview/core/layering.ts).

## Run it (macOS)

Mask selectors + passthrough are fully supported only on **macOS (WKWebView)**,
and the app needs the native Electrobun runtime, so this builds and runs on a Mac
— **not** in CI / Linux.

```bash
cd experiments/electrobun-canvas
bun install
bun run dev:hmr     # Vite HMR + Electrobun dev build
# or: bun run dev   # no HMR
```

> The `electrobun` dependency is pinned to a beta (`1.18.4-beta.6`). If it fails
> to resolve, bump to the latest with `bun add electrobun@beta`.

## What to look for

1. **Thesis (default scene):** three overlapping pages and a yellow sticky parked
   at z=1 — above `page-a` (z=0), below `page-b` (z=2). In the A∩B overlap it
   shows through A yet is covered by B **simultaneously**. Select it and press
   **▼ / ▲** to walk it across each page; drag it around the overlap.
2. **Select-to-interact (one rule, every item):** on launch nothing is selected,
   so everything is inert — a single click selects an item (outline + chrome
   accent) without driving it; click again / type to interact. Only the selected
   page takes scroll/keyboard; only the selected sticky is editable. **Esc** (or a
   click on empty canvas) returns everything to inert. The whole gate is one rule
   — `live = selected && !panActive` (`core/interactivity.ts`) — applied through
   one shared shell (`canvas/CanvasItem.tsx`); the only per-kind difference is how
   a body goes inert/live (page → `togglePassthrough`, sticky → `contentEditable`).
3. **No gesture overlay:** hold **space** and drag to pan even while the pointer is
   over a page (pages flip to passthrough); drag any item by its **chrome bar**.
   All of this works with **zero input-capturing overlay** — contrast Specular's
   `aboveView` + `cursorOverlayWindow` + `page-input-forwarding.ts`.
4. **Passthrough overlay:** click **Show passthrough overlay** — a transparent
   webview paints a watermark over everything, yet clicks and scroll still reach
   the pages beneath it. (This is what would replace the `cursorOverlayWindow`
   hack.)

## Map

```
src/bun/index.ts              main process: opens one window (thin by design)
src/mainview/
  App.tsx                     state + the rigged starting scene
  core/camera.ts              {x,y,zoom} + pure transforms (re-derived)
  core/scene.ts               Page/Sticky model + shared-z operations
  core/layering.ts          ★ per-page mask sets from the shared z-order
  core/interactivity.ts       the one rule: live = selected && !panActive
  canvas/Canvas.tsx           gesture root + world transform
  canvas/CanvasItem.tsx       shared item shell (placement, select, drag, chrome)
  canvas/CanvasItemView.tsx   kind → body dispatch (analog of RendererSwitch)
  canvas/bodies/PageBody.tsx  webview substrate: passthrough gate + mask sync
  canvas/bodies/StickyBody.tsx host-DOM substrate: contentEditable gate
  canvas/CursorOverlay.tsx    transparent + passthrough overlay
  canvas/EbWebview.tsx        typed React wrapper for the custom element
  hooks/                      useCamera, useDrag, usePanTool
```

## Known limits (in scope to observe, out of scope to solve)

- **Page-over-page is creation order.** Two *native* webviews overlapping have a
  single winner in the overlap; there is no public z-order API to reorder them
  (the `▲▼` on a page chrome restacks it relative to **stickies**, via masks, not
  relative to other pages). This is the page-reordering gap — see
  [`PAGE-STACKING.md`](PAGE-STACKING.md) for the three ways to close it:
  (1) a no-fork **single-live** model (only the selected page is live; others are
  cards; reordering becomes shared-z), (2) a small **native reorder** fork
  (multiple live pages, restacked in place), and (3) **snapshot/bitmap**
  offscreen compositing (the real, framework-agnostic fix for true page-over-page,
  "Problem B").
- **macOS only** for masks/passthrough (Windows would need `bundleCEF: true`).
- **Zoom** scales the page's native overlay to the anchor's screen rect; page
  content is not re-zoomed via `setPageZoom` (could be added).
- Native overlays may visibly lag the host DOM by a frame during fast pan/zoom.

## Findings log

_Fill in after running on macOS:_

- [ ] Thesis holds — sticky above A / below B renders correctly in the overlap?
- [ ] Restacking (▲▼) moves a sticky across individual pages as expected?
- [ ] Select-to-interact: one click selects (inert), second click / typing interacts; Esc deselects?
- [ ] The same shell + rule governs both pages and stickies (substrate logic isolated to body files)?
- [ ] Pan/zoom keeps pages + stickies locked to canvas coords; overlay lag tolerable?
- [ ] Native hit-testing + passthrough fully replaced the gesture overlay (no input layer needed)?
- [ ] Passthrough overlay: clicks/scroll reach pages beneath?
- [ ] Page interactivity intact in unmasked regions; masked regions route to host?
- [ ] Open question — synthetic input injection: is there a public API to inject a
      click at (x,y) into a page (Specular's `sendInputEvent`)? Needed for agent-driven
      annotation. Record what you find.
- [ ] Open question — macOS offscreen/snapshot path for many cold pages.
