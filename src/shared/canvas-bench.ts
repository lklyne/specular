/**
 * Contract for the canvas pan/zoom benchmark (ADR 0038's acceptance gate).
 *
 * The benchmark is renderer-driven: canvas-bg advances the camera one step per
 * `requestAnimationFrame` through the same bridge a trackpad gesture uses, and
 * records its own frame cadence. Two properties follow, and both are the point
 * of running it this way:
 *
 * - **Fixed workload.** Steps per phase come from `BENCH_NOMINAL_FRAME_MS`, not
 *   the display's refresh rate, so a 60Hz and a 120Hz machine drive the same
 *   number of camera updates over the same distance. A timer-paced driver
 *   instead delivers *fewer* steps when the app is slow, which shrinks the
 *   workload exactly when a regression should be enlarging it.
 * - **No round trip inside the loop.** Nothing waits on main between steps, so
 *   the measured cadence is the renderer's own, not IPC latency.
 */

import type { FrameIntervalSummary, PaintCostSummary } from './frame-stats'
import type { PanZoomPerfPhase } from './pan-zoom-perf-test'

/**
 * The step interval phases are expanded against, fixed rather than read from
 * the display. This is the single knob that makes two machines comparable.
 */
export const BENCH_NOMINAL_FRAME_MS = 16

/** Settling gap between phases, matching the main-side driver's own gap. */
export const BENCH_PHASE_GAP_MS = 250

/** Runs discarded before measurement, letting caches and JIT settle. */
export const BENCH_DEFAULT_WARMUP_RUNS = 1

/** Measured runs, collapsed to a per-phase median. */
export const BENCH_DEFAULT_RUNS = 5

export interface CanvasBenchRequest {
  phaseIds?: PanZoomPerfPhase['id'][]
  runs?: number
  warmupRuns?: number
  /** Record an all-process Chromium trace around the measured runs. */
  trace?: boolean
}

export interface CanvasBenchPhaseResult {
  phase: PanZoomPerfPhase['id']
  /** Wall-clock the phase took. Informational — the workload is step-fixed. */
  durationMs: number
  steps: number
  /** Page textures that arrived from main while the phase ran. */
  framesReceived: number
  frames: FrameIntervalSummary
  paintCost: PaintCostSummary
}

export interface CanvasBenchRunResult {
  refreshMs: number
  phases: CanvasBenchPhaseResult[]
}

/**
 * Page-host counters over the run. `framesWithoutTexture` and
 * `maxOutstandingTextures` are the pair that identified the pool cap as the
 * ceiling at 40 pages, so they are reported next to frame time rather than
 * left to a trace.
 */
export interface CanvasBenchPageHostTotals {
  hosts: number
  framesReceived: number
  framesWithoutTexture: number
  framesDroppedForPoolPressure: number
  sendFailures: number
  maxOutstandingTextures: number
  releaseLatencyMs: number | null
}

export interface CanvasBenchResult {
  config: {
    phaseIds: PanZoomPerfPhase['id'][]
    runs: number
    warmupRuns: number
    nominalFrameMs: number
  }
  pageCount: number
  /** The display's measured refresh interval — recorded, never an input. */
  refreshMs: number
  /** Per-phase median across the measured runs. */
  phases: CanvasBenchPhaseResult[]
  /** Every measured run, so spread stays inspectable behind the median. */
  runs: CanvasBenchRunResult[]
  pageHosts: CanvasBenchPageHostTotals
  tracePath: string | null
}

/**
 * What the renderer is asked to drive, once armed. `runs` is the total
 * including warmup — main slices the warmup off the front, so the renderer
 * holds no policy about which runs count.
 */
export interface CanvasBenchDriveRequest {
  phaseIds: PanZoomPerfPhase['id'][]
  runs: number
  /**
   * Screen-space point the zoom is anchored at. Main computes it from the
   * window's content bounds, the way the main-side driver always has —
   * a renderer reading its own `screenX` would answer for its view, not the
   * window.
   */
  anchor: { mouseX: number; mouseY: number }
}

/** What the renderer hands back when every run has finished. */
export interface CanvasBenchDriveResult {
  refreshMs: number
  runs: CanvasBenchRunResult[]
}
