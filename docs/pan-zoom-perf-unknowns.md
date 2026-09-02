# Pan/Zoom Perf — The Unknown Unknowns

*Research note, 2026-07-11. A deep sweep of Chromium/Electron internals, prior art,
and this codebase to map every avenue for canvas pan/zoom performance beyond the
two already on the table (folding the page background into the WCV, and
freeze-to-bitmap during gestures). Sibling to `docs/offscreen-rendering-research.md`
and `docs/canvas-motion-research.md`; corrects part of the ADR 0023 postmortem.*

*Everything below was verified against primary sources (Chromium `main` source,
Electron source/docs/PRs, issue trackers) by parallel research agents. Claims are
flagged confirmed/uncertain inline; links are load-bearing.*

---

## 1. The mental-model corrections (this changes the option space)

### 1.1 Device-emulation `scale` is NOT a reflow — the zoom storm is raster + surface churn

ADR 0023's postmortem says per-tick `enableDeviceEmulation({ scale })` "is the
reflow cost." **Confirmed false for our call shape.** Read from Blink source:

- With a **fixed `viewSize`** and **constant `deviceScaleFactor`** (exactly what
  `runtime-geometry.ts:289-296` passes — viewSize is the CSS content size,
  zoom-independent), a scale-only change never touches layout. The layout
  viewport is pinned to `viewSize`; `scale` becomes a root
  `TransformPaintPropertyNode` above the page-scale node
  ([screen_metrics_emulator.cc](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/frame/screen_metrics_emulator.cc),
  [web_view_impl.cc](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/exported/web_view_impl.cc)).
  Identical-params calls early-return in the renderer (near-free beyond the IPC).
- What each scale change **does** cost, per page, per tick:
  1. **A forced whole-tree re-raster.** `UpdateDeviceEmulationTransform` calls
     `SetNeedsRecalculateRasterScales()` — every `PictureLayerImpl` recomputes its
     ideal raster scale and re-rasters all tilings, with **none** of the throttling
     a real pinch gesture gets. This is the storm.
  2. **A media-query re-evaluation** (`MediaQueryAffectingValueChanged(kOther)`) —
     cheap-ish unless a site's MQ results flip.
  3. One mojo IPC + one main-thread commit.
- Separately, the per-tick **`setBounds` resize** (bounds = viewSize × zoom)
  allocates a **new viz `LocalSurfaceId` every tick** → output-surface
  reallocation (IOSurface/CALayer on macOS), full-frame redraw at the new pixel
  size, and browser⇄renderer surface synchronization — the classic resize-jank
  path. Likely co-dominant with the raster storm.
- **Trap to guard:** if `deviceScaleFactor` ever *varies* between calls, Blink
  **evicts the renderer's entire memory cache per call**
  ([dev_tools_emulator.cc](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/inspector/dev_tools_emulator.cc)).
  We pass `screen.getPrimaryDisplay().scaleFactor` — constant on one display, but
  it changes when the window moves between displays mid-session. Worth a log-line
  assertion.

**Why it matters:** Chromium's own pinch zoom demonstrates the cheap recipe —
during an active pinch, cc GPU-scales existing tiles, re-rasters lazily in ×2
steps snapped to existing tilings, and goes crisp on settle
([picture_layer_impl.cc](https://source.chromium.org/chromium/chromium/src/+/main:cc/layers/picture_layer_impl.cc):
`kMaxScaleRatioDuringPinch = 2.0`, `kSnapToExistingTilingRatio = 1.2`). Our per-tick
emulation demands ideal-scale re-raster *and* a new surface every 16ms instead.
A settle-only (or coarse-stepped) emulation cadence is aligned with how the
engine wants to be driven — and the final `enableDeviceEmulation` at gesture end
*is* the force-re-raster-crisp-at-settle primitive.

DevTools device mode's zoom % control is literally this mechanism (the `scale`
field of `setDeviceMetricsOverride`), and its frontend carries two tricks worth
stealing: it avoids size overrides when only scale changes, and it prefers
"sharp" scales where `size × scale` lands on integers
([DeviceModeModel.ts](https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/models/emulation/DeviceModeModel.ts)).

