# ADR 0023 — Renderer-owned camera and GPU-composited pan/zoom

**Status:** Proposed
**Date:** 2026-06-30
**Related:** [ADR 0014 — Canvas stack order](./0014-canvas-stack-order.md) (the layering constraint this ADR must preserve), [ADR 0002 — Canvas-anchored overlay UI](./0002-canvas-anchored-overlay-ui.md), [ADR 0021 — Focus session](./0021-focus-session-as-first-class-concept.md), [ADR 0022 — Pages select-first / interact-second](./0022-pages-select-first-interact-second.md).
**Origin:** Perf analysis on branch `claude/issue-265-perf-b-and-hud`, distilled from issues [#257](https://github.com/lklyne/specular/issues/257) (selection border lags during pan) and [#265](https://github.com/lklyne/specular/issues/265) (make `buildCanvasLayoutData` cheap for pan-only changes). Those two issues chipped at symptoms; this ADR names the root cause and the end-state architecture.

## Context

Pan and zoom are the two motions a spatial canvas does most, and today both are more expensive than the pixels they move. The cause is architectural, not a hotspot: **the canvas is ~60% migrated to a camera-transform model, but inconsistently, so it pays for two positioning models at once.**

### The two substrates

The canvas composites two kinds of content that behave oppositely under pan/zoom:

1. **Native `WebContentsView`s** (live pages, component previews) — OS-level layers positioned by main via `setBounds`, scaled via `enableDeviceEmulation({ scale: zoom })`. **They cannot ride a CSS transform** and cannot re-layout crisply at 60fps (each re-emulation is a full Chromium reflow). This is irreducible.
2. **DOM overlay** (grid, page borders, device shells, group backgrounds, text/shape/file/drawing bodies, edges, selection chrome, guides, marquee, handles) — split across two renderer surfaces, `bgView` (below pages) and `aboveView` (above pages). **This content wants a single GPU `translate/scale` transform** — pan/zoom for free, like tldraw and Figma.

### Why the DOM pays the pages' tax

Because pan/zoom must go through main for the pages, and because most DOM layers are pinned to main-computed per-entity `screenX/screenY`, every pan/zoom tick incurs three costs the DOM does not need:

- **Cost center 1 — full scene rebuild + serialize + 3× IPC.** `setPan` calls `markDirty('canvas')`; `setZoom` calls `markDirty('canvas', 'toolbar')` (`viewport-control.ts`). That flag gates `buildCanvasLayoutData`, which re-maps every entity to a scene entity (recomputing screen coords), builds an order-rank map, **sorts all entities + edges**, and structured-clones a payload carrying *every* entity, edge, annotation, presence cursor, and inspect-panel blob — sent to three webContents (`bgView`, `aboveView`, `cursorOverlayWindow`). The entity list is identical to the previous tick. Pure waste. (Note: `layout-dirty.ts` already documents that geometry — the per-page `setBounds` loop — runs *unconditionally*; the dirty flag gates only the IPC payload. So pan/zoom sets a flag whose sole job is to trigger work pan/zoom doesn't need.)
- **Cost center 2 — renderer re-render.** `aboveView` has no rAF batching (`bgView` got it in #265; `aboveView` was missed): every payload → `setLayoutData` → full `App` re-render → `SelectionOutlineLayer` re-runs eight `.filter()` passes over the whole entity array (dependency is `layoutData.entities`, a fresh reference every tick, so the memos never hit). ~10–20 ms per payload at 20 pages.
- **Cost center 3 — device-emulation reflow storm on zoom.** `pageEmulationKey` includes `zoom` (`layout-engine.ts`), so every zoom tick calls `enableDeviceEmulation` on every visible page *and* every component view — each a full Chromium reflow. There is no gesture-settle defer and no cheap visual scale during the gesture.

So **pan** is expensive only because of cost centers 1 + 2. **Zoom** is expensive because of 1 + 2 + 3, and it has no live shimmer at all (`useScenePanOffset` returns zero offset when zoom changes, so the DOM waits for the heavy payload).

### What is already correct (the half-migration)

Two pieces of the target model already exist and are load-bearing evidence that it fits:

- **Entity bodies already live in canvas space under one transform.** `ShapeViewportLayer`, `StickyViewportLayer`, `FileViewportLayer` render children at `canvasX/canvasY` inside a single `translate(canvasOrigin.x + pan.x, pan.y) scale(zoom)` container. This is the tldraw model, done right — for three of the layers.
- **A lightweight viewport channel already exists.** `viewport-nudge` (`{ pan, zoom }`, tiny, sent immediately on every `setPan`/`setZoom`, ahead of the debounced payload) is the correctly-shaped message. #264 wired it to the chrome layer as a live-pan *patch* (`translate3d(panOffset)` on the outer container) so the border stops trailing — but it is pan-only, applied over the still-per-entity-`screenX` model, and the heavy payload still rebuilds and sends every tick beside it.

### The constraint that cannot break: layering (ADR 0014)

The two-surface split is not incidental — it is the *only* way to interleave DOM with native WCVs. `LAYER_STACK` (`layer-stack.ts`) pins the child-view order bottom→top: **`bgView` → pages + component WCVs → `aboveView` → devtools → toolbar.** A native WCV cannot sit inside a single renderer's DOM z-order, so "some chrome below the page, some above" *requires* two renderer views with the pages sandwiched between them. This banding (all-DOM-below < all-WCVs < all-DOM-above) is exactly what ADR 0014 §"Cross-surface visual stacking" records as architecturally fixed. **Every decision below preserves it: no content moves between renderers, and `LAYER_STACK` / `applyStack()` are untouched.**

## Decision

Adopt one consistent rule and migrate the remaining layers to it.

> **A single camera, owned by the renderer, drives one GPU-composited transform per renderer over all canvas-space DOM. The heavy layout payload is viewport-independent — it flows only on real content change. Main is demoted to a follower for the native WCVs: it repositions them (`setBounds`) and re-emulates them once per zoom-*settle*, never per tick, and the DOM never waits on it.**

This is the tldraw/Figma model, adapted to the one thing they lack — native WebContentsViews. The specific decisions:

### 1. The renderer owns the camera

Live `camera = { x, y, zoom }` is renderer state, updated synchronously from wheel/gesture input and applied to the scene transform in the same frame — **zero input→transform latency**. This removes even the one-IPC-hop lag the `viewport-nudge` carries today. Main stops being the pan/zoom authority for the *visual*; it becomes a consumer of the renderer's camera.

Rejected: *keep main authoritative, just stop rebuilding* (the minimal version). It removes cost centers 1–2 but leaves one IPC hop of latency on every gesture — not Figma-parity. Adopted as the **Phase 1** stepping stone (main-authoritative, nudge-driven) because it de-risks the authority flip, but the end state is renderer-owned.

### 2. One transform, all DOM in canvas space

Every canvas-space layer in each renderer becomes a child of one `translate(x, y) scale(zoom)` container driven by the live camera:

- `aboveView`: fold selection outlines, edges, guides, drawings, marquee, group bounds, and reorder dots into the same transformed container the entity bodies already use. The outer `translate3d(panOffset)` shimmer patch and the inner body transform **merge into one** — the patch *becomes* the primary transform.
- `bgView`: grid, page borders, device shells, and group backgrounds under one transform.

Per-entity screen-coordinate math in `SelectionOutlineLayer` (`span.screenX`), `EdgeLayer` (`screenX − originY`), `GuideOverlayLayer` (`x*zoom + pan.x`), and `DrawingsLayer` (per-point `canvasToScreen`) **deletes** — the transform does it on the GPU.

**Screen-constant chrome exception.** Selection outlines, resize handles, and 1px borders must stay constant width in screen pixels, not scale with zoom. Standard fixes: SVG `vector-effect: non-scaling-stroke` (already used for shapes), counter-scale (`1/zoom`) on DOM handles, *or* keep selection chrome in a non-transformed overlay that projects entity bounds through the **live renderer camera** (how Figma/tldraw render selection UI). Either is fine; the invariant is that the projection source is the local camera, never a main round-trip.

### 3. The layout payload is viewport-independent

`setPan`/`setZoom` stop calling `markDirty('canvas')`. `buildCanvasLayoutData` no longer runs on pan/zoom, and `screenX/screenY/screenWidth/screenHeight` are removed from the entity payload and from the per-kind builders — entities ship canvas coordinates only. Anything main still needs in screen space (hit-test overlay rect, comment-hover bbox) computes on demand. The payload flows only on content/selection/order change; `aboveView` gets rAF batching and stable per-kind slices so even content payloads stop re-filtering the whole entity array.

### 4. Main is a follower for the native views

The renderer forwards absolute `{ pan, zoom }` to main, rAF-throttled and fire-and-forget — only so main can (a) position the WCVs via the existing unconditional `setBounds` loop and (b) persist viewport to the Y.Doc. Main does **not** echo the viewport back during a gesture, so there is no round-trip and no drift (main is a pure function of the forwarded camera). Programmatic camera moves (focus, zoom-to-fit, undo-to-tab) originate in main and *push* a target camera to the renderer, which animates its local camera via rAF; the existing `cameraTransitionStartedAt` field is the hand-off. This replaces the main-side `setInterval` camera tween in `viewport-control.ts` with a renderer rAF animation.

### 5. Zoom-settle device emulation (no rasterization)

A live WCV cannot re-layout crisply at 60fps. Without freezing to a bitmap, zoom has exactly one honest behavior for live pages, and it is the browser/Figma behavior: **smooth approximate scale during the gesture, one crisp re-raster on settle.**

- **During an active zoom:** do not re-emulate. Track the zoomed rect via `setBounds` so the view's existing raster scales (Chromium composites the view texture at the new size). Slightly soft mid-gesture, geometrically correct.
- **On settle** (~120–150 ms debounce after the last zoom tick): call `enableDeviceEmulation({ scale: finalZoom })` once. One reflow per gesture instead of one per 16 ms.
- Same treatment for component views (the second emulation loop in `layout-engine.ts`).
- Optionally re-raster DOM entity text crisp on the same settle (bump font-size once) so entity text and page content share one behavior: everything is a smooth scale during the gesture and re-rasters crisp together on settle.

### 6. Rasterization freeze during gesture — deferred final step, only if needed

The one way to make *live pages* GPU-smooth during a gesture (not just approximate-scaled) is to freeze them: at gesture-start, `capturePage()` each visible page into a DOM element that rides the camera transform, park the live WCV off-screen, and re-emulate + restore on settle. Then pan *and* zoom of the entire canvas — pages included — become 100% compositor operations with zero main work during the gesture, and cost becomes independent of page count.

This is **explicitly deferred** (Phase 5, may never be built) because:

- It re-introduces the ADR-0014 layering problem: a page snapshot must occupy the page z-band, which DOM cannot natively enter. Workable — snapshots ride `bgView`'s transformed container (above the border divs, below `aboveView` — the correct band), with live WCVs hidden for the gesture — but it is genuine coordination, not a free transform.
- `capturePage()` is an async GPU readback (a few ms per page); at gesture-start across many pages it needs pre-warming/pipelining or it spikes.
- Live content (video, animation) freezes for the gesture duration — imperceptible for arrange-gestures, but a real behavior change.

Phases 1–4 already make the DOM fully GPU-composited and cut the native cost to one reflow per zoom-settle. Phase 5 is only warranted if profiling after Phase 4 still shows page-count-dependent jank during gestures. Recorded here so the option is designed, not discovered.

## Consequences

**Enables:**

- Pan and zoom of all DOM become one compositor transform update — no rebuild, no sort, no serialize, no React re-render. Cost independent of entity count on the DOM side.
- Zoom gets a live DOM shimmer (today it has none) and stops reflowing every native page per tick.
- Zero input→transform latency (Phase 4) — Figma/tldraw-grade camera feel.
- One coherent zoom behavior across substrates: smooth scale during the gesture, crisp re-raster on settle, for both pages and DOM entities.
- Hit-testing and coordinate mapping read the live local camera instead of a lagged payload — more correct, not just faster.

**Replaces:**

- Per-entity main-computed `screenX/screenY` on DOM layers → canvas coordinates + one renderer transform.
- The `translate3d(panOffset)` live-pan *patch* (#264) → the primary camera transform.
- `useScenePanOffset` (pan-only nudge offset) → the renderer's own camera state (Phase 4) or a pan+zoom nudge (Phase 1).
- The main-side `setInterval` camera tween → renderer rAF animation.
- Per-tick `enableDeviceEmulation` → one emulation per zoom-settle.

**Costs / tradeoffs:**

- **The one visual concession of avoiding rasterization:** live pages are approximately scaled (slightly soft) *during* a zoom gesture and snap crisp on settle. This is the browser pinch-zoom / Figma-embedded-iframe behavior and is the correct tradeoff for a design tool. There is no third option — either soft-during-zoom (this ADR) or freeze-to-bitmap (Phase 5).
- Renderer refactor of both `bgView` and `aboveView` scene layers (net deletion of coordinate math).
- Authority flip (Phase 4) touches the focus/camera-animation machinery in `viewport-control.ts`; sequenced last and de-risked by Phases 1–3.
- Screen-constant chrome needs explicit counter-scaling / camera-projection instead of falling out of main-computed screen coords.

**Preserved (explicitly not changed):**

- `LAYER_STACK`, `applyStack()`, and the `bgView` / WCV / `aboveView` banding (ADR 0014). `layoutAllViews()` still runs every pan/zoom tick (the `setBounds` loop must, to move the WCVs); only the payload build/send is removed. Z-order reconciliation is independent of the viewport and untouched.
- All user-facing functionality — this is a rendering-path change, not a behavior change.

**Out of scope:**

- **Entity drag** carries the identical waste (`canvas-drag-*` → `requestLayout` → full rebuild per tick). The same principle applies (ship a minimal delta, move the entity live under the transform), and it falls out of this architecture cheaply, but it is a separate change.
- Cross-surface visual stacking (ADR 0014 out-of-scope) — unchanged.
- The `cursorOverlayWindow` presence-cursor rendering model beyond joining the viewport channel (Phase 1).

## Migration plan

Five slices, each an independently shippable PR (mapping to one AFK-local step each). Phases 1–3 deliver essentially all the felt smoothness; Phase 4 is Figma-parity; Phase 5 is the deferred rasterization escape hatch.

**Phase 1 — De-dirty the viewport + batch the renderer (main stays authoritative).**
The minimal high-leverage cut. Keep main as the pan/zoom authority, but:
- `setPan`/`setZoom` stop calling `markDirty('canvas')`; `buildCanvasLayoutData` stops running on pan/zoom.
- Extend `viewport-nudge` to carry and apply zoom (not just pan), and drive the *whole* scene transform from it in both renderers — including the layers currently pinned to `screenX` (temporarily projected renderer-side from the nudge).
- rAF-batch `aboveView`'s `onLayoutUpdate` (match `bgView`, #265); hoist stable per-kind slices so `SelectionOutlineLayer` stops re-filtering the full array.
- `cursorOverlayWindow` joins the `viewport-nudge` channel so cursors track pan without the heavy payload.
- *Result:* pan and zoom stop rebuilding/serializing/re-rendering. Biggest single win; low risk (authority unchanged). Largely resolves #265. HUD `build`/`hitch` should drop sharply during pan.

**Phase 2 — One transform, delete screen coordinates.**
- Move every remaining `aboveView` and `bgView` layer into one canvas-space transform container (selection outlines, edges, guides, drawings, page chrome).
- Delete `screenX/screenY/screenWidth/screenHeight` from the entity payload and the per-kind builders; entities ship canvas coords.
- Implement screen-constant chrome (non-scaling stroke / counter-scale / camera projection).
- Add GPU hygiene: `will-change: transform` on the scene containers so pan/zoom composite without paint (one compositor layer per renderer).
- *Result:* the coordinate math deletes; zoom shimmers on the DOM; the payload is fully viewport-independent.

**Phase 3 — Zoom-settle device emulation.**
- `setBounds`-only during an active zoom; `enableDeviceEmulation` once on a ~120–150 ms settle debounce, for pages and component views.
- Optional: re-raster DOM entity text crisp on the same settle.
- Cull hysteresis so edge-hovering pages don't toggle bounds every pan tick.
- *Result:* live pages stop reflowing per zoom tick; the worst zoom cost is gone. Orthogonal to 1–2.

**Phase 4 — Renderer-owned camera (zero-latency).**
- Move pan/zoom integration into the renderer; apply the transform in the input frame; forward absolute `{ pan, zoom }` to main rAF-throttled for `setBounds` + persist.
- Main→renderer camera *push* for programmatic moves via `cameraTransitionStartedAt`; renderer rAF animation replaces the main `setInterval` tween.
- Delete the two stacked 16 ms timers on the viewport path (the drag-IPC coalescer and the `requestLayout` viewport debounce).
- Memoize the scene build so content-change payloads don't re-map + re-sort from scratch.
- *Result:* zero input→transform latency; the true Figma/tldraw camera. De-risked by Phases 1–3.

**Phase 5 — Rasterization freeze during gesture (deferred, build only if needed).**
- Per decision §6. Snapshot visible pages into `bgView`'s transformed container at gesture-start, park live WCVs, re-emulate + restore on settle.
- Gate on profiling: only if page-count-dependent gesture jank survives Phase 4.

## Tests

- **Unit:** camera → transform projection is exact (canvas↔screen round-trips within 0.5 px at representative zooms); screen-constant chrome holds constant screen width across zoom levels.
- **Smoke:** during a pan of a ~20-page canvas, `buildCanvasLayoutData` does not run (instrument the build counter / `buildMs`) — the scene list is reused (the #265 acceptance criterion, now satisfied structurally).
- **Smoke:** chrome, borders, selection outlines, edges, and entity bodies stay pixel-locked to their pages/entities through pan → zoom → entity edit → undo/redo (the #257 lag, now impossible — DOM and WCVs read the same camera).
- **Smoke:** stack order (ADR 0014) is unaffected — two overlapping pages resolve clicks and paint correctly after the migration; `LAYER_STACK` order in `win.contentView.children` is unchanged by a pan/zoom.
- **Smoke:** a programmatic camera move (focus / zoom-to-fit) animates and lands at the same final camera as before (Phase 4 authority flip is behavior-preserving).
- **Perf (HUD-assisted, manual):** frame `hitch` during sustained pan and zoom on a 20-page canvas stays within budget; `build` is flat (not spiking) during movement.
