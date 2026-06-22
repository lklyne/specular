# A classic layers system with live pages: the substrate options

> Research survey, June 2026. Question from the team: can Specular have a
> **classic layers system** — one unified, user-defined, dynamic z-stack (drag to
> reorder, toggle visibility, set opacity) — where **live web pages** are just
> layers like everything else, freely interleaved with notes, drawings, and
> shapes at any depth?
>
> Short version: **the layers *model* is already built; the hard part is the
> compositing *substrate*.** `entityOrder` (ADR 0014) is already one ordered list
> across every kind, so the layers *panel* is renderer/UX work we can do today.
> What it can't yet do is tell the *visual* truth for overlapping **live** pages —
> and that is a property of how pages are painted, not of the model. This doc maps
> every substrate that could close that gap, from "ships now" to "true GPU
> compositing."

Builds directly on three existing docs — read them for depth:
[`electrobun-assessment.md`](electrobun-assessment.md) (Problem A / Problem B
framing), [`../offscreen-rendering-research.md`](../offscreen-rendering-research.md)
(the two-tier OSR work), and the prototype's
[`../../experiments/electrobun-canvas/PAGE-STACKING.md`](../../experiments/electrobun-canvas/PAGE-STACKING.md)
(closing the reorder gap inside the spike). It generalises all three to the
broader "unified layers" question and connects to
[`../adr/0014-canvas-stack-order.md`](../adr/0014-canvas-stack-order.md), which
explicitly parks cross-surface visual stacking as a future ADR.

## Two separable things

A "classic layers system" bundles two concerns that have very different costs:

1. **The layers model** — one back-to-front ordered list spanning every kind
   (pages, notes, drawings, shapes, edges), with mutations (bring forward / send
   to back / move before), per-layer visibility, lock, and opacity, surfaced in a
   panel you can drag to reorder.
   - **Specular already has this.** `entityOrder` is a flat back-to-front Y.Array
     of ids across all kinds (ADR 0014); the hit-tester already walks it, the
     persistence layer round-trips it through JSON Canvas, and the mutation math
     (`entity-order-math.ts`) exists. A *unified layers panel* is renderer/UX work
     on top of a model that is essentially done. **None of the substrate options
     below require changing this model** — they change only how a page *paints*.

2. **The compositing substrate** — what actually puts pixels on screen so that the
   model's z-order is *visually honest* for live pages.
   - **This is the unsolved part.** ADR 0014 records the reason: pages render as
     native `WebContentsView`s in a middle plane, with everything-else pinned in
     `aboveView` above them, so "one global stack order across all kinds is
     achievable as a *data* concept but not as a *visual* concept on the current
     surface model." The two-section Notes/Pages sidebar exists *only* to make that
     substrate limit honest. A unified layers panel becomes truthful the moment the
     substrate changes — and nowhere else.

So the rest of this doc is about concern 2 only. The panel UI is not gated on it;
the *visual truth* of that panel is.

## The governing constraint

One fact decides the whole design space:

> **A page rendered as a real OS surface** (`WebContentsView`,
> `<electrobun-webview>`, or a child window) **is composited by the operating
> system.** The OS only knows the order you registered surfaces in, and in any
> overlap it picks exactly **one winner per pixel** — no blending, no opacity, no
> arbitrary interleave with host-DOM content. You get cheap, crisp, fully
> interactive pages, but only OS-granted z.

Therefore: **arbitrary user-defined z among multiple *simultaneously live* pages
requires giving up "the OS composites the stacked pages."** You render pages to
images/textures and composite them yourself. Everything below is a different point
on that trade: how many pages stay live, who owns compositing, and what it costs.

This is the same wall ADR 0014, the Electrobun assessment (Problem B), and the
spike's `PAGE-STACKING.md` all hit from different directions. It is physics of
native surfaces, not an Electron or Electrobun limitation.

## The substrate options

Ordered by how much they relax "the OS composites the pages," i.e. roughly from
ships-now to true-compositing.

### S1 — Single live + proxies (one page live at a time)

At most one page is a real OS surface; every other page is a host-DOM **image**
(snapshot) or card, placed in the unified z like a note. Because only one page is
ever a competing native surface, the *entire* stack — pages and notes alike —
orders arbitrarily and is visually honest.

- **Liveness:** 1 simultaneous live page (plus linked scroll peers, which must
  ride along). **Z freedom:** fully arbitrary order + blending of everything
  *except* the one live page (which masks whatever sits above it).
- **Maturity / cost:** highest. This is the prototype's recommended option (the
  `electrobun-canvas` single-live / keep-alive single-visible model) and the
  `aboveView`-collapse it implies, *and* it's the demotion half of the offscreen
  research's two-tier plan. Ships on either framework, no native work.