### 1.2 On macOS, every WebContentsView is a whole compositor, not just a surface

Each visible WCV gets its own `ui::Compositor` + viz `Display` + `SurfaceAggregator`
+ remote CoreAnimation `CAContext` commit pipeline in the GPU process, plus a
`CALayerHost` NSView in the browser process
([recyclable_compositor_mac.h](https://raw.githubusercontent.com/chromium/chromium/main/ui/compositor/recyclable_compositor_mac.h)
exists precisely because "creating a new ui::Compositor for each tab would be
expensive"; Chrome recycles them because only ~1 tab is visible — a canvas with
20 simultaneously-visible views gets no such relief). Specular **doubles** this
with the `about:blank` frameView per page. There is no API to fence N views'
commits to each other or to a renderer's frame — content updates ride N
independent pipelines, so inter-view shear during gestures is **expected
behavior, not a bug** (per-view `DelegatedFrameHost`s; Chromium's "resize is
async" surface-sync model; no cross-surface atomicity anywhere).

Also real on macOS: CoreAnimation quad budgets (`kLayerLimitDefault = 128`
promoted CALayers per frame, fallback compositing beyond 30
`RenderPassDrawQuads` — [ca_layer_overlay.cc](https://raw.githubusercontent.com/chromium/chromium/main/components/viz/service/display/ca_layer_overlay.cc)).

**Why it matters:** the border/content drift is structural to the two-substrate
architecture. The fixes that work are the ones that *reduce the number of
independent pipelines* (kill frameView, unify substrates), not ones that try to
synchronize them harder. This also upgrades the "page bg as part of the wcv"
idea from a nicety to a structural win: it halves processes, halves compositor
pipelines, and halves CATransaction commits per frame.

### 1.3 "Frozen but visible" does not exist in the API surface

- CDP `Page.setWebLifecycleState('frozen')` **force-hides the WebContents before
  freezing** (`web_contents->WasHidden()` in
  [page_handler.cc](https://github.com/chromium/chromium/blob/main/content/browser/devtools/protocol/page_handler.cc));
  Blink refuses to freeze a visible page. On macOS the NSView is hidden — the
  last frame does **not** stay on screen. And thaw doesn't re-show. Dead end for
  gesture-time freezing of visible pages.
- The one Chromium state that keeps pixels while stopping the renderer —
  **occluded** — is window-granular and not app-triggerable per view.
- **Frame eviction limit:** hidden views keep their last frame only while locked;
  the `FrameEvictionManager` keeps at most **5** saved frames on desktop and
  culls idle ones after 5 minutes
  ([frame_eviction_manager.cc](https://github.com/chromium/chromium/blob/main/components/viz/client/frame_eviction_manager.cc)).
  More than ~5 simultaneously hidden pages cannot all resurrect instantly — they
  come back blank until a fresh frame. Any "park hidden, restore on demand"
  design must pair hiding with our own captured bitmap.
- The lever that *does* work mid-gesture with pixels on screen:
  **`Emulation.setCPUThrottlingRate(N)`** per page at gesture start, `1` at end —
  suspends only the guest's main thread in 200µs quanta, compositor untouched,
  instant and reversible ([thread_cpu_throttler.cc](https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/scheduler/common/thread_cpu_throttler.cc)).
  Unconventional use of a DevTools API, but mechanically exactly "shed guest CPU
  during gestures."
- `backgroundThrottling(false)` is the wrong-direction knob (it only *prevents*
  hidden-state throttling; for visible views it changes nothing, and it blocks
  the freeze machinery).

---

## 2. The unknown unknowns, ranked

### 2.A The `<webview>` / OOPIF single-compositor canvas — the structural bet

**The finding:** Electron's `<webview>` tag is today implemented as a Chromium
**out-of-process iframe** wrapped in a custom element
([docs](https://www.electronjs.org/docs/latest/api/webview-tag): "implemented
with Out-of-Process iframes"). An OOPIF lives *inside the embedder's layer tree*
as a `cc::SurfaceLayer` — so a `<webview>`:

- **rides CSS transforms** (translate *and* scale) applied to it or any ancestor,
  composited by Viz in one draw with the surrounding DOM;
- **re-rasters crisp at the embedder's screen-space scale**: Chromium propagates
  `compositing_scale_factor` through `VisualProperties` to the child renderer
  (there's a browsertest asserting a child under a parent `scale(0.5)` rasters at
  0.5× — [commit](https://chromium.googlesource.com/chromium/src/+/e76e8b1ba2c7b4bfb8edca177745a65877429602%5E!/)).
  During an animated transform the stale surface is texture-scaled (smooth,
  soft); the new scale propagates async and content snaps crisp — pinch-zoom
  semantics, for free, per page;
- **hit-tests correctly under transform** (surface-based input routing is how
  Chrome handles transformed cross-origin iframes on the open web);
- keeps per-page renderer processes (site isolation intact — same process count
  as WCVs), DevTools attach, and its own webContents.

Under this model the entire canvas — grid, borders, chrome, *and live pages* —
is one renderer with one camera transform. Pan/zoom becomes a single
GPU-composited transform update. **Border/content drift becomes impossible by
construction**, the two-substrate tax disappears, ADR 0014's z-band gymnastics
collapse (DOM interleaves with pages natively), and the layout engine's
per-tick setBounds loop deletes.

**Prior art:** Deta Surf ships spatial canvases of live `<webview>`s in
production ([deta/surf](https://github.com/deta/surf)). tldraw's embeds prove
the UX model. Stack Browser's postmortem of the *opposite* path (everything as
native BrowserViews, Electron fork for z-order) is the cautionary tale we're
currently living ([writeup](https://www.ika.im/posts/tech-behind-stack-browser)).
VS Code migrated *off* `<webview>` to iframes-in-one-renderer for its own
reasons — the direction was still "out of native views, into one compositor."

**Risks (real, testable):**
- Electron's docs have formally discouraged `<webview>` for ~6 years (Chrome
  Apps GuestView heritage). It still ships and works in Electron 40; removal
  risk hasn't materialized in a decade, but a canvas built on it rides an
  unloved API.
- Regression history for exactly our workload: ghosting when panning a
  transformed div full of webviews (Electron 7 era,
  [#20766](https://github.com/electron/electron/issues/20766), fixed);
  `capturePage` of a transformed webview captures blurry
  ([#44227](https://github.com/electron/electron/issues/44227), open — capture
  at scale 1 or via the guest webContents). Per-upgrade regression testing is
  the tax.
- Popup widgets (`<select>`, context menus), IME, and drag-drop anchoring under
  transform are the likely residual glitches — BrowserPlugin-era misalignment
  bugs are obsolete, but verify empirically.
- Whatever zoom you settle at is the raster resolution (crisp *for that zoom* —
  same as pinch-zoomed Chrome).

**Outcome (2026-08-23): tried, does not work. Closed.**

This section was written as the highest-leverage prototype in the space — a day
of work to find out whether Specular's rendering architecture problem simply
dissolves. It was built, and it does not. The specific failure mode was not
written down at the time and is no longer remembered, so the analysis above
stands as the reasoning, not as a live recommendation.

Everything in this doc that routes through 2.A — experiment 1 and Endgame A —
is closed with it. The remaining structural option for collapsing the two
substrates is §2.B (`sharedTexture`).

Do not re-open this on the strength of the argument above. The argument was
good and the result was still negative; if it is ever revisited, the first job
is recovering *why*, not rebuilding the spike.

### 2.B Electron 40's `sharedTexture` module — GPU compositing without a native addon

The "composite all live sites on the GPU" dream got a new primitive **in the
Electron major we already ship** (40.x, backported late 2025 —
[PR #48831](https://github.com/electron/electron/pull/48831), experimental):

- Offscreen rendering with `webPreferences.offscreen.useSharedTexture: true`
  delivers each page frame as an **IOSurface-backed GPU texture** (`paint` event,
  ~10-texture pool, dirty rects, *no frames generated when the page is idle*,
  no frame-rate cap; one GPU→GPU copy, zero CPU readback).
- The new **`sharedTexture` module** imports that handle in main
  (`importSharedTexture`) and transfers it to any renderer
  (`sendSharedTexture` → `setSharedTextureReceiver`), where
  **`getVideoFrame()` yields a standard `VideoFrame`** usable with Canvas2D
  `drawImage`, WebGL, or WebGPU `importExternalTexture`. Release is
  sync-token-aware. **No native addon.** The spec test for this pipeline runs
  only on **macOS arm64** — our exact target
  ([spec](https://github.com/electron/electron/blob/main/spec/api-shared-texture-spec.ts),
  [design doc](https://github.com/electron/electron/blob/main/shell/common/api/shared_texture/README.md)).
- The only published benchmark, from the feature author compositing OSR pages
  into WebGPU: **16 pages @720p ≈ 54fps, 16 @1080p ≈ 35fps, bottleneck
  CPU-side (per-frame IPC), not GPU**
  ([PR #46811](https://github.com/electron/electron/pull/46811)). Dirty-rect +
  idle suppression means a mostly-static canvas does far better than the
  all-animating worst case.

**Where it fits:** not a drop-in replacement for interactive pages (input must be
re-injected via `sendInputEvent`, popups arrive as separate textures to compose,
OSR is a creation-time flag — no live toggle on an onscreen view). Its natural
role is the **non-interactive tier**: parked/idle/far-zoomed pages live only as
OSR texture sources riding the canvas camera; selection promotes to a real WCV
(or the page is already a `<webview>` if 2.A wins). This is the modern,
JS-reachable version of `docs/offscreen-rendering-research.md`'s "Option B"
which previously required a native addon. Caveats: experimental API surface,
open bug that offscreen WebContentsViews paint at window size
([#45864](https://github.com/electron/electron/issues/45864) — use offscreen
BrowserWindows/webContents per page), OSR pages are never
background-throttled.

### 2.C Visual-viewport page scale — the compositor-side zoom nobody drives

Chromium has a second, cheaper scale axis we've never touched: **page scale**
(pinch zoom). It's a compositor transform that "doesn't interact with layout,"
drivable per-frame via CDP **`Emulation.setPageScaleFactor`** after
`webContents.setVisualZoomLevelLimits(min,max)` widens the clamp (Electron maps
that straight to `SetDefaultPageScaleLimits`). Per-call cost is one transform
node + commit — no forced whole-tree re-raster storm, no surface reallocation.

Open questions that decide viability (one afternoon of experiments):
- **Sub-1.0 scale** is the weak spot: desktop pages normally clamp min page
  scale to ~1, and the visual viewport can't exceed the layout viewport.
  Mobile emulation sets limits like (0.25, 5) — whether that yields usable
  zoom-out with sane gutter behavior on desktop pages is unverified.
- It scales content *within* fixed view bounds (crops rather than resizes), so
  it must be coordinated with bounds — possibly "bounds follow zoom on settle,
  page scale carries the mid-gesture visual."
- It goes through the guest's main thread (inspector agent), so a busy page
  delays it — unlike a real pinch, which lives on the compositor thread.

Even if it fails as the primary mechanism, the same investigation yields the
cheap intermediate: **quantized emulation** — step `enableDeviceEmulation`
scale in coarse increments (~×1.25, or ≤10Hz) during the gesture and exact on
settle, mimicking cc's own pinch heuristics.

### 2.D Tab capture as MediaStream — live pixels on the current Electron, ~40 lines

`session.setDisplayMediaRequestHandler` can route a `getDisplayMedia()` request
to **any page's `mainFrame` as the video source** — compositor-level frame-sink
capture of each page as a `MediaStream`, no ScreenCaptureKit, no screen-recording
permission ([session docs](https://www.electronjs.org/docs/latest/api/session)).
`<video>` elements (or WebGPU external textures) in canvas-bg then ride the
camera transform with live pixels.

Reality check: default delivery is I420 through shared memory — a GPU→CPU
readback + YUV convert per frame per stream + GPU re-upload (~46MB/s/stream at
1280×800@30). Fine for ~5–10 streams on Apple Silicon, fan-territory at 30.
Captured renderers are deliberately never background-throttled (streams don't
freeze when views hide — good; 30 permanently-foreground renderers — bad).
`getDisplayMedia` needs transient user activation (~5s window) — inherent for
gesture-time use, a constraint for pre-warming. **Verdict: the bridge, not the
destination** — the right way to prototype "live textures under one camera"
before committing to 2.B, and possibly the shipping mechanism for a handful of
"live thumbnail" pages.

### 2.E Crisp frozen pages — freezing doesn't have to mean blurry bitmaps

If a freeze tier ships (gesture-time or parked-page), bitmaps aren't the only
option, and crispness-under-zoom is exactly where bitmaps hurt:

1. **Frozen DOM (the sleeper):** `Page.captureSnapshot` (MHTML via
   `webContents.debugger`) or a SingleFile-style serializer, rendered in a
   **sandboxed, script-stripped `<iframe srcdoc>`** inside canvas-bg under the
   camera transform. Infinitely crisp at any zoom, selectable text, zero extra
   processes. Risks: fidelity survey needed (CSP/resource inlining, cross-origin
   iframes lost), heavy DOM lands in the compositor renderer.
2. **High-DPI bitmap:** capture at 2–3× (capturePage already returns DPR-scaled;
   OSR takes `deviceScaleFactor: 2`; CDP `Page.captureScreenshot` +
   `captureBeyondViewport`) → sharp until 200–300% zoom, re-capture on settle.
   The pragmatic default.
3. **`printToPDF`:** true vector (crisp at any zoom via pdf.js) but print-media
   reflow means it won't pixel-match the live view. Niche.
4. **Paint previews** (Chromium's SkPicture "freeze-dried tabs" — the perfect
   primitive) are **unreachable**: no CDP surface, player only implemented on
   Android. Upstream-contribution territory only.

`capturePage()` facts for the freeze design: it copies from the *last submitted
compositor surface* (no renderer BeginFrame), works on hidden views via
`{ stayHidden: true }` (capturer-count makes the page "visible"), always returns
DPR-scaled pixels, no downscale parameter, expect ~10–40ms per view — parallelize
across pages (different frame sinks). `beginFrameSubscription` still exists
(not removed) as a poor-man's low-fps live thumbnail without OSR.

**IPC constraint that shapes all bitmap designs:** Electron IPC is
structured-clone — `ImageBitmap` can't cross, `ArrayBuffer`s are copied, no
`SharedArrayBuffer` across main↔renderer. One-shot freeze bitmaps are fine;
per-frame bitmap streaming through IPC is not. The zero/low-copy transports are
exactly two: the `sharedTexture` module and MediaStreams.

### 2.F Process & pipeline count — kill frameView, ignore the flags

- The Chromium process-model flags are dead ends: `--renderer-process-limit` is
  defeated by site isolation, `--disable-site-isolation-trials` reopens
  Spectre-class cross-site theft and breaks DevTools extensions,
  `--process-per-site` is likely defeated by Electron creating a fresh
  SiteInstance per WebContents (see
  [electron#49960](https://github.com/electron/electron/issues/49960), open Feb
  2026 — the exact missing API; worth tracking/upvoting).
- The supported lever is architectural: **frameView carries no third-party
  content and needs no isolation** — its border/backing job can be done by the
  pageView itself (background color + `setBorderRadius`, already per-view) or by
  DOM. Removing it halves renderer processes, macOS compositor pipelines, and
  per-tick setBounds calls. This is the user-visible "page bg into the wcv"
  idea, now with structural evidence (§1.2) that it's worth more than memory.
- The only thing frameView uniquely provides is a colored 1px halo *between*
  overlapping pages in the z-band (bgView DOM sits below all pages). Options:
  accept border-radius'd pageView with its own background; or draw borders in a
  single transparent overlay (Loom-style click-through window is
  production-proven — but it's one more full-screen composited layer and needs
  the click-through workaround; prefer plain DOM once substrates unify).
- Child BrowserWindows as an alternative: **dead end** (no public atomic
  multi-window commit on macOS — only private `SLSTransaction`; per-window
  WindowServer cost; Electron child-window bug history).

### 2.G Measure before building: we have never traced this

Every hypothesis above (raster storm vs surface churn vs browser-CPU vs Viz
aggregation) is separable in one trace. Methodology verified against Chromium's
[debug-janks doc](https://github.com/chromium/chromium/blob/main/docs/speed/debug-janks.md).

**Shipped:** `src/main/perf-trace.ts`, menu item **View → Record Performance
Trace** (`Cmd+Alt+Shift+P`), available in packaged builds. Toggle, perform the
gesture, toggle again (auto-stops after 30s); the Chrome-JSON trace lands in
the logs folder and is revealed in Finder.

Open the JSON in ui.perfetto.dev and compare, during the gesture:
- **`CrBrowserMain` toplevel task lengths** — main-process cost (setBounds loop,
  buildCanvasLayoutData, IPC serialization);
- **per-renderer `PipelineReporter` stage breakdowns** — is each page spending
  its time in BeginMainFrame (style/MQ), commit, or **raster**? (Raster
  dominance confirms §1.1; Layout events falsify it);
- **`VizCompositorThread`: `Display::DrawAndSwap` + `Graphics.Pipeline`
  STEP_SURFACE_AGGREGATION durations** vs page count — the per-view-pipeline
  cost of §1.2.

This should be experiment #0 — a day of instrumentation that ranks everything
else by measured, not inferred, cost. (The perf HUD measures our JS; it cannot
see raster, viz, or surface sync.)

---

## 3. What's NOT worth exploring (checked, closed)

- **`Page.setWebLifecycleState('frozen')` for visible pages** — force-hides the
  view; pixels vanish (§1.3).
- **CDP `Page.startScreencast`** — JPEG-per-frame CPU path, ~9fps reported with
  cores saturated; strictly worse than tab capture.
- **Child BrowserWindows per page**; **`--single-process`**;
  **`--disable-site-isolation-trials`** for arbitrary web content (§2.F).
- **Header-stripped iframes as the backbone** — third-party cookie partitioning
  logs users out, frame-busting JS, Google OAuth hard-blocks framing. Viable
  only as a fallback tier for localhost/cooperative sites; `<webview>` avoids
  all of it by keeping sites first-party (2.A).
- **Waiting for an Electron "transformable WebContentsView"** — no such API
  exists or is planned; adjacent issues ([#32751 native
  ScrollView](https://github.com/electron/electron/issues/32751)) have sat for
  years.
- **Syncing N native views harder** — there is no fencing primitive; drift is
  structural (§1.2). Phase-locking (canvas-motion-research §7a) can reduce it,
  never eliminate it.

---

## 4. Ranked experiment plan

Each is independent; 1–3 are days, not weeks. Order reflects
information-per-effort, and 0/1/2 can run in parallel.

0. **Trace a zoom gesture** (§2.G, ~1 day). Deliverable: measured attribution of
   zoom jank across browser CPU / renderer raster / surface sync / viz
   aggregation on a 20-page canvas. Also run the §1.1 falsification: emulation
   with constant viewSize + varying scale, no setBounds — confirm zero Layout
   events, observe the raster storm; then setBounds-only to isolate surface
   churn.
1. ~~**Webview canvas spike** (§2.A, ~1–2 days).~~ **Run, and negative** — see
   the Outcome note in §2.A. The structural bet does not pay out; treat
   §2.B (`sharedTexture`) as the surviving path to one compositor.
2. **Settle-only / quantized emulation** (§1.1 + §2.C, ~1 day, ships
   independently). During zoom gesture: freeze emulation, step it coarsely or
   only on settle; compare feel vs today. Then try
   `setVisualZoomLevelLimits(0.25, 5)` + `Emulation.setPageScaleFactor`
   per-frame via the debugger — specifically test sub-1.0 on desktop pages.
   Even partial success replaces the worst per-tick cost with pinch-zoom
   semantics.
3. **Tab-capture wall** (§2.D, ~1–2 days). N pages as MediaStreams →
   `<video>`s under the camera transform; measure CPU/GPU at N = 5/10/20/30 and
   15/30fps; confirm the activation constraint. Go/no-go for "live pixels in
   the canvas renderer" on today's Electron.
4. **`sharedTexture` end-to-end** (§2.B, ~2–4 days). One offscreen page →
   `importSharedTexture` → canvas-bg `getVideoFrame()` → WebGPU quad; then 10
   pages; measure pool pressure and IPC rate against the author's 16-page
   benchmark. Validates the long-term non-interactive tier.
5. **Frozen-DOM fidelity survey** (§2.E, ~hours). `Page.captureSnapshot` on 10
   real sites → sandboxed srcdoc iframes under the transform; grade fidelity.
   Decides whether the freeze tier can be crisp instead of bitmapped.
6. **frameView removal** (§2.F, ~1–2 days). Border/backing via pageView
   background + DOM; delete one WCV per page. Measure process count, memory,
   and gesture fps delta on a 20-page canvas — expect the §1.2 halving to show
   up directly.

## 5. How the endgames compose

The experiments feed three coherent end-states (not mutually exclusive):

- **Endgame A — one compositor (webview canvas):** ~~pages become OOPIFs in the
  canvas renderer; pan/zoom is one CSS transform; drift impossible; ADR 0014
  layering dissolves.~~ **Closed** — the spike ran and the approach does not
  work (§2.A Outcome). A single compositor is still the only thing that makes
  page and chrome move as one by construction, but `sharedTexture` (§2.B) is
  the remaining route to it.
- **Endgame B — tiered pixels under the current architecture:** interactive
  pages stay WCVs with settle-only emulation (exp 2) + frameView removed
  (exp 6); parked/idle pages become sharedTexture/MediaStream textures or
  frozen DOM riding the camera (exps 3/4/5); gesture-time freeze uses
  high-DPI captures. Incremental, ships piecewise, keeps today's input model.
  *Correction (2026-08-23): "gesture-time freeze" is not per-gesture free. It
  pays for itself where a gesture forces re-raster — zoom — and loses where one
  does not, because a parked page drops its tiles and rebuilds them on unpark.
  Measured and reverted for pan; see [ADR 0037](adr/0037-pan-does-not-freeze-its-pages.md).*
- **Endgame C — full OSR compositor:** every page a GPU texture in one
  WebGL/WebGPU scene (the Ultralight/OBS shape). Ceiling measured at
  ~16 pages@720p/54fps today, CPU-bound; input/popup/IME reimplementation is
  the real cost. Only reach for it if A fails *and* B's live tier proves
  insufficient.

The cheapest true statement in the whole investigation: **pan is an IPC/rebuild
problem we already know how to fix (ADR 0023 Phase 1 was never landed); zoom is
a raster + surface-churn problem that settle-only emulation mostly fixes; and
drift is a substrate-count problem that only unification (A) or freezing (B)
truly ends.**

*Follow-up (2026-08-23): the pan half of that sentence proved incomplete. Pan's
IPC/rebuild cost was removed (ADR 0036, camera-local projection) and pan still
felt worst — because the dominant per-frame cost was neither substrate but
canvas-bg's own grid, drawn one path fill per dot. See
[perf-zoom-pan-log.md](perf-zoom-pan-log.md) Exp F–H.*
