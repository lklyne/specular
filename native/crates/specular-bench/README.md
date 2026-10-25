# specular-bench

Measures the Rust app: scripted pan and zoom, frame timing, per-frame work
and process-tree memory. The profiles and the result field names follow the
Electron app's `POST /perf/pan-zoom/run`, so old numbers still read the same.

## What it holds

| Module | Purpose |
|---|---|
| `profile` | The six gesture profiles plus `idle`, `build_steps` and `select_profiles`. A port of `src/shared/pan-zoom-perf-test.ts`, with the same ids, durations and pan and zoom totals |
| `stats` | `draws`, `drawFps`, `meanFrameMs`, `p50/p95/p99FrameMs`, `maxFrameMs`, `longFrames` (interval over 1.5 x `frameMs`) |
| `recorder` | One phase's presents into a `PhaseReport`, plus `drawsWithoutTexture` |
| `work` | Each frame's time step by step and what it drew |
| `textures` | Shared-texture pool counters, named after Electron's `PageHostStats` |
| `latency` | Event timestamp to the first presented frame that reflects it |
| `memory` | Physical footprint (macOS `proc_pid_rusage`, which counts IOSurface and GPU memory) and RSS, summed over a process and its descendants |
| `breakdown` | The same tree, one row a process |
| `bench_line` | The JSON lines `--bench` prints, read by `assemble` |
| `report` | `RunReport`, the results file |
| `resize_report` | Resize ledgers (`SPECULAR_RESIZE_TRACE`) into a table |

Step counts match the TypeScript: at 120 Hz `slow-pan` is 240 steps, at the
16 ms fallback 125. `Duration` holds whole nanoseconds, so `step_count`
ignores remainders under a thousandth of a step.

## Commands

```
specular-bench plan            [--profiles a,b] [--duration-ms N] [--frame-ms N]
specular-bench assemble        <bench.jsonl> [--fixture NAME] [--pages N]
                               [--memory-idle f] [--memory-end f] [--memory-peak f]
specular-bench rss             --pid N [--peak-ms N | --per-process true]
specular-bench resize-report   <ledger.jsonl>...
```

The `--memory-*` files are what `rss` writes.

## Reading a run

The app's stdout lines are `BenchLine`s (`bench_line.rs`, shared with
`assemble`, so a renamed field fails loudly): one per profile, a
`PhaseReport` plus `source`, `pages`, `representative`, `stepIntervalMs`,
`maxPaintToSubmitMs` and `paintPolicy`; and, when any input reached a page,
one `{"inputLatency": {...}}` line at exit. `assemble` refuses lines from
different sources, page counts, refresh intervals or paint policies, and
marks the report non-representative when any line says so (the synthetic
source, or a frame that went through a CPU upload).

The copy-paste sequence (build, bundle, three runs per fixture, memory
samples, `assemble`) is in [`native/README.md`](../../README.md#bench).

## Not covered

- **End-of-run memory.** The app exits when the bench finishes. Sample it
  from inside the app, or keep it open after the bench.
- **Input latency during a bench run.** `--bench` drives only the camera, so
  no page input is forwarded. Forwarded-input latency comes from an
  interactive session, whose `inputLatency` line `assemble` folds in.
  Gesture (wheel to present) latency is not measured.
