# Canvas pan/zoom perf — iteration log

Orchestrator log for the zoom-first (pan-second) perf improvement effort. Newest
entries at the bottom of each section. Numbers come from the automated pan/zoom
perf test (`POST /perf/pan-zoom/run`, 5 fixed profiles, real main-process
viewport path) summarized by `src/shared/trace-summary.ts`.

## Feedback loop mechanics

- Restart dev app: `herdr pane send-keys w2:p4 C-c` then `herdr pane run w2:p4 "pnpm dev"`.
- App-control HTTP on `localhost:29979`, secret from `~/.specular/specular-mcp.json`
  (`x-specular-secret` header).
- Run test: `POST /perf/pan-zoom/run {"summarize":true}` → `{fileName, summary}`.
- App CDP for driving UI: `localhost:9333` (agent-browser connect 9333).

### Blocker discovered (2026-07-18)
- `getTraceSummary` returns **null** for traces over `MAX_SUMMARIZABLE_BYTES = 500MB`
  (`src/main/perf-trace.ts:46`). Synchronous 1.3GB parse on main is too disruptive —
  raising the cap is the wrong fix.
- First run had 15 live external pages (Amazon, LinkedIn, GitHub, Medium…) →
  **1.33 GB** trace → summary null. Unusable for the loop.
- Fix: use a **controlled benchmark scene** — N identical deterministic static
  pages served locally. Keeps the trace summarizable AND makes A/B runs comparable
  (external sites add network/ad-render noise). Scene held fixed across iterations.

## Research digest (from docs/pan-zoom-perf-unknowns.md + ADR 0023 + PRs)

### Zoom root causes (PRIMARY focus)
1. **Per-tick `enableDeviceEmulation({scale})` → whole-tree re-raster storm**
   (`SetNeedsRecalculateRasterScales`), no pinch-style throttling. Dominant cost.
2. **Per-tick `setBounds` resize → new viz `LocalSurfaceId` every tick** → full
   output-surface realloc + full-frame redraw. Co-dominant.
3. **`setPan`/`setZoom` mark `canvas` dirty → `buildCanvasLayoutData` re-maps/sorts/
   serializes every entity+edge+cursor over IPC to 3 webContents each tick** even
   though the entity list didn't change.
4. **`aboveView` has no rAF batching** → `SelectionOutlineLayer` re-runs 8 full-array
   filters per payload (fresh layout ref each tick).
5. Multi-compositor drift on macOS: each page = 2 `WebContentsView`s (frameView +
   pageView) = 2 full compositor pipelines.

