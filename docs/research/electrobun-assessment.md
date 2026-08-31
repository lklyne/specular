# Electrobun assessment: would rebuilding Specular on it fix our stacking problem?

> Research spike, June 2026. Question from the team: could we rebuild Specular
> (or its core) on **Electrobun** instead of Electron, and would that make
> webpage-over-webpage stacking and layer management easier?
>
> Short version: **Electrobun would make the layer-management problem we
> actually feel today dramatically easier, but it does not — and cannot —
> make two opaque, interactive live web pages truly stack on top of each
> other.** That last part is a property of native webviews on every
> framework, Electron included. Details below.

---

## 1. What Electrobun is (state as of June 2026)

Electrobun (by Blackboard, `blackboardsh/electrobun`) is a desktop app
framework that is roughly "Electron, but the main process runs on **Bun**
instead of Node, and pages render in the **OS-native webview** instead of a
bundled Chromium."

- **Runtime:** A tiny Zig launcher boots Bun; your main-process TypeScript runs
  in a Bun web worker and drives the native GUI over Bun FFI into a native
  wrapper library (`libNativeWrapper.dylib` on macOS — Obj-C/C++; CEF-based
  `.so`/`.dll` on Linux/Windows).
- **Renderer per platform:** WKWebView on macOS, WebKit2GTK on Linux, WebView2
  on Windows by default — *or* bundled **CEF (Chromium)** on any platform via a
  `bundleCEF: true` flag when you want Chromium parity.
- **Size / speed:** ~13 MB self-extracting bundle (vs ~6× that for Electron),
  BSDIFF updates as small as ~14 KB, <50 ms startup. Built-in updater,
  code-signing/notarization, typed main↔webview RPC.
- **Maturity:** v1.0 shipped **Feb 2026**; **v1.18** by May 2026. macOS 14+,
  Windows 11+, Ubuntu 22.04+ are all "official." It is young — small ecosystem,
  thinner docs, rougher edges than Electron. This is the dominant *risk*, not a
  capability gap.

The headline feature for us is its **webview tag**, which is a genuinely
different and more capable take on embedding live web content than anything
Electron offers. That is the whole reason this is worth considering.

---

## 2. Our stacking problem, stated precisely

From our own docs, so we're measuring Electrobun against the real constraint:

- Live pages are **Electron `WebContentsView`** (WCV) instances. Native WCV
  z-order is controllable **only** by the order you call `addChildView` —
  there is no CSS `z-index`, no per-view z API
  (`src/main/runtime/layer-stack.ts`, `page-factory.ts`).
- So we run **three fixed planes**: `bgView` (grid/borders) below, the live
  pages in the middle, `aboveView` (notes, stickies, shapes, annotations,
  overlay UI — all React DOM) on top — pinned above *all* pages by
  `LAYER_STACK` (`docs/interaction-layer.md` §3.1).
- **ADR 0014** records the consequence bluntly: *"Cross-surface visual
  stacking (sticky behind a page, page over a drawing) is architecturally
  impossible on the current surface model. `aboveView` always paints above all
  pages."* The two-section sidebar (Notes vs Pages) exists to *surface* this
  limitation rather than pretend one unified stack order works.
- We also carry a hack: `cursorOverlayWindow` is a separate child
  `BrowserWindow` purely because **WCV can't `setIgnoreMouseEvents()`** in
  current Electron — i.e. we can't make a native layer click-through, so agent
  cursors live in a sibling window.

So "the stacking issue" is really **two different problems** wearing one coat:

- **Problem A — DOM ↔ page interleaving (the one that actually hurts):** notes,
  stickies, drawings, annotations, and overlay UI cannot interleave in z with
  live pages. Everything DOM is forced above or below the entire page plane.
- **Problem B — page ↔ page stacking (the hard one):** two live, opaque,
  interactive web pages overlapping, with arbitrary z-order between them, both
  live in their overlap region.

Electrobun has a very different answer for each.

---

## 3. How Electrobun's webview model differs

Architecturally the `<electrobun-webview>` tag is the *same shape* as our WCV
approach — a native view overlaid on a DOM anchor that reports its
position/size — and it carries the *same base constraint*. From the docs,
verbatim:

> "Because the embedded webview is a separate native layer painted on top of
> the host page, normal DOM stacking (`z-index`, absolute positioning, fixed
> headers, dropdown menus, modals) **cannot draw over it — the OOPIF always
> wins**."

Webview-to-webview ordering is likewise native subview order — on macOS the
wrapper literally does
`addSubview:webView positioned:NSWindowAbove relativeTo:nil`. There is **no
`z-index` and no public reordering API** on `BrowserView`. The
`multitab-browser` template confirms the idiom: switching tabs just calls
`webview.toggleHidden(true)` on the inactive ones — it hides, it doesn't
composite. So far this is Electron with different nouns.

**The difference is three primitives Electron simply does not have:**

