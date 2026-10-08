# specular-bench

Measures the Rust/CEF shell the way the Electron app measures itself, and
puts both on one table. The plan, pass/fail criteria and results table are in
[`docs/plans/rust-cef-spike.md`](../../../docs/plans/rust-cef-spike.md).

## What it holds

| Module | Mirrors | Purpose |
|---|---|---|
| `profile` | `src/shared/pan-zoom-perf-test.ts` | The six gesture profiles (same ids, durations, pan/zoom totals), `build_steps` (= `buildPanZoomPerfSteps`), `select_profiles` (= the `profiles` / `durationMs` request fields) |
| `stats` | lab `summarizeFrameIntervals`, `computeBuildStats` | `draws`, `drawFps`, `meanFrameMs`, `p50/p95/p99FrameMs`, `maxFrameMs`, `longFrames` (interval > 1.5 x `frameMs`) |
| `recorder` | lab `runLabBenchmark` | One phase's presents into a `PhaseReport`, plus `drawsWithoutTexture` |
| `textures` | `PageHostStats` in `src/main/runtime/page-host.ts` | `framesReceived`, `popupFrames`, `framesWithoutTexture`, `framesDroppedForPoolPressure`, `outstandingTextures`, `maxOutstandingTextures` (read from `/perf/page-hosts` snapshots) |
| `latency` | — | Event timestamp to the first presented frame that reflects it |
| `memory` | — | Physical footprint (macOS `proc_pid_rusage`, the memory verdict's metric: it includes IOSurface and GPU memory) and RSS, summed over a process and all its descendants found via `ps` (same code for both shells) |
| `report` | — | `RunReport`, the results file |
| `electron_trace` | `docs/perf-tracing.md` | Electron trace -> `RunReport` |
| `compare` | — | Two results files -> markdown |

Step counts match the TypeScript exactly: at 120 Hz `slow-pan` is 240 steps,
at the 16 ms fallback 125. (`Duration` holds whole nanoseconds, so
`step_count` ignores sub-millistep remainders that the float-ms TypeScript
never sees.)

## Commands

```
specular-bench plan            [--profiles a,b] [--duration-ms N] [--frame-ms N]
specular-bench electron-trace  <trace.json> | --response run.json  [options]
specular-bench assemble        <bench.jsonl> [options]
specular-bench rss             --pid N [--peak-ms N | --per-process true]
specular-bench compare         <baseline.json> <candidate.json>
```

Options shared by `electron-trace` and `assemble`: `--fixture NAME`,
`--pages N`, `--memory-idle f`, `--memory-end f`, `--memory-peak f` (files
written by `rss`), `--page-hosts-before f`, `--page-hosts-after f` (bodies
of Electron's `GET /perf/page-hosts`). `electron-trace` also takes
`--frame-ms`, `--profiles`, `--duration-ms` (must match what was run),
`--gap-ms` (default 200), `--thread` (default `VizCompositorThread`) and
`--paint-policy` (default `electron-lod`, the shipped page-host LOD; pass
`full-rate` if the Electron run had its LOD disabled).

`compare` reads a `RunReport`, a bare array of phase objects (the ADR 0038
lab's results), or JSON lines (the app's `--bench` output). Concatenate three
runs into one file and each cell becomes the median, as the plan asks. It
warns when the runs used different paint policies, lists every run note
(including the Electron present-count check), and shows footprint rows
above RSS rows: judge memory by footprint.

## Capturing the Electron baseline

Run a release build of Specular on `main`, open the fixture `.canvas` in the
active tab, and leave the window still. The control server is on
`localhost:29979`; every perf route needs the secret.

```sh
SECRET=$(jq -r .secret ~/.specular/specular-mcp.json)
H="x-specular-secret: $SECRET"
PID=$(pgrep -xo Specular)   # main process; `pgrep -xo Electron` under pnpm dev

# idle memory and texture counters, after pages have settled
specular-bench rss --pid "$PID" > e-mem-idle.json
curl -s -H "$H" localhost:29979/perf/page-hosts > e-hosts-before.json

# the scripted test: six profiles, stepped at the display refresh interval,
# one all-process trace (blocks until the trace is flushed)
curl -s -X POST localhost:29979/perf/pan-zoom/run -H "$H" \
  -H 'Content-Type: application/json' -d '{}' > e-run.json

specular-bench rss --pid "$PID" > e-mem-end.json
curl -s -H "$H" localhost:29979/perf/page-hosts > e-hosts-after.json

specular-bench electron-trace --response e-run.json \
  --fixture static-20 --pages 20 \
  --memory-idle e-mem-idle.json --memory-end e-mem-end.json \
  --page-hosts-before e-hosts-before.json --page-hosts-after e-hosts-after.json \
  > electron.json
```

For a peak during the run, start `specular-bench rss --pid "$PID" --peak-ms
20000 > e-mem-peak.json &` just before the `run` request and pass
`--memory-peak`.

How `electron-trace` reads frames: presented frames are the
`Display::DrawAndSwap` slices on the busiest `VizCompositorThread`. The trace
has no phase markers, so phases are recovered from the 250 ms idle gaps the
test leaves between profiles, and the run of bursts whose lengths best match
the profiles' planned durations is used (a leading page-settle burst or the
trailing camera restore is skipped). That needs the gaps to be quiet:

- **Static fixtures:** run all six profiles in one request, as above.
- **Animated fixtures** (pages repaint through the gaps): one profile per
  request, `-d '{"profiles":["slow-pan"]}'`, then `electron-trace
  --profiles slow-pan`. The burst then also contains animation frames from
  before and after the gesture; read those numbers as "frames composited
  while animating", not gesture-only.

If segmentation fails, the error says how many bursts it found.

## Capturing the Rust run

The copy-paste sequence (build, bundle, three runs per fixture, memory
samples, `assemble`, `compare`) is in
[`native/README.md`](../../README.md#morning-run-on-macos-apple-silicon).
The app's stdout lines are `BenchLine`s (`bench_line.rs`, shared with
`assemble`, so a renamed field fails loudly): one per profile, a
`PhaseReport` (`phase`, `durationMs`, `draws`, ..., `drawsWithoutTexture`,
`framesReceived`, `textures`) plus `source`, `pages`, `representative`,
`stepIntervalMs`, `maxPaintToSubmitMs` and `paintPolicy`; and, when any
input reached a page, one `{"inputLatency": {...}}` line at exit.
`assemble` refuses lines from different sources, page counts, refresh
intervals or paint policies.

`assemble` marks the report non-representative when any line says so (the
synthetic source, or any frame that went through a CPU upload); `compare`
prints a warning above the table for such runs, and the plan excludes them.

## Not covered yet

- **End-of-run memory for the Rust app.** The app exits when the bench
  finishes; sample it from inside the app, or keep it open after the bench.
- **Latency in the Electron baseline.** `/perf/pan-zoom/run` calls the
  viewport mutator directly, so the trace has no `EventLatency` slices for the
  gesture, and the forwarded-input fixture needs manual trace reading.
- **Per-phase texture counters for Electron.** `/perf/page-hosts` is
  cumulative, so only whole-run deltas are available.
- **Input latency during a Rust bench run.** `--bench` drives only the
  camera, so no page input is forwarded. Forwarded-input latency comes from
  an interactive session: the app prints an `inputLatency` line when it
  closes, and `assemble` folds it into the report. Gesture (wheel ->
  present) latency is measured in neither shell.
- **Electron present counting.** `electron-trace` counts every
  `Display::DrawAndSwap` on the viz thread. If offscreen page windows draw
  there too, fps reads high; the converter notes any phase with far more
  presents than steps, and its numbers stay unverified until a real trace
  shows about steps + 1 presents per phase.
