# A classic layers system in this spike

How far can the `electrobun-canvas` spike go toward a **classic layers system** —
one unified, user-defined, dynamic z-stack (drag to reorder, toggle visibility,
set opacity) where **live pages are just layers**, freely interleaved with
stickies at any depth — using only the primitives this prototype already has?

Read [`PAGE-STACKING.md`](PAGE-STACKING.md) first for the page↔page reorder gap;
this note zooms out from "reorder pages" to "treat pages as layers," but stays
entirely inside the spike. Nothing here touches anything outside
`experiments/electrobun-canvas/`.

## The model is already in the spike

The hard half of a layers system — the **ordered model** — is already here.
`core/scene.ts` gives pages and stickies **one shared `z`**, and `stepZ` walks
*any* entity past *any* other, so a sticky parks between two pages and (in the
model) a page reorders past another page. A "layers panel" would just be a list
view bound to that shared z-order with drag-reorder + a visibility/opacity flag
per item — pure renderer work in `mainview/`, no new substrate.

So the open question is **not** the model. It is the **rendering substrate**: what
makes that shared z *visually true* when live pages overlap.

## The primitives the spike has to work with

Everything below is built from what `<electrobun-webview>` exposes today
(`EbWebview.tsx`) plus the host DOM:

- **mask selectors** — punch host-DOM-shaped holes in a webview (`core/layering.ts`).
- **`transparent`** — webview background lets host content show through.
- **`togglePassthrough`** — click-through gate (the inert↔live rule in `PageBody`).
- **`toggleHidden`** — stop a webview painting without destroying it.
- **snapshot** (`SnapshotCallback` → data URL) — a frozen bitmap of a page, drawable
  as a host-DOM `<img>`. *Not yet wired in the spike; macOS-only.*
- **the host DOM is the canvas** — stickies, cards, and `<img>` snapshots all
  obey ordinary `z-index` among themselves.

## The constraint, in spike terms

An `<electrobun-webview>` is **one native layer painted above the host DOM**. In
any overlap of two *live* webviews, exactly one wins the pixels and input there —
masks reveal *host DOM*, never the other webview. So **arbitrary z among multiple
simultaneously-live pages is not reachable by stacking webviews**; it needs the
stacked-under pages to stop being live native surfaces and become host-DOM content
the browser can composite. That's the axis the options below move along.

## Substrate options reachable in the spike

### S1 — Single-live + host-DOM proxies  *(in reach today, no native work)*

Only the selected page is a live `<electrobun-webview>`; every other page is a
host-DOM **card** (or snapshot `<img>`) sitting in the shared z like a sticky.
Because only one native surface is ever live, the *whole* stack — pages and
stickies alike — orders arbitrarily and is visually honest, and the live page
masks whatever sits above it.

- This is exactly `PAGE-STACKING.md` option 1 + the keep-alive single-visible
  refinement. **Liveness: 1 page at a time.**
- It already delivers the classic-layers *feel*: one unified z-stack, drag/▲▼ to
  reorder anything past anything, only the *active* layer is live.
- **Verdict:** the layers system the spike can actually ship. Start here.

### S2 — Snapshot-composited stack  *(in reach on macOS, needs the snapshot primitive)*

Upgrade S1's proxies from placeholder cards to **live-looking frozen pages**: every
unselected page is its own snapshot `<img>` in the host DOM, composited by the
browser's own `z-index`; promote a page to a live webview only while it's the
focused layer. Refresh a page's snapshot on a timer or on demand so it doesn't go
stale.

- Still **one live page at a time**, but the stack now *looks* like real pages
  layered at arbitrary depth — the closest the spike gets to "many pages as
  layers" without leaving stock electrobun.
- Only new spike work: wire the `SnapshotCallback` path into a `PageBody`
  snapshot body. macOS-only, like masks.
- **Verdict:** the high-fidelity form of S1; the recommended target *inside* the
  spike.

### S3 — iframe layers for framing-permissive content  *(in reach, renderer-only)*

For content that *allows* embedding (your own pages, local dev servers), render the
layer as a host-DOM `<iframe>` instead of an `<electrobun-webview>`. An iframe is
ordinary DOM: it obeys `z-index`, blends, and stays fully live — **true arbitrary
z, no native-surface problem at all**.

- In the spike: a second body kind alongside `PageBody` (an `IframeBody`) chosen
  per page by a "does this URL allow framing?" check; `CanvasItemView` dispatches
  to it like any other body.
- **Limited** to framing-permissive URLs (most public sites block it via
  `X-Frame-Options` / CSP), so it's a *per-content fast path*, not a general
  substrate — but where it applies, it's the cleanest possible layer.
- **Verdict:** worth adding as a coexisting body; gives genuinely-live, fully-
  composited layers for the content that permits it.

## Out of the spike's reach (named so they're not mistaken for options)

- **Many pages live *and* blended at arbitrary z.** Requires rendering every page
  offscreen and compositing the textures yourself — i.e. a CEF/OSR build
  (`bundleCEF: true`) plus your own GPU compositor. That's a different app than
  this single-window, host-DOM-canvas spike, so it's out of scope here.
- **Per-page native windows ordered by window level.** Would keep pages live and
  reorderable, but the spike is deliberately one window with the host DOM as the
  canvas (`src/bun/index.ts` is thin by design); multiple windows break the
  mask-based DOM interleave the spike exists to prove. Not a fit.

## Recommendation for the spike

1. The **layers model is already done** (shared z in `scene.ts`) — a layers-panel
   list view with drag-reorder + visibility/opacity is pure `mainview/` UX.
2. Ship **S1** as the substrate: one unified z-stack, selected page live, proxies
   for the rest (keep-alive single-visible to avoid reload-on-select).
3. Upgrade to **S2** by wiring the snapshot primitive so inert layers look like
   real frozen pages.
4. Add **S3 (iframe body)** as a coexisting fast path for framing-permissive
   content — the only way to get a *fully live* page layer inside the spike.

Anything requiring many simultaneously-live blended pages is explicitly beyond
this prototype and stays out of scope.
