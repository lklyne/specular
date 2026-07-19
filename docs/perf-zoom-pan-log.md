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

## Summary of wins so far
- **Zoom (primary):** raster ms −80%, GPU main −12% (Exp A). Remaining lever =
  zoom IPC broadcast (~1720ms), deferred as risky.
- **Pan (secondary):** scene-broadcast IPC eliminated, renderer main −48% (Exp B).
- Both coexist; no regressions. Tunables: `BUCKETS_PER_OCTAVE`, `SETTLE_MS`.