- **The compromise:** "live pages" plural is *sequential*, not simultaneous —
  click a back page to make it the live one (a snapshot covers the swap). For a
  design-iteration canvas where you focus one page at a time, this is often
  enough.
- **Verdict:** the honest unified layers panel you can ship first. Everything is
  one z-stack; only the *active* layer is live.

### S2 — Per-page native windows, ordered by window level (all live, order only)

Give each page its own borderless child window instead of a subview in one window.
On macOS, child windows reorder arbitrarily via `orderWindow:relativeTo:` / window
levels — so **every page stays live and restacks in place**, the only *native*,
all-live, reload-free way to get arbitrary *order*.

- **Liveness:** N (all live). **Z freedom:** arbitrary *order* only — still one
  opaque winner per pixel, no blending or opacity.
- **The catch — it breaks the wins we just got.** Host-DOM items can't interleave
  *between* two page-windows (DOM lives in yet another window), so the mask-based
  DOM↔page interleaving the spike proved (Problem A) is lost. Windows are
  rectangular and opaque: clipping to the canvas, rounded corners, overflow, and
  the canvas pan/zoom transform must be re-applied to N windows that visibly lag
  the host surface. Specular already feels this with its single
  `cursorOverlayWindow` sibling window; N of them is worse.
- **Verdict:** delivers unified *order* among live pages but shatters
  DOM-interleave and blending. Niche — only if opaque, rectangular, order-only
  live pages are acceptable and DOM interleaving isn't.

### S3 — Self-composited offscreen rendering (the real classic-layers substrate)

Render every page offscreen to a bitmap/GPU texture and composite all layers —
pages, notes, drawings — yourself, in one surface, in arbitrary z with opacity,
blend modes, and effects. This is literally how Figma, Photoshop, and Chromium's
own compositor work: everything is a layer, you own the stack.

- **Liveness:** N (all live). **Z freedom:** fully arbitrary — order **plus**
  opacity, blending, masks, effects. This is the only substrate that satisfies the
  question in full.
- **Transport sub-variants** (from `offscreen-rendering-research.md`):
  - **S3a — CPU bitmap OSR.** Electron `offscreen: true` → `paint` events, or
    `capturePage()` (already used in `frame-compositor.ts`). Works today, simplest,
    but a GPU→CPU→GPU round-trip + IPC bandwidth (~32 MB/frame at 1080p@2×). Fine
    for many *low-fps* layers; janky for many high-fps live ones.
  - **S3b — GPU shared-texture OSR.** `useSharedTexture` (Electron 33+, available
    in our v40) or CEF `OnAcceleratedPaint`: frames stay on the GPU as
    IOSurface/D3D textures, consumed by a WebGL/Metal compositor — near zero-copy,
    scales to many live layers. Needs a native addon. **This is the performant,
    correct version** and the standard technique for browser-as-layer (OBS, Steam
    overlay, game-engine web views).
  - **S3c — Dedicated GPU web renderer.** Ultralight / Servo-WebRender render web
    content straight to GPU textures built for compositing. Architecturally the
    cleanest "many pages as layers," but fidelity/compat tradeoffs (not full
    Chromium) and a heavy integration.
- **The costs (all S3 variants):**
  - **Input.** You lose native hit-testing; every click/scroll/keystroke must be
    forwarded as synthetic input at canvas-transformed coords. Specular already
    does exactly this (`page-input-forwarding.ts` → `sendInputEvent`), so it's a
    known quantity, not a new unknown — but it now applies to *every* live layer.
  - **Fidelity.** DRM/EME content (Netflix, protected video) won't capture;
    some GPU-accelerated video/WebGL paths differ under OSR; text must render at
    device pixel ratio for crispness; the page's accessibility tree is lost to the
    OS.
  - **Resources.** OSR removes the *window surface*, not the *renderer process* —
    N live pages is still N Chromium processes. Memory/CPU scale with live layers.
- **Verdict:** the true classic-layers substrate, at real cost. Framework-agnostic:
  buildable on today's Electron, no migration required. Overkill if you never need
  many pages live and blended at once.

### S4 — Hybrid promote/demote (the pragmatic classic-layers substrate)

Bound S3's cost: keep a small **hot set** as live native surfaces (the
frontmost-per-overlap and/or focused page), render the rest as snapshots (instant)
or low-fps OSR, and composite. As z changes, **promote** a layer to native when it
needs to be live/frontmost and **demote** it to a texture when it drops behind.

- This is `offscreen-rendering-research.md`'s two-tier model, generalised from
  *selection-driven* to *z-driven*, and the `PAGE-STACKING.md` hybrid mask+snapshot
  idea is its degenerate one-live case.
- **Scope it to overlaps.** The expensive treatment only ever needs to run where
  layers actually overlap; pages with disjoint rects stay native and free
  regardless of z. On a typical canvas (mostly non-overlapping pages) the hot set
  is tiny and OSR fires rarely. (Cross-cutting refinement 1 from `PAGE-STACKING.md`.)