### Ranked experiments (zoom first)
1. **[#0] Trace + attribute a zoom gesture** — measure, don't infer. ← IN PROGRESS.
2. **Settle-only / quantized emulation** — S, low risk, ships alone. Freeze
   `enableDeviceEmulation` during gesture; step coarsely or on settle only.
   NOTE: unmerged branch `perf/settle-only-emulation` already prototypes this
   (freeze pages→bitmaps during zoom). Also unmerged: `claude/pan-zoom-smoothness`
   (optimistic zoom/pan delta correction).
3. **frameView removal** — M, low risk. Halves compositor pipelines per page.
4. **De-dirty the viewport (ADR 0023 Phase 1)** — M, low risk, PAN-focused. Stop
   `markDirty('canvas')` on pan/zoom; rAF-batch `aboveView`; drive scene transform
   from `viewport-nudge`. "The pan fix we already know how to do", never landed.
5. `<webview>`/OOPIF single-compositor spike — M/L, higher risk, structural.

### Already tried & abandoned — do NOT re-attempt naively
- **ADR 0023 renderer-owned camera (Rejected 2026-07-01, PR #277 closed)**: the
  `setBounds`-only "texture scales smoothly mid-zoom" assumption was FALSE — content
  is pinned to `viewSize × scale`, so a live page doesn't visually scale until
  settle re-emulation. HUD stats came out worse than baseline. If revisited, start
  from snapshot-freeze, not a setBounds/transform trick.
- `Page.setWebLifecycleState('frozen')` on visible pages: pixels vanish, dead end.
- CDP `Page.startScreencast`: ~9fps, worse than tab-capture.
- Child BrowserWindows / `--single-process` / site-isolation flags: no atomic
  multi-window commit on macOS; reopens Spectre holes.

### Key hot-path files
- `src/main/runtime/layout-engine.ts:369-452` `layoutAllViews` — native `setBounds`
  loop, `pageEmulationKey` includes `zoom`.
- `src/main/runtime/viewport-control.ts` `setPan`/`setZoom`/`moveCameraTo` (16ms tween).
- `src/main/runtime/runtime-geometry.ts` — screen-rect projection, `viewSize`/
  `deviceScaleFactor` → `enableDeviceEmulation`.
- `src/main/runtime/page-factory.ts:73-127` — frameView/pageView/chromeView per page.
- `src/renderer/canvas-bg/useCanvasLayoutState.ts` — rAF-coalesced setState.
- `src/main/perf-trace.ts`, `src/main/pan-zoom-perf-test.ts` — the loop itself.

## Measurement strategy (revised after two failed baseline attempts)

- **Trace size is NOT driven by page content.** 15 heavy external pages → 1.33 GB;
  9 light data:-URL pages → 1.43 GB. The ~7s of gesture (5 profiles) emits a
  colossal event volume from the device-emulation re-raster storm itself. So a
  "lighter scene" does not make traces summarizable.
- Trace categories (`viz`,`cc`,`gpu`,`blink`,…) are load-bearing for attribution;
  trimming them to shrink the file would break `trace-summary`.
- **Decision: primary loop signal = a lightweight main-side numeric metric**
  (mean/p95 per-tick cost of `layoutAllViews` / `buildCanvasLayoutData` / setBounds
  loop) returned directly from `runPanZoomPerfTest` JSON — no trace parse. The top-3
  zoom root causes are all main-driven, so main-side tick cost is a strong proxy.
- Full 1.4 GB traces reserved for occasional deep GPU/compositor attribution
  (would need a zoom-only <500 MB slice or a streaming parser — deferred).

### Benchmark scene (reproducible)
- 3×3 grid of one deterministic `data:`-URL static page (scratchpad/bench/index.html,
  rebuilt by scratchpad/build-scene.sh). No network, no timers → stable A/B.
- Before every run: `specular focus <center page>` for identical start camera.

## Instrumentation added (loop enablers)
- `POST /perf/pan-zoom/run` now accepts `{profiles: string[], durationMs: number}`:
  run a subset of phases at a custom duration → keeps the trace summarizable.
  A `slow-zoom` at `durationMs:1400` on the 9-page scene → ~381 MB (< 500 MB cap).
- Response now returns `buildStats {n,mean,p95,max}` (buildCanvasLayoutData ms),
  no trace parse needed.
- Measurement helper: `scratchpad/perf-run.sh <label> [profile] [durationMs]`
  re-reads the secret (it ROTATES on every app restart), sets identical start
  camera, runs, and prints the scoreboard.

## Baseline — slow-zoom, 1400ms, 9-page data: grid

Scoreboard metric = **counts + thread self-times** (stable ±2-3%). Raster
`totalMs` is NOISY (cold first run 2857ms vs warm ~1700ms) — use COUNT not ms.

| metric | run1(cold) | run2 | run3 | warm baseline |
|---|---|---|---|---|
| raster_tasks count | 7062 | 7376 | 7096 | **~7180** |
| raster_tasks ms | 2857 | 1799 | 1673 | ~1740 (noisy) |
| layout_recalc count | 14032 | 14548 | 14250 | **~14270** |
| layout_recalc ms | 537 | 565 | 552 | ~555 |
| compositor_commits | 9003 | — | — | ~9000 |
| gpu_main_ms | 2384 | 2492 | 2417 | **~2430** |
| renderer_ms (sum) | 2957 | 3003 | 2867 | **~2910** |
| buildMs mean | 0.11 | — | — | ~0.11 (negligible) |

**Attribution:** GPU raster storm dominates. In a 1.4s zoom the emulation churn
forces **~7180 raster tasks + ~14270 layout/style recalcs**, ~2430ms GPU-main +
~2910ms renderer-main self-time. buildCanvasLayoutData is negligible here (9
simple pages) — root cause #3 only bites with many entities.
Top events: GPU CommandBuffer flushes + `ContextBridge::PassValueToOtherContext`
(IPC broadcast). Discard cold run 1 for ms comparisons.

## Iterations

### Exp A — Quantized-key device emulation during zoom motion (BUCKETS_PER_OCTAVE=4, SETTLE_MS=120)
- **Files:** new `src/main/runtime/zoom-motion.ts`; `layout-engine.ts:454` (`pageScale = isZoomInMotion() ? quantizeZoomForEmulation(zoom) : zoom` — quantizes the emulation *cache key*); `viewport-control.ts:76` (`markZoomMotion(() => requestLayout())` in setZoom); unit test `tests/unit/zoom-motion.test.ts`.
- **Mechanism:** the per-page emulation cache key uses a log2-quantized zoom during
  motion, so `enableDeviceEmulation` re-fires only at bucket crossings (~every 19%
  zoom change) instead of every 16ms tick. When it fires it still applies the EXACT
  zoom (`computeApplyEmulation` uses `input.zoom`), so pages stay crisp — just
  re-rastered ~4-5× per gesture instead of ~87×. On settle, motion exits and one
  exact re-emulation runs.
- **Measured (3 clean warm runs, slow-zoom 1400ms, 9 pages):**

  | metric | baseline | exp-A | delta |
  |---|---|---|---|
  | raster count | ~7180 | ~4175 | **−42%** |
  | raster ms | ~1740 | ~347 | **−80%** |
  | gpu_main_ms | ~2430 | ~2150 | −12% |
  | layout_recalc count | ~14270 | ~15800 | +11% |
  | compositor_commits | ~9000 | ~10400 | +16% |
  | renderer_ms | ~2910 | ~3155 | +8% (noisy) |

- **Read:** big win on the dominant cost (raster/GPU). The small rises in
  layout-recalc & commits suggest pages relayout to track the view each tick but
  skip re-raster between buckets — the desired pinch-like behavior, cheap costs.
- **Status:** PROVISIONAL KEEP. Mid-gesture screencapture at 26% zoom shows the
  9 pages crisp and correctly framed — no gross shear. Open item: human eyeball a
  fast interactive zoom + tune BUCKETS_PER_OCTAVE. Settled state crisp.
- **Untouched by A:** IPC scene-broadcast cost — `ContextBridge::PassValue` ~1680ms
  and `MessagePort::Accept` ~2000ms are flat (A only touched emulation). That's the
  next lever (Exp B).

## Pan baseline — slow-pan, 1400ms, 9-page grid

| metric | run1 | run2 | run3 | ~avg |
|---|---|---|---|---|
| raster count | 2714 | 2706 | 2756 | ~2725 (tiny — pan doesn't re-raster) |
| layout_recalc count | 6396 | 6208 | 6325 | ~6300 |
| gpu_main_ms | 1843 | 1666 | 1789 | ~1766 |
| renderer_ms | 2247 | 2250 | 2288 | ~2262 |
| ipc_contextbridge_ms | 1645 | 2042 | 2056 | ~1900 (noisy) |
| ipc_messageport_ms | 1222 | 1216 | 1243 | ~1227 |

**Attribution:** pan cost is dominated by the per-tick **scene rebuild+broadcast**
(ContextBridge ~1900ms + MessagePort ~1227ms) and GPU/renderer — NOT raster. This
is the de-dirty target and confirms Exp B helps pan most, zoom secondarily.

## Exp B (planned) — de-dirty the viewport on pure camera changes
Hypothesis: `setPan`/`setZoom` mark `canvas` dirty → `buildCanvasLayoutData` +
full-scene contextBridge broadcast to 3 renderers every tick, even though the
entity list is unchanged. Native page positioning (setBounds/emulation) must still
run per tick, but the SCENE payload broadcast can be skipped on pure camera moves —
the renderer already receives `broadcastViewportNudge()` and can transform its
existing scene. ADR 0023 *Phase 1* (the low-risk part; the rejected part was the
later GPU-composite phases).

### Exp B — Pan de-dirty (SHIPPED)
- **Mechanism:** `applyViewportInputDelta` already calls `requestLayout()` every
  tick, and `layoutAllViews` runs native page `setBounds` UNCONDITIONALLY —
  `markDirty('canvas')` only gates the scene-payload rebuild+broadcast
  (layout-engine.ts:251). So `setPan` no longer marks canvas dirty; the DOM scene
  rides the existing `broadcastViewportNudge()` CSS translate, and a `markPanMotion`
  settle re-baselines (markDirty+requestLayout once) ~120ms after pan stops.
- **Files:** `zoom-motion.ts` (+`markPanMotion`); `viewport-control.ts:86` setPan
  (drop `markDirty('canvas')`, add settle + import).
- **Measured (3 warm runs, slow-pan 1400ms):**

  | metric | pan baseline | exp-B | delta |
  |---|---|---|---|
  | ipc_contextbridge_ms | ~1900 | **0** | **eliminated** |
  | ipc_messageport_ms | ~1227 | ~706 | −42% |
  | renderer_ms | ~2262 | ~1177 | **−48%** |
  | layout_recalc count | ~6300 | ~4732 | −25% |
  | gpu_main_ms | ~1766 | ~1595 | −10% |
  | raster count | ~2725 | ~2124 | −22% |

- **Visual:** mid-pan screencapture at 17% — 9 pages hold a perfect grid, no
  native-vs-DOM drift, selection chrome stays aligned (rides the container translate).
- **Zoom regression check:** re-ran slow-zoom after exp-B — raster 4174, gpu 2132
  (identical to exp-A). No regression; zoom's IPC (~1720ms) untouched (zoom de-dirty
  NOT done — nudge is pan-only; would need a zoom-scale transform = the risky
  rejected-ADR territory, deferred).
- **Status:** KEEP. Open item: human eyeball pan with active selection + agent
  cursors (cursor overlay is a separate window not yet on the nudge channel — may
  lag during pan until settle; not present in benchmark).

### Exp C — Zoom de-dirty via translate+scale scene transform (SHIPPED)
- **Mechanism:** extended the pan-only nudge hook into a full camera transform.
  `useScenePanOffset` → `useSceneCameraTransform` now returns `{x,y,scale}` with
  `s = nudge.zoom/payload.zoom`, `x = nudge.pan.x − s·payload.pan.x` (same for y),
  applied as `translate3d(x,y,0) scale(s)` with `transform-origin:0 0` on both
  scene containers (canvas-bg + above-view). This exactly maps payload-baseline
  screen coords to the live camera, so the DOM scene tracks the native pages
  (which move via exact per-tick setBounds) with NO rebuild. `setZoom` drops
  `markDirty('canvas')` (keeps `'toolbar'`); the zoom settle re-baselines
  (markDirty('canvas')+requestLayout) → snaps crisp when zoom stops.
- **Why this isn't the rejected ADR:** ADR 0023 rejected a *permanent*
  renderer-owned GPU-composited camera (worse HUD stats, setBounds-can't-scale
  assumption). This is a *motion-only* CSS transform that self-reconciles to
  identity on settle; main-side setBounds still authoritative. Contained + revertible.
- **Files:** `useScenePanOffset.ts` (rewritten/renamed `useSceneCameraTransform`);
  `canvas-bg/App.tsx:110`, `above-view/App.tsx:992` (transform+origin);
  `viewport-control.ts` setZoom.
- **Measured (slow-zoom 1400ms):** `ipc_contextbridge_ms` **~1720 → 0**,
  `ipc_messageport_ms` ~1965 → ~1615 (−18%), layout_recalc ~15800 → ~14100.
  Raster unchanged from Exp A (~4-5k, still quantized).
- **Combined slow-pan-zoom (A+B+C):** `ipc_contextbridge_ms` = **0** (real-world
  gesture broadcast eliminated), raster count ~1767 (vs ~2466 A+B).
- **Visual:** mid-zoom (30%) + settled (21%) screencaptures — pages crisp,
  selection chrome frames the page with NO native-vs-DOM shear, grid consistent,
  crisp snap on settle.
- **Status:** KEEP (provisional). Benchmark = pages + selection chrome only.
  HUMAN eyeball needed on real content before shipping: DOM text/shape/drawing
  entities (CSS-scaled → blurry mid-zoom, crisp on settle — expected), edges,
  annotations, and especially **agent-presence cursors** (separate overlay window,
  NOT on the nudge channel → will lag during zoom until settle; wire it up before
  ship). Also test fast pinch + extreme zoom ratios.

## Summary of wins (A+B+C on branch perf/zoom-emulation-pan-dedirty)
- **Zoom (primary):** raster ms **−80%** (A) + zoom scene-broadcast IPC **eliminated** (C).
- **Pan (secondary):** scene-broadcast IPC **eliminated**, renderer-main **−48%** (B).
- **Combined pan+zoom (real-world):** ContextBridge IPC **0**, raster kept low.
- No regressions across pan / zoom / combined; visuals verified. Tunables:
  `BUCKETS_PER_OCTAVE`, `SETTLE_MS` in `zoom-motion.ts`.

## Manual test checklist (before merge)
1. Interactive pinch/scroll zoom on a canvas with **text + shape + drawing entities**
   and **edges** — confirm entities/edges track the camera, blur-then-crisp on
   settle is acceptable, no permanent drift.
2. Zoom/pan with an **active selection** — selection chrome stays glued to entities.
3. Zoom/pan with **agent-presence cursors** live — expected to lag (known gap);
   decide whether to wire cursorOverlayWindow onto the nudge before ship.
4. Fast flick pan + fast pinch zoom — no stuck/blurry frames after motion stops.
5. Undo/redo across a zoom (viewport isn't undoable, but confirm no desync).
6. Multi-monitor / non-retina (nativeScale differs) — emulation quantization still crisp.

## Exp D — snapshot refresh pipeline, link by link (2026-08-20)

Harness: `scripts/zoom-snapshot-bench.sh [repeats] [zoom]` → `POST /perf/zoom-snapshot/bench`
(`src/main/runtime/zoom-snapshot-bench.ts`). Frames all pages in the window (or
sets `zoom`), then times capturePage → encode (sync, main thread) → IPC → renderer
decode for four encodings against the same captures. 20 Wikipedia pages, Retina
(DPR 2), medians of 3 runs, ms.

| zoom | captured | px/page   | PNG enc (max) | JPEG85 enc (max) | raw enc | PNG MB | JPEG85 MB | raw MB | JPEG85 decode |
|------|----------|-----------|---------------|------------------|---------|--------|-----------|--------|---------------|
| fit  | 20       | 144×312   | 73 (4.8)      | 11 (0.6)         | 23      | 1.4    | 0.7       | 10     | 7             |
| 0.25 | 20       | 196×426   | 123 (7.9)     | 18 (1.1)         | 42      | 2.1    | 1.3       | 19     | 10            |
| 0.5  | 5        | 1280×800  | 128 (28)      | 17 (3.6)         | 43      | 1.8    | 1.0       | 20     | 8             |
| 1.0  | 3        | 2560×1600 | 218 (91)      | 34 (12)          | 103     | 2.6    | 1.5       | 47     | 14            |

Capture itself: 4–13ms per page, parallel, wall ≈ 6–20ms regardless of count.
IPC for data URLs: 0–2ms. Raw bitmap IPC: 7–37ms (structured-clone copy of 10–47MB).

Findings:

1. **PNG encode is the cost, and it is synchronous on the main process thread.**
   120–220ms per prepare at normal zooms; a single 2560×1600 page is 91ms. JPEG-85
   is 6–7× cheaper (17–34ms total) and decodes in under 15ms. Raw bitmap is out:
   20–47MB per prepare through IPC, and `toBitmap()` itself is slower than JPEG.
2. **`capturePage()` captures at the view's on-screen size × DPR, not the page's
   content size.** Cost scales with window area, not page count (20 tiny pages
   cost less than 3 big ones). It also means a snapshot taken zoomed-out has no
   pixels to zoom *into*: at zoom 0.25 each page is 196×426, and scaling that to
   zoom 1 is a 4× blow-up. Capture resolution is the crispness ceiling, and it
   is set by the live zoom at capture time.
3. Total pixel budget per prepare is bounded by ~window × DPR (views tile the
   window), so the worst case is a few large pages at zoom ≥ 1, not many pages.

## Exp E — hi-res settle captures over CDP (2026-08-21)

Problem: the raster you zoom with is the last settle's `capturePage()`, which
copies the compositor surface at on-screen size (`css × zoom × dpr`). Settle
at zoom 0.1 and a 1194px page is a 239px bitmap; zoom back in and it is a 10×
blow-up. The cache also replaced a zoom-1.0 frame with that 239px one, so
even a page you had just seen at 1:1 went blurry.

Probe (`POST /perf/zoom-snapshot/cdp-probe {scale, save}`): `Page.captureScreenshot`
with `clip.scale` rasters in the renderer at `css × scale × dpr`, independent of
the emulation zoom (zoom 0.618, scale 1 → 2388px for a 1194px page). 25–100ms
per page at a 2048px long edge, JPEG encoded in the renderer, not on main.

Two things it does that `capturePage` does not:

1. It installs its own metrics override and then restores the DevTools
   handler's *previous* params, which are empty when Electron's
   `enableDeviceEmulation` set the widget state. Without a fix the page snaps
   to native scale and stays there (the emulation key is unchanged, so layout
   never re-applies it). Fix: seed `Emulation.setDeviceMetricsOverride` with the
   same params before the screenshot; the restore then lands on the right
   state. Scroll position survives (checked at scrollY 900). A debugger detach
   now clears emulation too, so the detach listener resets the emulation key.
2. It re-lays the page out at the target scale for the capture. On a visible
   page that is a visible flip, so the capture only runs in the settle warm
   park, where it is hidden behind the raster. It works at zero bounds too.

Second finding, unrelated to CDP but exposed by it: the warm park at
`x = -20000` never presents. viz stops issuing BeginFrames to a surface that
is entirely outside the window, so `requestAnimationFrame` fires once or
twice after parking and then stops. The presentation gate was timing out
(750ms, then revealing blind) on most settles; a 1px row under the toolbar is
culled the same way. A 1px column inside the window's left edge keeps frames
flowing: all 7 pages present in 35–70ms.

Shipped:
- `captureParkedPagesAtSettle`: park → 2 rAF → CDP hi-res (if needed) → 2 rAF →
  `capturePage` presentation frame → reveal. Target scale puts the page's long
  edge at `SNAPSHOT_MAX_EDGE_PX` (2048), capped at max zoom, never below the
  live zoom; at or above the target the presentation frame is the snapshot.
- Frames carry a `contentKey` (page, viewport, nav generation, scroll). A
  page keeps its frame when the incoming one pictures the same content at a
  lower resolution, so a zoom-out never downgrades what a zoom-in captured,
  and a background `capturePage` prepare after a drag keeps the hi-res frame.
- The CDP capture is skipped when the retained frame already meets the
  target, so repeat zooms cost only the presentation wait.
- `POST /perf/flags {zoomSnapshotHiRes: false}` disables the CDP path for A/B.

Measured (7 pages, cold start at zoom 0.1): first settle 216ms wait with all
7 hi-res captures (118–182ms each, parallel); every later settle 35–70ms
with zero CDP captures; frame widths held at 2388/2880 through a 1.0 → 0.1
→ 1.0 cycle. Mid-gesture window frames at zoom 1.0 from a 0.2 raster are
crisp.

Still owed: a decoded-bitmap budget (20 pages at a 2048px edge is ~200MB of
GPU textures; most canvases have fewer visible pages, but nothing enforces
it), and a human check that the 1px park column is invisible in practice.

### Addendum — one emulation owner (2026-08-21)

Finding 1 above (seeding the DevTools handler so its restore lands right) was
a two-owner workaround, and it leaked: after a resize the restore could land
on a scale the layout engine's emulation key still believed was current, and
`clip.scale` re-laid the page out larger than its view, which the native layer
does not clip (an oversized page hanging off the warm park, and a page
rendering ~1.3× past its selection outline after resize → zoom → select).

Now `page-emulation.ts` is the only party that sets page metrics, through
Electron's `enableDeviceEmulation`; nothing touches CDP's
`Emulation.setDeviceMetricsOverride` any more (tried as the owner's backend:
the DevTools handler also resizes the widget view to the override size, and
pages rendered tiny during every zoom). The hi-res capture borrows metrics
through `withCaptureMetrics`, which raises only `deviceScaleFactor` (CSS
viewport and view-space size unchanged, so the frame never outgrows its
bounds), copies the surface with `capturePage`, and restores from the
owner's record. JPEG encode of that copy is on main like every other frame.
The warm park is back to a plain one-pixel column.

The encode bench, the CDP probe route, the `/perf/flags` A/B switches, and the
`[zoom-snap]` lifecycle logging were removed before merge. The log above keeps
the numbers they produced; re-create them from git history if a follow-up needs
to measure again.
