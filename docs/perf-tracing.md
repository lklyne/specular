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
  flushing on stop can take a moment on big captures.