1. **Mask selectors** — the big one. You hand a webview tag a list of CSS
   selectors (`masks=".tooltip, #sidebar"`, or `addMaskSelector()` at runtime).
   Every frame, Electrobun runs `querySelectorAll` and **cuts holes in the
   native webview wherever those host-DOM elements land**. The host page paints
   *through* the holes, and clicks/scrolls in those regions hit the host
   element instead of the embedded page. The docs call this out as exactly the
   thing that "makes interactive host-page UI on top of an OOPIF possible:
   tooltips, autocomplete popovers, context menus, sidebars that overlap the
   webview, drag handles, custom title bars." Holes track their elements
   automatically on every layout sync.

2. **`transparent`** — a webview tag can have a transparent background so host
   content shows through its non-painted regions.

3. **`passthroughEnabled` / `togglePassthrough()`** — mouse and touch events
   pass through the webview to underlying elements. This is the built-in,
   first-class version of the `setIgnoreMouseEvents` capability we had to fake
   with a sibling `BrowserWindow`.

There's also a native **snapshot** path (`SnapshotCallback` → data URL; docs
mention "mirror a screenshot of the webview tag's contents to the host anchor
and hide it, or stream an image of the contents") — useful for cheap static
representations of off-screen pages.

---

## 4. Does it fix the stacking problem?

### Problem A (DOM ↔ page interleaving): **yes, substantially.**

This is the part that actually causes us pain, and Electrobun's mask + transparency +
passthrough primitives are almost purpose-built for it. The whole three-WCV
plane dance collapses:

| Specular today (Electron) | Electrobun equivalent |
|---|---|
| `aboveView` WCV pinned above all pages; notes/stickies/shapes can't go behind a page | The **host BrowserView *is* the canvas DOM**. Pages are `<electrobun-webview>` tags punched into it. Any note/sticky/annotation that should sit **above** a page is just a host-DOM element registered as a **mask** → it paints and receives input on top of the live page. |
| `bgView` WCV for grid/borders below pages | Host-DOM background; pages are transparent-capable layers above it. |
| `cursorOverlayWindow` sibling window because WCV can't be click-through | Native `passthroughEnabled` — delete the hack. |
| Cross-surface stacking "architecturally impossible" (ADR 0014) | A note *above* a page = mask it. A note *below* a page = leave it in host DOM and don't mask it. The Notes-vs-Pages sidebar split stops being a forced compromise. |

In other words, the specific thing ADR 0014 calls impossible — "sticky behind a
page, page over a drawing" *between our DOM surfaces and pages* — becomes a
per-element masking decision rather than a hard plane boundary. That's a real,
concrete win and it maps cleanly onto what we already have.

### Problem B (page ↔ page true stacking): **no — and this is physics, not Electron.**

Two opaque, interactive, *live* native web pages overlapping, both fully live
in the overlap region with arbitrary z between them, is **not solvable by
Electrobun either.** A native webview is a real OS surface; in any overlap,
exactly one surface is frontmost and owns the pixels and the input there. Masks
help (you can punch the front page so the back one shows/clicks through a hole),
but a hole is all-or-nothing per region — it's not blending two live pages, and
it doesn't give you "page B floating semi-transparently over page A while both
keep running and both receive input." Electron, Tauri, and Electrobun are all
equally bound by this.

The **only** way to get true arbitrary page-over-page compositing is to stop
using live native surfaces for the stacked ones and **render pages to
bitmaps/textures and composite them yourself** (offscreen render → GPU/canvas
compositor). We already identified exactly this as the "Tier 2 offscreen
bitmap" approach in `docs/offscreen-rendering-research.md`. That path is
**renderer-agnostic** — we could build it on our *current* Electron stack
(`offscreen: true` + `paint` events) without switching frameworks at all.

So: **Electrobun is not the thing that unlocks Problem B.** Offscreen
compositing is, and that's available to us today.

---

## 5. Capability parity for Specular's actual dependencies

Could we do "the core" of what we do on Electron? Mostly yes. Checklist:

| Specular needs | Electrobun status |
|---|---|
| Many isolated live webviews | ✅ Core feature; each OOPIF is its own process. |
| Per-view partitions / sessions | ✅ `partition` / `persist:` partitions. |
| DevTools per page | ✅ `openDevTools()` per webview. |
| Navigation control + allow/block rules | ✅ `setNavigationRules()` (native, synchronous). |
| Snapshots of pages | ✅ Native snapshot/stream-image path. |
| DOM overlays interleaved with pages | ✅✅ Masks + transparency + passthrough (the headline win). |
| Click-through overlay layers | ✅ Native passthrough (removes our sibling-window hack). |
| Main-process HTTP API (we have one) | ✅ Bun has first-class `Bun.serve` — arguably nicer than our current setup. |
| Yjs / `.canvas` file persistence | ✅ Pure JS/files; runs fine on Bun. |
| Typed main↔renderer IPC | ✅ Built-in typed RPC (replaces our preload-bridge zoo). |
| **Programmatic synthetic input injection** into a page (we forward events via `webContents.sendInputEvent` in `page-input-forwarding.ts`) | ⚠️ **Gap to verify.** The native layer forwards *real* OS input to the focused OOPIF, and `executeJavascript()` can synthesize DOM events, but I did **not** find a public "inject a mouse event at (x,y)" Bun API equivalent to `sendInputEvent`. Much of our forwarding need disappears because the webview tag gets native input directly + passthrough/masks, but **agent-driven/automated clicking into a page needs a spike.** |
| Per-page offscreen render to bitmap (for Problem B / many cold pages) | ⚠️ Partial. CEF path has OSR (Linux/Win); macOS WebKit OSR is unclear. If we want offscreen compositing we'd likely commit to `bundleCEF`, which gives up the small-bundle/native-webview advantage. |
| Mature crash logging / Crashpad equivalent | ⚠️ Less developed than Electron's. We rely on `errors.log` + Crashpad dumps today. |