- **Hard part:** the promote/demote handoff must be seamless — snapshot before
  demotion, show it while the native view spins up on promotion, preserve scroll /
  form / focus across the swap (the same state-preservation problem the offscreen
  research flags).
- **Verdict:** the realistic way to ship S3's UX. Live where it's cheap (native,
  on top, non-overlapping); composited only where the stack actually demands it.
  Built on **S3b** for the demoted set, this is the recommended target architecture
  for a true unified layers system.

### S5 — Put pages in the DOM (no separate surface at all)

If web content lived in the host DOM, `z-index` would give arbitrary z, blending,
and all-live for free — no substrate problem at all.

- **S5a — `<iframe>`.** True DOM layers, but only for **first-party or
  framing-permissive** content. Most real sites block embedding via
  `X-Frame-Options` / CSP `frame-ancestors` — the very reason Specular uses native
  webviews. Viable as a *fast path* for content you control (your own design
  system, local dev servers), not as a general substrate.
- **S5b — framing proxy.** A proxy that strips `X-Frame-Options`/CSP and rewrites
  URLs makes arbitrary sites iframe-able, but breaks auth, relative URLs, and
  in-page JS; it's fragile, carries real security exposure (you're MITM-ing the
  page) and ToS/legal risk. Not recommended for core; noted for completeness.
- **Verdict:** not a general answer, but S5a is a legitimate *per-content* fast
  path that can coexist with any substrate above (permissive content → DOM layer;
  everything else → native/OSR).

## Summary

| Substrate | Live pages | Z freedom | Owns compositing | Maturity | DOM-interleave kept |
|---|---|---|---|---|---|
| **S1** Single-live + proxies | 1 at a time | arbitrary (1 live) | OS (1) + host DOM | **Ships now** | ✅ (masks) |
| **S2** Per-page windows | N | order only | OS (windows) | Native, fiddly | ❌ |
| **S3** Self-composited OSR | N | order + blend + opacity | **You** | Real build | ✅ (you compose) |
| **S4** Hybrid promote/demote | hot set live | arbitrary | You + OS (hot set) | Real build | ✅ |
| **S5a** iframe | N | order + blend | DOM | trivial / limited | ✅ |

## Recommendation

1. **Build the unified layers panel now.** It rides on `entityOrder`, which already
   spans every kind. The model is done; the panel is UX. This alone replaces the
   ADR-0014 Notes/Pages split with one honest list for the common (non-overlapping)
   case.
2. **Ship S1 (single-live) as the first visually-honest substrate.** One z-stack,
   only the active layer live, snapshots for the rest. Mature, cheap, no native
   work, and it's exactly what the prototype research already lands on.
3. **Target S4 (hybrid promote/demote on S3b shared textures) for true
   simultaneous-live layers**, scoped to overlapping clusters. This is the real
   classic-layers substrate at bounded cost, and it's framework-agnostic —
   buildable on today's Electron without an Electrobun migration.
4. **Reach for S3-full only if** many pages must be live *and* blended at once;
   **S2 only if** opaque order-only live pages are acceptable (they usually aren't,
   given how much the mask-based DOM interleave buys); **S5a as a fast path** for
   framing-permissive content alongside whichever substrate you pick.

**Tie to ADR 0014:** this is precisely the "future ADR re-architecting `aboveView`
content into per-z-band layers" that 0014 left out of scope. A unified, dynamic
layers system = collapsing today's three planes (`bgView` / pages / `aboveView`)
into **one self-composited z-stack** (S3/S4) or **one live page + proxies** (S1).
The Notes/Pages sidebar split is a symptom of the substrate, not the model — fix
the substrate and the unified panel becomes the truth.

### Sources

- Specular: [`../adr/0014-canvas-stack-order.md`](../adr/0014-canvas-stack-order.md),
  [`../offscreen-rendering-research.md`](../offscreen-rendering-research.md),
  [`electrobun-assessment.md`](electrobun-assessment.md),
  [`../../experiments/electrobun-canvas/PAGE-STACKING.md`](../../experiments/electrobun-canvas/PAGE-STACKING.md),
  `src/main/runtime/layer-stack.ts`, `entity-order-math.ts`,
  `page-input-forwarding.ts`, `frame-compositor.ts`, `docs/interaction-layer.md`.
- Techniques: Electron OSR (`offscreen`, `paint`, `useSharedTexture`); CEF
  `OnAcceleratedPaint` shared textures; macOS child-window ordering
  (`orderWindow:relativeTo:`); Ultralight / Servo-WebRender GPU web rendering;
  `X-Frame-Options` / CSP `frame-ancestors` framing constraints.
