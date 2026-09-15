# Perf tracing

How to record and read all-process Chromium performance traces of Specular —
for humans via the UI, and for agents via the HTTP API and files on disk.
Why the categories were chosen and what the numbers mean:
`docs/pan-zoom-perf-unknowns.md` §2.G.

## What a trace captures

One recording spans **every process** (browser main, GPU/Viz, all page
renderers) with categories tuned for pan/zoom jank attribution: `viz`, `cc`,
`gpu`, `blink`, `benchmark`, `toplevel`, `input`, `latency`,
`graphics.pipeline`, `electron`, and the frame-lifecycle category behind
Perfetto's `PipelineReporter` tracks. Recording auto-stops after **30 seconds**
(the buffer fills fast); typical use is toggle → gesture → toggle.

## Recording from the UI

- **View → Record Performance Trace** (`Cmd+Alt+Shift+P`) — works in packaged
  builds. On stop, the trace is revealed in Finder.
- **Debug window → Performance** (`Cmd+Shift+D`, dev builds) — record/stop
  button, list of recorded traces, and an **Analyze** action that renders the
  summary as charts (busiest threads, top events, thread activity over time).

## Files

Traces live in the app logs folder (macOS: `~/Library/Logs/Specular/`):

```
specular-trace-<timestamp>.json          Chrome-JSON trace (open at ui.perfetto.dev)
specular-trace-<timestamp>.summary.json  cached summary (written on first analyze)
```

Both are plain JSON an agent can read directly. The summary is small; the raw
trace can be tens–hundreds of MB.

## Recording from an agent (HTTP API)

The app-control server listens on `http://localhost:29979` while the app runs.
All perf routes require the secret from `~/.specular/specular-mcp.json`:

```bash
SECRET=$(jq -r .secret ~/.specular/specular-mcp.json)

# start recording
curl -X POST http://localhost:29979/perf/trace/start \
  -H "x-specular-secret: $SECRET"

# ...drive the gesture under test (e.g. via the CLI / control API)...

# stop, analyze, and get the summary in one call
curl -X POST http://localhost:29979/perf/trace/stop \
  -H "x-specular-secret: $SECRET" \
  -H 'Content-Type: application/json' -d '{"summarize": true}'
# -> { "tracePath": "...", "fileName": "specular-trace-....json", "summary": { ... } }

# other endpoints
curl -H "x-specular-secret: $SECRET" http://localhost:29979/perf/trace/status
curl -H "x-specular-secret: $SECRET" http://localhost:29979/perf/traces
curl -H "x-specular-secret: $SECRET" \
  "http://localhost:29979/perf/trace/summary?file=specular-trace-....json"
```

## Repeatable pan/zoom test

The debug window's Performance section has a **Run test** button. It records one
trace while driving the real main-process viewport path through five fixed
profiles: slow pan, slow zoom, fast diagonal pan, slow pan + zoom, and fast pan
and zoom. The original camera is restored before the trace is saved. **Stop test**
cancels the remaining profiles, restores the camera, and saves the partial trace.

Agents can run the identical test through the control API:

```bash
SECRET=$(jq -r .secret ~/.specular/specular-mcp.json)

curl -X POST http://localhost:29979/perf/pan-zoom/run \
  -H "x-specular-secret: $SECRET" \
  -H 'Content-Type: application/json' -d '{"summarize": true}'
# -> { "cancelled": false, "tracePath": "...", "fileName": "...", "summary": { ... } }

curl -H "x-specular-secret: $SECRET" \
  http://localhost:29979/perf/pan-zoom/status

curl -X POST http://localhost:29979/perf/pan-zoom/stop \
  -H "x-specular-secret: $SECRET"
```

The run request remains open until the test and trace flush finish. Send the stop
request from a second process when cancellation is needed.

## Canvas benchmark (deterministic frame timing)

`/perf/pan-zoom/run` above answers *where* the time went. It is not the tool for
*did this change make things worse*, for three reasons: its headline number
(GPU-process busy as a share of wall-clock) saturates near 100% and stops
separating once the compositor is pinned; input is paced by `setTimeout`, so a
slow build delivers fewer steps over more wall-clock — shrinking the workload
exactly when a regression should enlarge it — and moves the denominator; and
step count tracked the display's refresh rate, so two machines drove different
workloads.

The canvas benchmark fixes the workload and reports percentiles instead:

```bash
curl -X POST http://localhost:29979/perf/canvas-bench/run \
  -H "x-specular-secret: $SECRET" \
  -H 'Content-Type: application/json' \
  -d '{"runs": 5, "warmupRuns": 1}'
```

