# ADR 0038 — Offscreen GPU-texture compositing for live pages

**Status:** Proposed — spike validated the perf hypothesis; production build-out (agent routes, capability parity, pool tuning) is unstarted. Implementation landed on branch `claude/offscreen-rendering-canvas-c81a4k`; the Post-build validation checklist below is the acceptance gate for Accepted.
**Date:** 2026-09-10 (revised 2026-09-12 — animated-content spike reversed the mode recommendation; see "Animated-content fidelity" below)
**Related:** [ADR 0023 — Renderer-owned camera and GPU-composited pan/zoom](./0023-renderer-owned-camera-gpu-panzoom.md) (rejected; its postmortem named the live-page substrate, not the transform, as the hard part), [ADR 0037 — A pan does not freeze its pages](./0037-pan-does-not-freeze-its-pages.md) (the freeze/park machinery this ADR makes unnecessary **on the desktop target only** — a browser tab cannot offscreen-render a cross-origin iframe, so a web build has no shared-texture path and gesture-time rasterization is its only lever; ADR 0037's measurements are what a web version re-derives from, not its deleted files), [ADR 0014 — Canvas stack order](./0014-canvas-stack-order.md) (the layering constraint this ADR removes rather than preserves), [ADR 0036 — Diffed runtime store](./0036-diffed-runtime-store.md) (established pan is not a page-count problem for the *DOM* side — this ADR addresses the *compositor* side that remains).
**Origin:** `docs/one-live-view-research.md` (the assessment that scoped this spike against Electron 43 source) and the Offscreen Rendering Lab (`View → Open Offscreen Rendering Lab`, `src/main/osr-lab/`, `src/renderer/osr-lab/`), built on branch `claude/offscreen-rendering-canvas-c81a4k`.

## Context

Specular's canvas composites live pages as native `WebContentsView`s (WCVs) positioned via `setBounds` under `bgView`/`aboveView` (ADR 0014's layering constraint — a native view cannot sit inside a single renderer's DOM z-order, so chrome above and below pages requires two separate renderer surfaces sandwiching N page WCVs). Prior perf work (ADR 0036, the perf log's Exp G dot-grid fix) established that the *DOM* side of pan/zoom is no longer page-count-dependent. This ADR is about the cost that remained: **native compositor backpressure from presenting N separate WCV surfaces per frame.**

`docs/one-live-view-research.md` §1 frames the three possible shapes for a live page:

- **Shape A** (offscreen at rest, swap to a fresh live WCV on enter) — loses DOM/JS/scroll state on swap. Ruled out.
- **Shape B** (every page offscreen, always; the "live" page is the one receiving forwarded input) — no swap, no native view at all.
- **Shape C** (current architecture: every page a live WCV, textured only during gestures/parked) — what the last three months of perf work built.

The research doc scoped two spikes to decide between B and C: **Spike 1** (the texture tier: does shared-texture OSR actually perform?) and **Spike 2** (can the *interactive* page be offscreen too, or does B need a live-WCV carve-out for text editing?). Both are implemented as one prototype window, the Offscreen Rendering Lab.

## Spike results (measured 2026-09-10)

**Method:** the app's own scripted pan/zoom perf test (`/perf/pan-zoom/run`, all-process Chromium trace, summarized per `docs/perf-tracing.md`) against a 9-page synthetic fixture (the "Perf test" tab), compared to the lab's own benchmark (`labBenchmark.ts`, mirroring the same four gesture profiles) at matched page counts.

### Real canvas (current shape-C architecture) — GPU-process saturation scales badly with page count

| Pages | `CrGpuMain` busy (of wall-clock pan duration) |
|---|---|
| 9 | 1913ms / 3102ms — **62%** |
| 20 | 3140ms / 3222ms — **97.5%** |

Top events at both counts are dominated by `Graphics.Pipeline`, `SkiaOutputSurfaceImplOnGpu::SwapBuffers`, `ImageTransportSurfaceOverlayMac::Present`, and `CALayerTreeCoordinator::ApplyBackpressure` — native CALayer presentation, not JS or layout. `Layout / style recalc` stayed cheap at both counts (313ms / 635ms, ~10–20% of duration), and `buildStats.n: 0` in every run — **zero `setBounds`-loop calls happened during either pass.** The cost is the GPU process synchronizing N separately-composited native surfaces per frame, a cost that has nothing to do with what changed since ADR 0036 and everything to do with page count itself.

### osr-lab, shared-texture mode — flat through 20 pages, first cracks at 40

| Pages | drawFps (all 4 phases) | meanFrameMs | maxFrameMs | longFrames |
|---|---|---|---|---|
| 9 (bitmap-JPEG baseline) | 120.0 | 8.33 | 9.3–10.6 | 0 |
| 20 | 119.9–120.2 | 8.32–8.34 | 9.4 | **0** |
| 40 | 119.5–120.3 | 8.32–8.37 | up to **16.1** | 2 (slow-pan), 1 (slow-zoom) |

At 20 pages the shared-texture lab is indistinguishable from 9 — fully pinned to the 120Hz budget, zero long frames across all four gesture profiles. At 40, the first real ceiling shows: two long frames in `slow-pan`, and `maxOutstandingTextures` pegged the pool cap (`MAX_OUTSTANDING_TEXTURES = 6` in `osr-lab-pages.ts`) with 105 `framesWithoutTexture` in aggregate (vs. 26 at 20) — not yet visible jank, but the first sign the shared-texture pool needs tuning past ~20–40 pages.

Notably, even the **bitmap-JPEG baseline** (no GPU shared texture, `toJPEG(80)` on main + decode in renderer — the same pipeline the current zoom freeze uses) matched this ceiling-free profile at 9 pages. **This result is real but incomplete: every page tested here (Wikipedia, GitHub, MDN, HN, docs sites) is effectively static — no continuous repaint, so `paint` events are rare and JPEG's per-frame encode cost never gets stressed.** It does not generalize to animated or actively-interactive content — see below.

**Caveat:** the real-canvas trace used 9/20 synthetic static pages; the lab runs used 9/20/40 real websites. Not a perfect content match, and the real-canvas trace is all-process (briefly includes other open windows), so 97.5% is a slight overestimate of that tab's isolated cost. The *mechanism* (`ApplyBackpressure` scaling with WCV count) is a structural property of shape C independent of page content, so the diagnosis is trusted even if the exact percentage shifts.

### Animated-content fidelity — the actual differentiator between the two texture modes (spiked 2026-09-12)

The static-page result above was read, in the original version of this ADR, as "both modes are flat, so ship the simpler one." That conclusion doesn't survive contact with actually-animating content, which is the case that matters for a live-page canvas. Re-run with 20 pages of a synthetic `requestAnimationFrame`-driven page (a continuous counter + CSS-animated element, uncapped, targeting the display's native ~60fps) over a fixed 6-second window, `MAX_OUTSTANDING_TEXTURES` raised 6→9 beforehand:

| Mode | Frames delivered per actively-painting page (6s window) | Effective rate | Per-frame cost |
|---|---|---|---|
| Shared-texture | 360–361 (16 of 20 pages; 4 were culled off-viewport by `stopPaintingOffscreen`) | **~60fps — full native rate, zero drops, zero pool pressure** | — |
| Bitmap-JPEG | 27–50 (all 20 pages actively painting) | **~4.5–8.3fps** | 8.1–9.5ms/frame, synchronous on the main process |

Bitmap-JPEG's effective animation rate collapsed to roughly **1/8th of real-time**. The arithmetic explains why: `handleBitmapPaint`'s `image.toJPEG(80)` call runs synchronously on the main process for every paint. Twenty pages animating at 60fps each want ~20 encodes every ~16.7ms; at ~8.8ms/encode that's ~176ms of serialized main-thread work demanded per 16.7ms window — roughly 10x more than the main process has time for, so the achieved rate falls until it fits. Shared-texture has no equivalent serialization point (the GPU handle passes through without a main-process encode step), and with the pool cap raised to 9 it never came close to pressure (peak 3 outstanding across 16 fully-animating pages) — the pool-pressure signal seen at 40 *static* pages with the old cap of 6 does not reappear here even under much heavier real animation load.

**This reverses the mode recommendation.** JPEG-first was the right read of the static-page evidence; it is the wrong read once animated/interactive content — which is most of what a "live" page canvas exists to show — is in the mix.

## What this would delete (contingent on Spike 2 — see Open questions)

If Spike 2 passes fully, `docs/one-live-view-research.md` §5 names the payoff directly: *"a one-renderer canvas with no `setBounds` loop, no device emulation, no freeze machinery, and no layering bands — a large deletion."* Concretely, against the current architecture:

- **`LAYER_STACK` / `applyStack()` reconciliation** (`src/main/runtime/layer-stack.ts`) — the `win.contentView.children` diff/patch that exists solely to interleave N page WCVs between `bgView` and `aboveView`. With pages as canvas draws, there is nothing native to reconcile into the stack.
- **The `bgView`/`aboveView` split itself.** `docs/interaction-layer.md` §2 states the goal as "minimal stacking surface: three WCVs in the stacking region (bgView, aboveView, liveViews)" — with `liveViews` at zero, the stacking region has no reason to exist; one renderer can own grid, borders, pages, entity bodies, and selection chrome as one draw surface. `docs/interaction-layer.md` §2 Non-Goals already flags this as a separate track ("A full offscreen-texture compositor for live pages") — that line needs to be retired, not just satisfied, if this graduates.
- **The per-page `setBounds` loop** in `layoutAllViews()` — confirmed already unnecessary for the *pan* path (`buildStats.n: 0` in both traces above); a full graduation removes it for page creation/resize/z-order too.
- **Device-emulation re-raster machinery** — `enableDeviceEmulation({ scale })` and the zoom-settle debounce dance that ADR 0023's postmortem identified as the real cost center. A texture page just scales in the canvas draw call.
- **The freeze/park system** — `zoom-snapshot-freeze.ts`, `page-freeze.ts`, `drag-freeze.ts`, `useFrozenPagesState.ts` / `useFrozenPageBitmaps.ts`, and ADR 0037's warm-park/hidden-park distinction. (`chromeItemDraw.ts`'s `drawItemSnapshot` stays — it draws the live texture instead of a frozen bitmap.) This machinery exists to *simulate* shape B during gestures only; if every page is already a texture all the time, there is nothing to freeze or unpark.

  That premise holds where shape B is reachable, which is the desktop app. A web build's live content is sandboxed cross-origin iframes (CONTEXT.md, "Sandbox origin" and "HTML prototyping loop"), and a browser tab has no offscreen-render path across that boundary — `page` nodes have no pixels there at all without Cloudflare Browser Rendering. Rasterizing during a gesture is not a simulation of anything on that target; it is the only lever. The code still goes — git history holds it, and what a web version needs is ADR 0037's measurements and its reasoning about which gestures the trade wins on, not these files.
- **Most of `FocusReconciler`'s WCV-focus-stealing workarounds** (`docs/interaction-layer.md` §4.4, mitigating Electron #42578 and #22201 across N pages) — shrinks to at most one hidden `BrowserWindow`'s focus state (the single "entered" page), not N competing native views.
- **The input gate** (`shouldGateBeOpen`, `aboveView` visibility toggling between capturing canvas gestures and letting native page input through) — a single canvas surface routes all input uniformly through one hit-test, the pattern `osr-lab`'s own `hitTestPages`/`canvasXY` (`LabCanvas.tsx`) already implements.
- **The "Bitmap compositor" special case** (`docs/interaction-layer.md` §4.7) — currently scoped as "a future memory/CPU optimisation" for *inactive* pages only, not load-bearing. It becomes the universal default rather than a special case, and the "is this page active enough to deserve a real view" branch disappears.

Not affected: `cursorOverlayWindow` (agent-presence cursors; unrelated to page compositing), `toolbar`/`sidebar`/`devtools` WCVs (independent lifecycle, not part of the pages/chrome banding problem).

## Open questions before graduation