### Platform caveats on the headline feature

Masks/passthrough are **fully supported only on macOS (WebKit).** Per the docs:

- **Linux:** passthrough + mask punch-through are **not supported** inside
  transparent `BrowserWindow`s (transparent CEF renders offscreen into the X11
  parent). Use a non-transparent window for masked overlays.
- **Windows:** **not supported** on default WebView2 (its D3D intermediate
  window bypasses the compositor). You must set `bundleCEF: true` so the webview
  is CEF-backed to get masks/passthrough.

Since Specular is macOS-first (our crash-log paths are all `~/Library/...`),
this is fine *today*, but it means the layering win is partly macOS-specific
unless we ship CEF on Windows.

---

## 6. Migration cost & risk

A move is a **full main-process rewrite**, not a port:

- Everything in `src/main/` that touches Electron (`BrowserWindow`,
  `WebContentsView`, `app`, `ipcMain`, `webContents`, the 9 preload bridges,
  `layer-stack.ts`, `page-factory.ts`, the whole interaction layer) is
  Electron-specific and gets rebuilt against Electrobun's window/webview/RPC
  model. Node built-ins → Bun. The good news: `src/shared/`, `.canvas`
  persistence, Yjs, and most of `src/renderer/` (it's just React/DOM) port with
  far less churn — and the renderer side actually gets *simpler* because the
  canvas DOM becomes the host page rather than an isolated WCV.
- We'd be betting the app on a **~5-month-old v1 framework** with a small
  community. If we hit a native bug, we're reading Zig/Obj-C/CEF, not leaning on
  Electron's enormous corpus of answers.
- Some current behaviors need spikes before committing: synthetic input
  injection, macOS offscreen rendering, DevTools/automation depth, and our
  crash-logging story.

---

## 7. Recommendation

1. **Don't expect Electrobun to be the fix for "pages stacking on pages"
   (Problem B).** That's offscreen-bitmap compositing, which is renderer-
   agnostic and buildable on our *current* Electron stack. If page-over-page is
   the priority, prototype Tier-2 offscreen compositing in Electron first — no
   framework switch required.

2. **Electrobun *is* a strong fit for the layer-management pain we actually
   feel daily (Problem A).** Masks + transparency + passthrough would let DOM
   notes/stickies/annotations interleave with live pages and delete both the
   rigid three-plane model and the `cursorOverlayWindow` hack. That directly
   dissolves the ADR-0014 constraint and the forced Notes/Pages sidebar split.

3. **Suggested next step — a time-boxed spike, not a rewrite.** Build a tiny
   Electrobun prototype that reproduces Specular's core layering scenario:
   2–3 overlapping live `<electrobun-webview>` pages on a pannable/zoomable
   host-DOM canvas, with draggable host-DOM stickies that mask **over** some
   pages and sit **under** others, plus a passthrough cursor layer. That single
   prototype validates (or kills) the layering thesis and surfaces the input-
   injection and macOS-OSR unknowns cheaply, before we commit to a port.

**Bottom line:** Electrobun won't make two live pages truly stack — nothing
short of offscreen compositing will, and that's already on our table without
switching. But it would make *cross-surface* layer management (the thing that
forced ADR 0014 and the split sidebar) genuinely easier and more native. The
honest framing for the team: this is a **layer-management upgrade with a real
rewrite cost on a young framework**, not a silver bullet for page-on-page
stacking.

---

### Sources

- Electrobun repo `blackboardsh/electrobun` — `docs/`, `package/src/native/macos/nativeWrapper.mm`,
  `package/src/bun/core/BrowserView.ts`, `templates/multitab-browser/`.
- Docs: Webview Tag Architecture; Electrobun Webview Tag (Mask Selectors,
  `transparent`, `passthroughEnabled`); BrowserView/BrowserWindow API;
  Architecture Overview; changelog v1.0–v1.18.
- Specular: `docs/adr/0014-canvas-stack-order.md`, `docs/interaction-layer.md`,
  `docs/offscreen-rendering-research.md`, `src/main/runtime/layer-stack.ts`,
  `page-factory.ts`, `page-input-forwarding.ts`, `CONTEXT.md`.
