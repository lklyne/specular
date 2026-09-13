# ADR 0038 — Offscreen GPU-texture compositing for live pages

**Status:** Proposed — spike validated the perf hypothesis; production build-out (agent routes, capability parity, pool tuning) is unstarted. Implementation landed on branch `claude/offscreen-rendering-canvas-c81a4k`; the Post-build validation checklist below is the acceptance gate for Accepted.
**Date:** 2026-09-10 (revised 2026-09-12 — animated-content spike reversed the mode recommendation; see "Animated-content fidelity" below)
**Related:** [ADR 0023 — Renderer-owned camera and GPU-composited pan/zoom](./0023-renderer-owned-camera-gpu-panzoom.md) (rejected; its postmortem named the live-page substrate, not the transform, as the hard part), [ADR 0037 — A pan does not freeze its pages](./0037-pan-does-not-freeze-its-pages.md) (the freeze/park machinery this ADR would make unnecessary), [ADR 0014 — Canvas stack order](./0014-canvas-stack-order.md) (the layering constraint this ADR removes rather than preserves), [ADR 0036 — Diffed runtime store](./0036-diffed-runtime-store.md) (established pan is not a page-count problem for the *DOM* side — this ADR addresses the *compositor* side that remains).
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
- **The freeze/park system** — `zoom-snapshot-freeze.ts`, `drag-freeze.ts`, `useFrozenPageBitmaps.ts` / `chromeItemDraw.ts`'s `drawItemSnapshot`, and ADR 0037's warm-park/hidden-park distinction. This machinery exists to *simulate* shape B during gestures only; if every page is already a texture all the time, there is nothing to freeze or unpark.
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

## Decision

Proceed to build production-shaped offscreen-texture compositing for live pages, replacing per-page `WebContentsView`s with offscreen `BrowserWindow`s composited onto one canvas surface — **starting from shared-texture mode as the initial production target, not bitmap-JPEG.**

This reverses the original (2026-09-10) call. The static-page spike showed both modes flat and recommended JPEG-first on complexity grounds; the animated-content spike (2026-09-12) showed bitmap-JPEG's synchronous per-frame main-thread encode collapses to ~1/8th of native frame rate under realistic concurrent-animation load, while shared-texture sustains full native rate with no pool pressure once `MAX_OUTSTANDING_TEXTURES` is raised from its overly-conservative default of 6 to 9. A live-page canvas exists specifically to show pages that animate or are being interacted with — that is not an edge case to defer, it is close to the point of the product — so the mode that only works well on static content is not an acceptable production default. The operational complexity JPEG-first was chosen to avoid (`sharedTexture.importSharedTexture` lifecycle, pool sizing, release-latency tracking) is real but now clearly worth paying: it is what actually delivers working page animation at page counts the app needs to support.

This does not yet decide shape B (every page offscreen, including the interactive one) vs. a hybrid — that depends on closing open question 2 above. If IME/drag-out/upload prove unacceptable, the fallback named in the research doc still holds: shape C plus the texture tier, keeping today's input model but removing the `setBounds` cost this spike measured.

## Implementation

Shape B, built. Every page is a hidden offscreen `BrowserWindow` (`page-host.ts`) sized to its CSS viewport, painting with `offscreen: { useSharedTexture: true, deviceScaleFactor }`. Each painted frame is imported as a GPU shared texture in main and sent to canvas-bg's main frame, where a pool of 9 outstanding textures per page (`MAX_OUTSTANDING_TEXTURES`, raised from the spiked default of 6) bounds how far the renderer can fall behind before a frame is dropped.

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
- Re-run the original 40-*static*-page pool-pressure check at the new cap of 9 to confirm that ceiling also moved before treating pool sizing as fully closed.
- The macOS `<select>` fallback dropdown (page preload, in parallel with this branch) needs its own real-site validation — see Post-build validation.
- The manual Post-build validation checklist below is unrun; it is what moves this ADR from Proposed to Accepted.

**Not resolved by this ADR:** whether shape B fully replaces shape C, or lands as a hybrid — in practice, shape B shipped and the hybrid fallback was not needed.

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
   runs `three/webgpu` with TSL compute kernels. Pages become instanced quads
   with a `VideoFrameTexture` each (zero-copy external texture), corners and
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

- Stop copying. `drawImage` and WebGPU both accept a `VideoFrame` directly;
  `createImageBitmap` is the copy that shows up in every trace.
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