These are gaps identified while assessing this spike. Items 1 and 2 were spiked directly against a running osr-lab page (2026-09-10, raw CDP over the app's `--remote-debugging-port`, no new plumbing built) to check the claims below before they landed in this ADR; 4 remains open.

1. **Agent HTTP routes are unbuilt for osr-lab pages, but the underlying mechanism is confirmed working.** `src/main/routes/pages.ts`'s DOM snapshot, selector query, and CDP-proxy routes depend on a preload (`page-content`) injected into every page; `osr-lab-pages.ts`'s `createPage` sets none, so there is no route today. Spiked directly: attached over CDP to a live osr-lab page's `webContents` (exposed automatically via Electron's global remote-debugging port even though the `BrowserWindow` is `show:false`) and ran `Runtime.evaluate`, `DOM.getDocument`, and `Accessibility.getFullAXTree` — all three returned correct, complete results with zero errors. Since `specular snapshot`'s agent-facing accessibility tree is CDP-`Accessibility`-domain-based (ADR 0019), not OS-level VoiceOver, this is the capability that actually matters for the product, and it's unaffected by offscreen rendering. Remaining work is wiring (a preload + the existing route logic pointed at osr-lab's page registry), not a feasibility question.
2. **Spike 2's interactivity checklist, updated:**
   - Confirmed working (prior lab sessions): in-page popup widgets (date/color pickers, autofill — own texture), cursor mirroring, `capturePage()`, DevTools, hover/scroll/click/drag-select.
   - **Corrected finding: `<select>` is not a popup texture on macOS.** It is an external OS popup menu, and Electron's offscreen `WebContentsView` implements no `ShowPopupMenu` — Blink's `should_disable_external_popups` preference, which would force it to render in-page like other browsers' headless/offscreen modes, is not exposed by Electron. So a native `<select>` menu never reaches the offscreen paint pipeline at all; it is not a case of "arrives as a popup texture with no position" like a date picker, it does not arrive. Mitigation: an in-page fallback dropdown drawn by the page preload for menulist `<select>`s (see Implementation).
   - Confirmed working (spiked 2026-09-10, via CDP): **file upload** — `Page.setInterceptFileChooserDialog` correctly fired `Page.fileChooserOpened` on a real click, and `DOM.setFileInputFiles` successfully set the input's file list (verified via `Runtime.evaluate` reading `files[0].name` back). **Permission prompts** — `navigator.permissions.query({name:'geolocation'})` resolved cleanly (`state: "granted"`, Electron's default with no `setPermissionRequestHandler` registered — same default any BrowserWindow gets, not OSR-specific) with no hang.
   - **Closed: printing.** `Page.printToPDF` is not implemented in Electron's CDP debugger domain, so printing goes through the main-process `webContents.printToPDF()` API directly, exposed as `POST /pages/:id/print-pdf` and the `print-pdf` CLI verb.
   - **Closed: drag-out.** The page's own `dragstart` arms a payload in main and aboveView drops it on release outside the page (image → image entity, link → page, text → sticky); a release inside the page is a plain mouse-up and in-page drops are lost — the accepted carve-out.
   - Confirmed limited (unchanged): IME composition (commits via `Input.insertText`, no live composition UI — Chromium's OSR widget host makes `Focus()`/`TextInputStateChanged` no-ops).
3. **Native OS accessibility (VoiceOver/AXTree) is not currently supported in canvas mode at all**, so this is not a regression gate — noted for the record, not blocking.
4. **Shared-texture pool sizing** — spiked 2026-09-12. Raised `MAX_OUTSTANDING_TEXTURES` 6→9 (Chromium's own OSR frame pool caps at 10/page, so 9 keeps minimal headroom) and re-ran under real animated load: 20 pages, 16 of them actively repainting at ~60fps for 6 seconds. Peak outstanding textures across all pages: 3 — nowhere near the cap, zero pool drops, zero `framesWithoutTexture`. The 40-*static*-page cracks seen at cap=6 in the original spike do not reappear here even under substantially heavier real animation load. Not fully closed — the original 40-page test should be re-run at cap=9 to confirm the static-page ceiling also moves — but the signal reverses from "needs tuning" to "6 was simply too conservative; 9 clears real load with margin."

   **Correction (2026-09-12): the cap is back at 6.** The evidence above does not support 9. `framesWithoutTexture` counts `paint` events that arrive with no texture, before the cap is checked, so the cap cannot cause it; an `invalidate()` on an unchanged page emits exactly that. Drops the cap causes are `framesDroppedForPoolPressure`, which the 40-page run did not report. `maxOutstandingTextures` is a per-page peak, so reaching 6 points at canvas-bg falling behind, not a small pool. The animated run peaked at 3, which 6 also clears. A higher cap only lets more stale frames queue when the renderer is behind. Still open: re-run 40 static pages and read `framesDroppedForPoolPressure`.

## Decision

Proceed to build production-shaped offscreen-texture compositing for live pages, replacing per-page `WebContentsView`s with offscreen `BrowserWindow`s composited onto one canvas surface — **starting from shared-texture mode as the initial production target, not bitmap-JPEG.**

This reverses the original (2026-09-10) call. The static-page spike showed both modes flat and recommended JPEG-first on complexity grounds; the animated-content spike (2026-09-12) showed bitmap-JPEG's synchronous per-frame main-thread encode collapses to ~1/8th of native frame rate under realistic concurrent-animation load, while shared-texture sustains full native rate with no pool pressure once `MAX_OUTSTANDING_TEXTURES` is raised from its overly-conservative default of 6 to 9. A live-page canvas exists specifically to show pages that animate or are being interacted with — that is not an edge case to defer, it is close to the point of the product — so the mode that only works well on static content is not an acceptable production default. The operational complexity JPEG-first was chosen to avoid (`sharedTexture.importSharedTexture` lifecycle, pool sizing, release-latency tracking) is real but now clearly worth paying: it is what actually delivers working page animation at page counts the app needs to support.

This does not yet decide shape B (every page offscreen, including the interactive one) vs. a hybrid — that depends on closing open question 2 above. If IME/drag-out/upload prove unacceptable, the fallback named in the research doc still holds: shape C plus the texture tier, keeping today's input model but removing the `setBounds` cost this spike measured.

## Implementation

Shape B, built. Every page is a hidden offscreen `BrowserWindow` (`page-host.ts`) sized to its CSS viewport, painting with `offscreen: { useSharedTexture: true, deviceScaleFactor }`. Each painted frame is imported as a GPU shared texture in main and sent to canvas-bg's main frame, where a cap of 6 outstanding textures per page (`MAX_OUTSTANDING_TEXTURES`; see the correction under open question 4) bounds how far the renderer can fall behind before a frame is dropped.

- **Texture surface.** `PageTextureSurface.tsx` draws every presented page's latest frame at the camera-projected content rect, clipped to the page's corner radius, on a dedicated canvas mounted above `ChromeCanvasSurface` and below `aboveView`. `usePageFrames.ts` holds the latest bitmap (and latest popup bitmap) per page, replacing and closing the previous one as frames arrive; the focused page draws last so it wins any overlap. The layout pass decides whether a page's host paints at all — `isPagePresented` plus the presentation policy skip pages a focus session hides, so an offscreen host that nobody can see stops costing frames.
- **Input path.** aboveView forwards pointer and wheel events to the keyboard-target/hovered page via `sendInputEvent` (`page-input-forwarding.ts`), mapped from window space into the page's own CSS viewport through its camera-projected rect. Keys are different: an offscreen widget host's `Focus()` is a no-op, so aboveView keeps OS keyboard focus permanently and runs a hidden keyboard sink (`usePageKeyboardForwarding.ts`) that forwards unclaimed keys to the target page over CDP `Input.dispatchKeyEvent`, with IME composition committing whole via `Input.insertText` (no live composition UI). `page-focus-emulation.ts` makes the target page believe it has focus via `Emulation.setFocusEmulationEnabled`, so caret and active-selection color render correctly. Main's binding dispatcher evaluates with the target page's context, so page-scoped bindings (Escape → exit interactive) still fire before a key is forwarded. The inspect tool forwards hover/press to whichever page is under the pointer, not only the keyboard target.
- **Popup anchoring.** A popup widget (`<select>`, autofill, date/color picker) arrives as its own texture with no reported position. Its anchor is the page's focused element, queried through the page preload; the popup hangs below-left of that element like Chromium's own pickers, flipped above when there's no room. Electron sends no close event for a popup, so closing is inferred from paint order: a page paint with no popup paint after it, past a short grace window, is read as the popup having closed.
- **Drag-out.** An offscreen page can't start a native OS drag, so the page's own `dragstart` (caught by the page-content preload) arms a payload in main (`page-drag-out.ts`); aboveView, already forwarding the held-button pointer, drops it on release outside the page's content — an image becomes an image entity, a link becomes a page at the source's preset, text becomes a sticky note. A release inside the page is a plain mouse-up; in-page drops are lost.
- **Print.** `webContents.printToPDF()` — Electron's CDP debugger domain has no `Page.printToPDF` — exposed as `POST /pages/:id/print-pdf` and the `print-pdf` CLI verb.
- **Deleted:** the per-page `setBounds` loop, warm/hidden parking, device-emulation re-raster and its zoom quantizer, the zoom and drag snapshot freezes, the `setVisible` override, CDP coordinate compensation, and the aboveView input gate (`shouldGateBeOpen` — aboveView now always covers the canvas region).
- **Left for follow-up, not because it's blocked:** merging `bgView` and `aboveView` into one renderer (this ADR's "what this would delete" section named the split as deletable; it's unnecessary now but harmless), and component-entity views (`component-page-factory.ts`), which remain native `WebContentsView`s in the window's child list.

## Consequences

**Enables:**
- Native-compositor cost becomes decoupled from page count on the canvas draw path (measured flat 9→20 pages; first pressure only at 40 static pages with the old pool cap, not reproduced at cap=9 even under animated load).
- Full native frame rate for animated/interactive page content at realistic page counts, which bitmap-JPEG cannot deliver at more than a handful of simultaneously-animating pages.
- The large deletion enumerated above, if Spike 2 closes clean.

**Costs / open work:**
- `bgView` and `aboveView` remain two renderers; merging them (named as deletable in "What this would delete") is a follow-up, not a blocker.
- Component-entity views (`component-page-factory.ts`) are still native `WebContentsView`s in the window's child list, outside this migration.
- Shared-texture's operational complexity (`sharedTexture.importSharedTexture` lifecycle, pool sizing, release-latency tracking) is now committed to, not deferred — this is real production surface area to build and maintain, accepted because the animated-content evidence shows it's load-bearing, not optional.
- Re-run the original 40-*static*-page pool-pressure check at cap 6, reading `framesDroppedForPoolPressure`, before treating pool sizing as closed.
- The macOS `<select>` fallback dropdown (page preload, in parallel with this branch) needs its own real-site validation — see Post-build validation.
- The manual Post-build validation checklist below is unrun; it is what moves this ADR from Proposed to Accepted.

**Not resolved by this ADR:** whether shape B fully replaces shape C, or lands as a hybrid — in practice, shape B shipped and the hybrid fallback was not needed.

**Cost model, measured 2026-09-19.** Electron 43.2, on the animated perf canvas (30 animating and 28 static pages) at zoom 0.1, with 24 animating pages on screen at 15fps, which is 360 frames/s. Every frame pays a fixed cost (capture handshake, one IPC hop through main, one shared-texture import) and two costs that grow with its pixels (the capture blit and the bitmap copy in canvas-bg). Static pages are damage-driven and cost nothing at rest.

| Share of total CPU | Main | GPU process | canvas-bg | Pages | Total |
|---|---|---|---|---|---|
| Whole pipeline | 21% | 75% | 17% | 33% | 146% |
| Capture only (main drops every frame) | 10% | 43% | 0% | 32% | 87% |
| Transfer + copy (the difference) | 11% | 32% | 17% | 1% | 59% |

So about 60% of the cost is Chromium's offscreen capture, which only frame rate and texture size move, and about 40% is this design's hand-off and copy. Drawing the textures onto the item surface is not where the time goes. With every page draw disabled, total CPU moved 2–5% at zoom 0.1, 0.35 and 0.6. That rules out three ideas, which should not be retried without new evidence:
- Latest-wins copies, where canvas-bg holds the newest frame and copies it at paint time. Once repaints are paced, 0% of copied frames go undrawn, so there is nothing to skip.
- Dirty-rect repaints and a canvas per page. Both only save surface draws, which are the 2–5%.

What landed from the measurements:
- `stopPainting()` only stops texture delivery; a culled animating page kept compositing 60fps of discarded frames (~110% GPU with every page off-screen). Culling drops the host frame rate with painting (`applyPainting` in `page-host.ts`).
- **Frame-rate LOD.** A page earns frame rate by its on-screen scale (`page-frame-rate.ts`). It gets 60fps from 0.44, 30fps from 0.25 and 15fps below, with sticky boundaries. Agent-driven and focus-session pages stay at full rate.
- **Texture-size LOD.** Once the camera settles, the host view shrinks to 1, 0.5 or 0.25 of CSS size (`page-texture-scale.ts`), never below the on-screen scale. CDP's `Emulation.setDeviceMetricsOverride` holds layout at full width. `enableDeviceEmulation` cannot do this job, because it leaks the small viewport to a reloading document. Input is scaled into view px; agent screenshots hold full resolution. At zoom 0.1 with 60 pages on screen, GPU-process CPU went from 155% to 77%. An earlier reading on this date that cost is per-frame only was wrong. Emulation alone does not shrink the offscreen texture. The view resize does.
- **Paced repaints.** Frames carry their tier rate, and the item surface repaints at the fastest rate arriving instead of on every arrival (`useFramePaintPacing.ts`).
- **An entity drag wakes only the pages it moves.** Every presented page used to paint for the length of any drag. Holding one page with the rest off-screen cost 473% total CPU before and 60% after.

**Postmortem, blank pages after a zoom-out (2026-09-19).** A culled page woke at whatever texture size it last had, and a zoom-out from 0.7 to 0.1 woke ~55 pages at full size for the length of the shrink settle wait. canvas-bg could not copy that many 2560×1600 textures in time, so `sendSharedTexture` timed out after its fixed 1s (about 160 failures per run). Main released each timed-out texture. canvas-bg copied the recycled buffer late and drew a transparent or black page, and one static page stayed that way for 4.7s. A timed-out transfer also never reports its references released, so the host's outstanding count leaked up to its pool allowance and the host dropped every later frame. That page was stuck for good and its transition never ended. The fix is at the cause. A page wakes at the scale it is owed (`setPainting` → `reconcileTextureScale(true)`, and the layout pass grades scale before painting), a failed transfer releases and uncounts its texture and asks for another frame, and the in-flight check runs ahead of the pool check. There were two wrong turns on the way. Waiting for outstanding textures before the view resize changed nothing. Refusing blank frames in canvas-bg caught only some of them, and it was removed. `GET /perf/page-hosts` reports per-host transfer failures, outstanding textures and transition state. Look there first for any blank or stuck page.

## Post-build validation (manual, run after the implementation lands — not pre-build spikes)

The items above were spiked before deciding to proceed. The items below surfaced in review *after* that decision and are deliberately not gating build-out — they're the checklist for confirming the shipped implementation behaves as expected, not further pre-build research. Run each manually against the real production build:

- **Resolution/zoom fidelity.** Confirm text stays sharp across zoom levels once device-emulation re-raster machinery is gone — a texture captured at one resolution and scaled in the canvas draw call risks going soft the way a bitmap does when scaled past 100%, unlike today's per-zoom re-raster.
- **Startup cost at real page counts.** Open a `.canvas` file with 100+ saved pages. Shape B is every page offscreen, always — confirm this doesn't mean instantiating 100+ hidden `BrowserWindow`s at launch if pages were previously lazy-loaded near the viewport.
- **GPU memory under load**, not just frame time — sustained high page count + concurrent animation, watched for VRAM pressure the fps/ms-per-frame traces above wouldn't surface, especially on integrated-GPU Macs.
- **GPU process crash blast radius.** Force a GPU process crash/reset with several pages open; confirm whether textures fail/recover independently per page or all pages blank simultaneously (a shared pool serviced by one GPU process could turn an isolated failure into a whole-canvas outage).
- ~~**Interaction with ADR 0035's idle-freeze policy**~~ — **Resolved 2026-09-12: they did not compose.** `Page.setWebLifecycleState('frozen')` froze an offscreen page but `'active'` never resumed its compositor, and every document loaded afterwards started hidden; focus emulation on the keyboard-target page masked it. The idle lever for page hosts is now `setFrameRate(1)` + `stopPainting()` (`PageHost.setIdle`), which composes with culling as `painting && !idle`. Full evidence in ADR 0035's offscreen postmortem.
- **CI testability.** Confirm what's exercisable in `test:integration` (no real Electron) versus what only runs in `test:boot` (pre-release only) — shared-texture behavior needs a real GPU, which may push runtime-mutator coverage for this path out of the tier your test contract expects it in.
- **Video and nested-GPU content.** Every spike page was static or a synthetic `requestAnimationFrame` counter. Test a page with `<video>` (normally its own hardware-decode compositing path) and a page that itself renders WebGL/canvas content.
- **Input-to-photon latency**, not just throughput — 60fps delivered doesn't rule out an added frame or two of buffering between a click/drag and the pixel update from the shared-texture round trip.
- **Native page chrome beyond `<select>`** — spellcheck's suggestion menu, autofill/password-manager dropdowns, native `<input type=date/color>` pickers, macOS's Look Up/Services menu on selected text.
- **Agent-driven interaction.** Coordinate mapping for element-targeted vs. canvas-coordinate-targeted click/type actions through pan/zoom transform; concurrent human-canvas-gesture and agent-CDP-injection input on different pages at once; z-order/"is this page visible" semantics for agent routes that may currently assume native window stacking; CDP exposure scope now that every page (not just visible ones) is an always-alive, always-attachable offscreen target; and whether `tests/agent/` scenario coverage needs an OSR-mode counterpart, subject to the same CI/GPU ceiling above.
- **`<select>` fallback dropdown behaves like the native menu on real sites** (keyboard, scroll, change events) — the in-page mitigation for the macOS external-popup gap (Open questions, item 2).
- **Popup widgets (date/color pickers, autofill) anchor under the right element and close cleanly** on a range of real sites, not just the fixtures the anchoring logic was built against.

None of these block starting production build-out. They're the acceptance checklist before this ADR moves from Proposed to Accepted.

## Follow-up: one surface for pages and everything else

Written 2026-09-12 after the pan/zoom A/B (GPU process busy 10.6 s → 0.15 s on
the same 9-page test; report on the "Perf test" tab). Pan and zoom jank is
solved. What remains is the shape of the renderer: pages are drawn by one
`<canvas>` (`PageTextureSurface.tsx`) that sits between the chrome canvas and
aboveView, so the canvas is still three bands in two renderers, and page pixels
are copied twice on the way to the screen (`createImageBitmap`, then
`drawImage`) and redrawn on every camera tick. None of this is a decision yet.
These are the options considered, with the reasoning, so the next pass does not
re-derive them.

**The constraint every option lives under.** The browser composites a canvas as
one layer and DOM as other layers; the only choice is band order. Anything that
must be typed into is DOM. Everything else can be pixels. Full z-order
interleaving between pages and other entities is therefore only possible if, at
rest, everything is pixels in one surface and DOM appears only for the thing
being edited. Options are ranked by how close they get to that.

1. **One 2D canvas, one item list, draw in z-order.** Recommended. Pages are
   `drawImage(videoFrame, rect)` clipped to a rounded rect, no
   `createImageBitmap`, since Chromium's 2D canvas rasterizes on the GPU and a
   `VideoFrame` draw is a texture sample. Shapes, edges, borders, selection,
   grid, cursors and particles are paths and sprites in the same pass. A note at
   rest is a raster drawn as a quad, produced by canvas text for plain stickies
   or an SVG `foreignObject` for anything with real layout, re-rastered on zoom
   settle; the one note being edited is a DOM editor mounted above the canvas at
   the camera transform. Pan and zoom apply a CSS transform to the canvas
   element and redraw once on settle. Interleaving is draw order, so it costs
   nothing. Gives up per-pixel effects and instancing, neither of which is
   planned. Dirty-rect redraw and an `OffscreenCanvas` worker are the known
   optimizations if a trace ever asks for them. This merges bgView and
   aboveView, deletes the layer stack, the chrome canvas, the page texture
   canvas, the overlay child window, and main's window-pixel projection.
2. **HTML in canvas (`drawElement`).** The eventual rasterizer for option 1:
   browser layout for markdown and rich notes painted into the 2D canvas in
   draw order, with the elements staying in the accessibility tree. Chromium 150
   (Electron 43) ships it behind a Blink feature flag Electron can enable by
   default. Same scene, swapped rasterizer. Not a foundation on its own: no
   editing story, and pages are not DOM so it does nothing for them.
3. **Three.js WebGPU renderer.** Already a dependency: `PresenceParticleTrail.tsx`
   runs `three/webgpu` with TSL compute kernels. Pages become quads with a
   `VideoFrameTexture` each. That texture is not zero-copy: three 0.184 uploads
   it with `copyExternalImageToTexture` once per new frame
   (`WebGPUTextureUtils._copyImageToTexture`), and upstream removed the
   external-texture path in three.js PR #31416. The WebGPU spike below measured
   the copy and found it costs nothing we can see. Corners and
   focus rings in TSL, grid and chrome as geometry in one pass, particles fold
   into the same scene. Text stays DOM in a band above, so interleaving needs
   the same at-rest raster trick as option 1. Right choice only if per-pixel
   effects or thousands of instanced marks show up; otherwise it is more code
   than option 1 for the same picture. The particle trail's own rAF loop would
   have to become a participant in one shared loop first.
4. **Raw WebGPU.** Option 3 without the scene graph: one pipeline for textured
   rounded quads, one for lines, `importExternalTexture` for pages,
   `setScissorRect` exists but is the wrong tool (clip with geometry and UVs).
   A few hundred lines, full control, and every text and layout problem of
   option 3. No reason to pick it over three while three is already installed.
5. **DOM element per page** (a small canvas per page fed by
   `transferFromImageBitmap`, registered through `RendererSwitch`). Gives
   CSS z-order, corner radius, device shells and `EntityChrome` for free, and
   Chromium composites the layers. Rejected as the end shape: it moves the
   per-page compositor cost this ADR just removed back into the renderer, and
   twenty composited canvases is a different cost profile than one.
6. **drei-style `Html` overlays with occlusion.** Same band constraint,
   automated. Its raycast mode hides a whole element behind a mesh; its
   blending mode puts DOM under the canvas and punches transparent holes. In a
   2D world of rounded rectangles the exact equivalent is a `clip-path` on the
   DOM element subtracting the rects of entities above it, written on z-order
   or geometry change, not per frame. Worth keeping as the technique if some
   DOM has to sit mid-stack; not a foundation.

**Notes as DOM, pages and chrome as pixels.** tldraw splits its canvas this
way. Shapes and text are DOM and SVG inside a container that pans and zooms by
CSS transform. Chrome that changes every frame draws on a 2D canvas above it:
selection indicators, the brush, handles, snap lines and cursors. Excalidraw
and Figma take option 1's route and draw everything. Excalidraw can because its
text is plain, and Figma wrote its own text engine. Specular's notes carry
markdown and real layout, so the tldraw split fits better. Pages and chrome go
on the canvas and notes stay DOM above it, which drops option 1's note
rasterizer.

The cost is that a page can't sit above a note. Nothing needs that yet, and
this split keeps it possible:

- Z-order stays data. The scene is one ordered item list. "Notes paint above
  the canvas" is a renderer decision, and the model never encodes it.
- A page above a note uses option 6. The covered note gets a `clip-path` that
  cuts out the rounded rect of each page above it, rewritten when z-order or
  geometry changes. Pages are opaque rounded rects, so the cut is exact.
- A case clipping gets wrong, like a page shadow falling across a note, is
  handled by rasterizing that one note with option 1's trick, or with
  `drawElement` once it ships. The cost lands on that note, not on every note.

**Independent of the option chosen:**

- Stop copying: tried and measured slower in the 2D canvas, reverted
  2026-09-12. `43cf9203` transferred each page's `VideoFrame` and drew it
  as-is; `51f01686` went back to `createImageBitmap`. Same 30-page canvas
  (15 animating), two fresh runs of each build (`/perf/pan-zoom/run` plus a
  canvas-bg rAF recorder):

  | Build | Steady fps | Gesture fps | Gesture p95 | Frames > 25 ms | Pool drops |
  |---|---|---|---|---|---|
  | VideoFrame | 79 / 51 | 66 / 55 | 24.5 / 25.7 ms | 141 / 418 | 24 / 21 |
  | ImageBitmap | 89 | 116 / 111 | 9.3 / 16.6 ms | 2 / 9 | 0 / 0 |

  A gesture redraws every page each frame while page frames arrive far less
  often. A bitmap is copied once per page frame and drawn many times; a
  `VideoFrame` appears to pay a texture import on every 2D draw. The GPU
  process sat at 170–220% CPU in every run, so the copy is not the ceiling.
  The lifetime findings still hold for any `VideoFrame` path: the frame keeps
  its own reference to the shared texture after `imported.release()`, a held
  frame occupies one of its page's 6 slots until closed, and a frame posted to
  no listener holds that slot until GC. Revisit only with WebGPU
  `importExternalTexture` (options 3–4), where sampling a frame per draw is the
  intended path. Revisited 2026-09-19: see the WebGPU spike at the end of this
  section.
- Present on the renderer's rAF, not on frame arrival. A page frame marks its
  page dirty; the loop draws once.
- Set the host frame rate from the display (`setFrameRate`); offscreen hosts
  default to 60 on a 120 Hz panel.
- Re-render on zoom settle by resizing the host or its device scale factor, or
  text goes soft past 1x. Any option needs this policy.
- Hit testing moves to the renderer with the pixels; main stops projecting
  rects for input mapping and keeps state and page hosts only.
- Measure with `/perf/pan-zoom/visual-run` at 30 pages before deleting the
  current draw loop, whichever surface replaces it.
- Paint each item whole. Landed 2026-09-12: one `CanvasItemSurface` draws a
  page's shell, border, and texture before the next page, in z-order. Two
  passes put every shell under every texture, so a page stacked above another
  let that page's content show over its own bezel. The experimental SVG shell
  layer was deleted in the same change.

**WebGPU spike, measured 2026-09-19.** Branch `spike/webgpu-page-surface`,
never merged. The question was whether drawing page textures with WebGPU
removes the per-frame copy cost, and which integration we could live with. Four
arms, switched at launch by the `specular.spike.pageSurfaceArm` localStorage
key:

- A. Today's path. `createImageBitmap` in the preload, 2D canvas draw.
- B. Raw WebGPU with no copy. The preload transfers the `VideoFrame`, and the
  surface calls `importExternalTexture` for every page on every draw.
- C. Raw WebGPU, import then blit. Each new frame is imported once and blitted
  into a `GPUTexture` we own, sized to the page's on-screen size. Draws sample
  that texture.
- D. Plain `three/webgpu`, one mesh per page with a `VideoFrameTexture`. three
  copies each new frame with `copyExternalImageToTexture`.

Same canvas as the cost model above, 30 animating and 28 static pages, pan
(518, 158). Each arm ran in two fresh launches with two 16 s samples per zoom,
so every cell is the range over four samples. Numbers are percent of one core.

| Zoom | Process | A: 2D | B: import | C: blit | D: three |
|---|---|---|---|---|---|
| 0.1 | Total | 143–150 | 131–138 | 130–132 | 128–133 |
| 0.1 | GPU process | 72–74 | 59–62 | 59–60 | 59–61 |
| 0.1 | canvas-bg | 16–17 | 15 | 15 | 16–17 |
| 0.1 | Main | 20–21 | 20–21 | 20 | 19–20 |
| 0.1 | Pages | 33–38 | 35–40 | 35–37 | 34–36 |
| 0.352 | Total | 90–91 | 84 | 81–82 | 83–85 |
| 0.352 | GPU process | 48 | 41 | 40 | 40–42 |
| 0.6 | Total | 88–95 | 81–83 | 78–85 | 79–84 |
| 0.6 | GPU process | 51–55 | 42–43 | 40–44 | 40–42 |

| Check | A: 2D | B: import | C: blit | D: three |
|---|---|---|---|---|
| Gesture rAF rate, zoom ramp and pan sweep | 120 fps | 120 fps | 120 fps | 120 fps |
| Gesture frames over 25 ms | 0 | 0 | 0 | 0 |
| Total CPU, 12 s zoom oscillation with 60 pages drawn per tick | 233–235 | 235–236 | 230–238 | 240 |
| canvas-bg CPU in that oscillation | 28 | 30–32 | 32 | 32–34 |
| `sendFailures` / `framesDroppedForPoolPressure` | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| `outstandingTextures` per page at rest | 0 | 1 | 1 | 1, sometimes 2 |
| GPU-process footprint, two launches | 1769 / 1589 MB | 1773 / 1670 MB | 1801 / 1687 MB | 1912 / 1810 MB |

The oscillation total includes about 83 points of test-driver cost, the same
in every arm. That is the toolbar renderer evaluating 60 `zoomSet` calls a
second over CDP.

What the numbers say:

- WebGPU saves 10 to 15 points of 146 at thumbnail zoom and 6 to 10 at the
  other two. The GPU process gives back all of it. Main, canvas-bg and the
  pages do not move. It is a real saving and a small one, about a quarter of
  the 59 points the cost model charges to transfer and copy. The rest of that
  59 is the capture handshake, the IPC hop and the shared-texture import, and
  no renderer choice touches those.
- The copy is not what costs. B never copies, C copies once into a small
  texture, D copies at full texture size, and the three land within 5 points of
  each other at every zoom. B's 60 imports per draw at 120 draws a second also
  cost nothing we could measure in the oscillation run. So the saving over A
  comes from leaving `createImageBitmap` and the 2D canvas behind. These runs
  cannot split it between the two.
- Gestures are a tie. Every arm held 120 fps with no frame over 25 ms. The 2D
  `VideoFrame` attempt lost this test, 55 to 66 fps against 111 to 116.
  WebGPU does not repeat that, because sampling an imported frame per draw is
  what the API is for.
- No arm made transfers worse. A held `VideoFrame` keeps one of its page's 6
  texture slots, so B, C and D sit at 1 outstanding per page where A sits at 0.
  48 of 60 hosts held a frame, culled pages included. Pool drops and send
  failures stayed at zero, but the headroom is 5 slots, not 6.
- Memory moves less than it drifts. D carries about 140 to 220 MB more than A,
  which fits one full-size copy per page. B and C sit within 100 MB of A, and
  launch-to-launch drift on A alone was 180 MB. C's owned textures totalled
  45 MB at zoom 0.352.

Recommendation. C is as cheap as B, so the numbers support the first of the
three long-term shapes. Pages become ordinary three objects, with no patch to
three, full z-order interleaving, and R3F possible later. D matching both means plain
`VideoFrameTexture` is enough to start with. Wrapping a `GPUTexture` we own in
three's `ExternalTexture`, as C does, is the step to take if per-page GPU
memory matters, since it holds an on-screen-size texture where D holds a
texture-size copy. C could also close each frame right after its blit and give
the slot back. It would then need `requestPageFrames` to re-blit a static page
after a zoom settles. The spike did not try that.

Do not port the renderer for the CPU alone. Ten percent at thumbnail zoom does
not pay for a rewrite. What the spike changes is the ranking above. Option 3 no
longer carries a performance risk against option 1, so the choice between them
can rest on z-order, effects and how much code each one deletes. A raw page
layer beside a three scene, or a maintained patch to three, has no case: B is
not ahead of C or D.

What the spike skipped. Popups are dropped in the preload. D draws square
corners. B and C mask corners in the fragment shader. Shells, borders and
shadows stayed on the 2D `CanvasItemSurface`, which ran underneath in every
WebGPU arm with no page bitmaps to draw. So B, C and D each paid for a second
full-window canvas layer that a one-surface port would not have, and their
canvas-bg numbers read a little high for it. CDP drove the gestures at 60
inputs a second, not a trackpad. The rAF rate shows that canvas-bg's main
thread kept up. It does not count frames the compositor presented.

**Native page layer spike, measured 2026-09-19.** Same branch, arm E. The
question was how much a native host could save by skipping the hand-off to
canvas-bg. A Rust N-API addon (`spike/native-page-layer/`, launched with
`SPECULAR_SPIKE_NATIVE_PAGES=1`) gives each page one `CALayer` inside an
`NSView` in the window. Main sets every painted frame's `IOSurfaceRef` as that
layer's contents straight from `deliver`, and moves the layers on every layout
pass. There is no `sendSharedTexture`, no import in canvas-bg, no copy and no
shader. canvas-bg receives no frames and draws shells and borders only.

Core Animation composites in WindowServer, outside the app, so these runs count
WindowServer's CPU as well. Arms A and C were re-measured the same way. Same
canvas, zooms and pan as the WebGPU spike. Each cell is two 16 s samples from
one launch, in percent of one core.

| Zoom | Measure | A: 2D | C: WebGPU blit | E: native layer |
|---|---|---|---|---|
| 0.1 | App processes | 142 | 133–135 | 88–90 |
| 0.1 | GPU process | 71 | 60–61 | 36 |
| 0.1 | WindowServer | 40–41 | 37–42 | 60–61 |
| 0.1 | App plus WindowServer | 182–183 | 170–178 | 148–151 |
| 0.352 | App plus WindowServer | 131–134 | 127–128 | 112–119 |
| 0.6 | App plus WindowServer | 145–147 | 135 | 110–113 |
| 0.1↔0.25 oscillation | App plus WindowServer | 283 | 286 | 285–286 |

An earlier launch of E, measured before WindowServer was counted, put the app
processes at 81 to 84 at zoom 0.1. WindowServer's number is system-wide, so it
carries whatever else the desktop was drawing. That load was the same terminal
session for every arm.

What the numbers say:

- Counting the app alone, E looks like a 40% cut, 142 down to 88. That figure
  is wrong to quote. WindowServer takes on 20 of the points Chromium's GPU
  process gave up. The saving that holds is 32 points at zoom 0.1, 17 at 0.352
  and 35 at 0.6, which is 13 to 24% of the machine-wide cost. WebGPU saved 2 to
  8% on the same footing.
- E reaches the floor the cost model predicted. Its app processes sit at 81 to
  90 against the 87 of the capture-only run. Everything left in the app is
  Chromium's capture and the pages themselves, so no host, native or not, goes
  lower without cutting frame rate or texture size.
- Main did not get cheaper. It stayed at 19 to 20 points with no import and no
  send, so main's share is paint-event and capture bookkeeping, not the hop to
  canvas-bg.
- Gestures gain nothing. Moving 60 layers per camera tick costs WindowServer 75
  points, and the three arms finish within 4 points of each other. Every arm
  held 120 fps with no frame over 25 ms.
- Page hosts stayed clean, with 0 send failures and 0 pool drops. E holds each
  replaced frame for 34 ms so Core Animation is done reading it, which shows
  as 1 to 2 outstanding textures per page. GPU-process footprint was 1446 and
  1458 MB, below every other arm.

What it costs. Electron draws bgView, aboveView, the toolbar and the side
panels through one `ViewsCompositorSuperview`. The `WebContentsViewCocoa`
subviews beside it are zero-size. A native layer can therefore sit above all
web content or below all of it, and nowhere in between. The spike put it above,
so pages covered the toolbar, selection outlines and menus. Shipping it means
putting it below and making the window and every web layer transparent over
each page, which is option 6's hole-punching, with no interleaving between
pages and other items at all. Pages also move in main's layout pass while their
borders move after an IPC hop, so the two can separate during a pan. The spike
did not measure that. The addon is macOS-only and would need signing,
notarizing and a Windows counterpart.

Recommendation. Do not build this. It is the best result of the day, and it
confirms that a native host's ceiling is about a fifth off the machine-wide
cost at rest and nothing during gestures. The price is z-order, the opposite of
what the one-surface work is for. The remaining 85 to 90 points are capture,
and frame-rate and texture-size policy are the only things that move them
(`page-frame-rate.ts`, `page-texture-scale.ts`). Thumbnail pages at 8 fps
instead of 15 is the next measurement worth an hour.

What the spike skipped. Popups, corner radius, and any clipping to the canvas
area. Pages drew over the toolbar and panels. Z-order among pages followed
entity order with the focused page last, not the full draw-order rules.

**HTML-in-canvas probe, 2026-09-19.** Option 2 is real in this Electron. The
Chromium 150 binary in Electron 43.2 carries the API behind a Blink feature.
Launching with `--enable-blink-features=CanvasDrawElement,HTMLInCanvas`
(`SPECULAR_SPIKE_BLINK_FEATURES` on the spike branch) exposes
`ctx.drawElementImage` for 2D, `gl.texElementImage2D`,
`GPUQueue.copyElementImageToTexture`, and `layoutSubtree`, `requestPaint` and a
`paint` event on the canvas. The probe ran in canvas-bg over CDP, with no app
code, against DOM notes appended as children of a `<canvas layoutsubtree>`:

- The note draws with real layout: wrapping, bold, italic, lists, inline code.
  A draw under a 2.5× transform is vector-crisp, because Chromium replays the
  element's paint record at the canvas transform. There is no bitmap to go
  soft, so notes would need no re-raster when a zoom settles.
- Draw order is call order, so a note between two pages is two `drawImage`
  calls around one `drawElementImage`.
- 60 distinct notes redrawn every frame under a continuous zoom held 120 fps
  with no frame over 25 ms and 0.14 ms of script per frame. It cost 33 points
  of app CPU while the motion lasted. Repainting all 60 at a fixed transform
  30 times a second, which is what animating pages force on a shared surface,
  cost 3.5 points.
- The element stays live DOM. A `contenteditable` child took focus and typed
  text, and the canvas got a `paint` event whose `changedElements` named the
  note. `drawElementImage` returns a `DOMMatrix`. Set as the element's CSS
  transform, it put the DOM rect exactly on the drawn pixels, and
  `elementFromPoint` found the note there.
- Drawing stops at the border box. A `box-shadow` is cut off, so shadows need
  to be drawn by the canvas or the element needs a padded wrapper.
- A `paint` event only fires while the document is visible. With the window
  covered, `drawElementImage` throws "No cached paint record for element".

These CPU numbers were taken with the app window covered, so they compare with
each other and not with the tables above. Not tested: the caret and selection
while editing, IME, the real note editor, images and video inside a note,
scrolling content, accessibility, and how stable a flagged API stays across
Electron upgrades. The probe also put note DOM in canvas-bg. Real notes live in
above-view, which owns keyboard focus, so a build has to choose between drawing
read-only copies in canvas-bg and keeping today's editor in above-view, or
merging the two renderers.

**What notes cost today, measured 2026-09-19.** The perf canvas holds 60 pages
and nothing else, so no run before this one could see note cost. The tab
"Perf test (animated + notes)" is the same 60 pages at the same rects plus 60
stickies, 50 in the gaps between rows and 10 on top of pages. Current 2D path,
window on screen, same launch for both tabs, percent of one core:

| Measure | Pages only | Pages plus 60 stickies |
|---|---|---|
| At rest, zoom 0.1, app total | 138 | 140–141 |
| At rest, above-view | 0 | 0 |
| Zoom oscillation 0.1↔0.25, app total | 203–204 | 245–246 |
| Zoom oscillation, above-view | 36 | 63 |
| Zoom oscillation, GPU process | 63–64 | 75 |
| Zoom oscillation, rAF | 120 fps | 120 fps |

DOM notes are free at rest. During a zoom, 60 of them add about 42 points, 27
of those in above-view, which reprojects every note on every camera tick. The
HTML-in-canvas probe drew 60 richer notes per frame for about 33 points, under
different conditions, so the two are in the same range and HTML-in-canvas is
not a performance win on this evidence. It buys interleaving. The number worth
chasing is the other one: above-view spends 36 points per zoom with no notes
at all, on per-page overlay work for 60 pages.

**Where this leaves the renderer, 2026-09-19.** The 2D item canvas stays. Four
alternatives were measured against it and none earns a port now. The code for
all of them is on `spike/webgpu-page-surface`, which does not merge.

- three/webgpu, with or without R3F, is deferred. It saves 2 to 8% of
  machine-wide CPU and nothing in gestures. It costs nothing either, so it is
  the renderer to pick if effects or a shared scene ever justify a port. Only
  plain three was measured. R3F was not.
- Raw WebGPU has no case. It ties with three.
- A native layer is rejected. It saves 13 to 24% at rest and cannot sit between
  Electron's web views.
- HTML-in-canvas is deferred. It is the way to draw notes in the item canvas
  with real layout, interleaved with pages, at a cost in the same range as DOM
  notes today. It is behind a Blink flag, and the caret, IME, the real editor
  and media inside notes are untested. ADR 0014 has notes painting above pages,
  so nothing needs interleaving yet.

What to measure next for CPU, in order of expected return:

1. Capture policy. About 60% of the cost is Chromium's capture, and only frame
   rate and texture size move it (`page-frame-rate.ts`,
   `page-texture-scale.ts`). Thumbnail pages at 8 fps where they now get 15 is
   unmeasured.
2. above-view during a zoom. It spends 36 points with no notes on the canvas,
   on per-page overlay work for 60 pages, and 27 more with 60 stickies. No
   renderer swap touches that.

Two tabs carry the benchmark. "Perf test (animated)" has 60 pages, and
"Perf test (animated + notes)" adds 60 stickies to the same layout. Count
WindowServer whenever a change moves work into native or compositor layers,
and check `document.visibilityState` before a run, because a covered window
stops rendering.