Request fields, all optional: `phaseIds` (defaults to every phase in
`shared/pan-zoom-perf-test.ts`), `runs` (5), `warmupRuns` (1, discarded before
measuring), `trace` (also record a Chromium trace, for drilling into a number
that moved).

What makes a run comparable to the last one:

- **canvas-bg drives the gesture**, one step per `requestAnimationFrame`,
  through the same bridge calls a trackpad gesture makes. Steps per phase come
  from `BENCH_NOMINAL_FRAME_MS`, never the display, so 60Hz and 120Hz machines
  drive the same number of camera updates over the same distance. A slow build
  takes longer to cover that ground instead of covering less of it.
- **Nothing waits on main inside the loop**, so the recorded intervals are the
  renderer's own cadence rather than IPC latency.
- **Five runs, median per phase.** Median, not mean: the failure mode being
  guarded against is one run catching a background process.

What comes back, per phase (`frames`, ms): `drawFps`, `meanFrameMs`, `p50/p95/
p99FrameMs`, `maxFrameMs`, `missedFrames` (past one refresh interval, with a 5%
tolerance for vsync jitter) and `longFrames` (past 1.5x — long enough to read as
a hitch). `paintCost` is time inside the item-surface paint callback itself: a
run can hold 120fps while each paint grows, right up until the frame it doesn't.
`pageHosts` carries the offscreen page-host counters over the run —
`framesWithoutTexture` and `maxOutstandingTextures` are the pair that identified
`MAX_OUTSTANDING_TEXTURES` as the ceiling at 40 pages, so they are reported next
to frame time rather than left to a trace. Every measured run is returned under
`runs`, so spread stays inspectable behind the median.

### The fixture canvas

Numbers are only comparable if the pages are. ADR 0038's spike measured against
live websites, which makes a result irreproducible once the content changes, and
every page it tested was effectively static — which is why its JPEG-baseline
finding never generalized to animated content.

`tests/perf/fixtures/` holds six archetypes: `static-text` (no repaint),
`css-animation` (compositor-driven layers), `raf-canvas` (per-frame CPU raster),
`webgl-shader`, `webgpu-shader` (a second, distinct GPU path), and `video`. Each
is seeded by page index and advances by frame count rather than wall-clock, so
two runs do identical work. Each also names what it is actually doing in a badge
— a page that fell back to no GPU is never measured as if it were still under
load, and the WebGPU page reads a frame back from an offscreen texture before
starting its loop, so a device that acquires but renders nothing says so.

```bash
pnpm perf:fixtures                                  # serve on :8931
pnpm perf:canvas -- --pages 20 --out perf-20.canvas # generate the canvas
pnpm perf:canvas -- --pages 40 --mix static         # the old spike's conditions
```

Open the generated `.canvas`, then run the benchmark against it.

### What this cannot be

GPU compositing numbers are hardware-dependent, and shared-texture behavior
needs a real GPU — so this is not a CI gate, and an absolute number from one
machine means nothing on another. What it is: a reproducible A/B on one pinned
machine. Run it on the current build, run it on the change, compare medians.
Commit a baseline result if you want drift visible over time.

## Reading the summary

`TraceSummary` (built by `src/shared/trace-summary.ts`, all values ms):

- `threads[]` — busiest threads by **self time** (`process`, `thread`,
  `busyMs`). Where the CPU went.
- `topEvents[]` — event names by **total duration** (includes nested time).
  What the CPU was doing.
- `timeline[]` — per-bucket self-time series for the top threads
  (`bucketMs` × `bucketCount`). When it happened; spikes = hitches.
- `markers[]` — counts/totals for load-bearing signals: layout/style recalc,
  raster tasks, `Display::DrawAndSwap`, surface aggregation, compositor
  commits, device emulation.

How to attribute a janky gesture:

1. **`CrBrowserMain` busy** → main-process cost (setBounds loop,
   `buildCanvasLayoutData`, IPC serialization).
2. **Renderer threads busy with raster tasks, few layout markers** → the
   per-tick re-raster storm (see the research doc §1.1). Many layout markers
   instead → something is actually reflowing.
3. **`VizCompositorThread` heavy in draw/aggregation** → per-view compositor
   pipeline cost scaling with page count.

For anything deeper than the summary (flame charts, frame lifecycles, input
latency flows), drag the raw trace into <https://ui.perfetto.dev>.

## Caveats

- Analysis parses the whole trace on the main process — expect a brief UI
  hitch on first analyze of a large file. Traces over 500 MB are not analyzed
  (summary returns null); use Perfetto for those.
- Recording auto-stops at 30 s; start/stop responses are immediate but trace
  flushing on stop can take a moment on big captures. The debug UI shows
  **Saving…** during that flush and ignores repeated start/stop toggles.
