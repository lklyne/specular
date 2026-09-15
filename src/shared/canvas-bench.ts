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

import {
  medianPaintCost,
  medianSummary,
  roundPaintCost,
  roundSummary,
  type FrameIntervalSummary,
  type PaintCostSummary,
} from './frame-stats'
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

/** One offscreen page host's own counters, as the host keeps them. */
export interface PageHostStats {
  pageId: string
  framesReceived: number
  popupFrames: number
  framesWithoutTexture: number
  framesDroppedForPoolPressure: number
  sendFailures: number
  outstandingTextures: number
  maxOutstandingTextures: number
  releaseLatencyMs: number | null
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

/**
 * Page-host counters over one run. Cumulative fields are differenced against
 * their pre-run values so a long-lived session's totals don't leak into a
 * single run's numbers; `maxOutstandingTextures` is a high-water mark, so it
 * is taken as a max rather than a difference. A host created mid-run has no
 * baseline and contributes its whole count.
 */
export function diffPageHostStats(
  before: readonly PageHostStats[],
  after: readonly PageHostStats[],
): CanvasBenchPageHostTotals {
  const baseline = new Map(before.map((stats) => [stats.pageId, stats]))
  const totals: CanvasBenchPageHostTotals = {
    hosts: after.length,
    framesReceived: 0,
    framesWithoutTexture: 0,
    framesDroppedForPoolPressure: 0,
    sendFailures: 0,
    maxOutstandingTextures: 0,
    releaseLatencyMs: null,
  }
  const latencies: number[] = []
  for (const stats of after) {
    const was = baseline.get(stats.pageId)
    totals.framesReceived += stats.framesReceived - (was?.framesReceived ?? 0)
    totals.framesWithoutTexture += stats.framesWithoutTexture - (was?.framesWithoutTexture ?? 0)
    totals.framesDroppedForPoolPressure +=
      stats.framesDroppedForPoolPressure - (was?.framesDroppedForPoolPressure ?? 0)
    totals.sendFailures += stats.sendFailures - (was?.sendFailures ?? 0)
    totals.maxOutstandingTextures = Math.max(
      totals.maxOutstandingTextures,
      stats.maxOutstandingTextures,
    )
    if (stats.releaseLatencyMs !== null) latencies.push(stats.releaseLatencyMs)
  }
  if (latencies.length > 0) {
    totals.releaseLatencyMs =
      Math.round((latencies.reduce((sum, value) => sum + value, 0) / latencies.length) * 100) / 100
  }
  return totals
}

/**
 * Collapses each phase across the measured runs to its median. Phases keep
 * the order they were first seen in, so a report reads in gesture order
 * rather than alphabetically.
 */
export function medianPhases(
  runs: readonly CanvasBenchRunResult[],
): CanvasBenchPhaseResult[] {
  const byPhase = new Map<PanZoomPerfPhase['id'], CanvasBenchPhaseResult[]>()
  for (const run of runs) {
    for (const phase of run.phases) {
      const bucket = byPhase.get(phase.phase)
      if (bucket) bucket.push(phase)
      else byPhase.set(phase.phase, [phase])
    }
  }
  const mean = (values: number[]): number =>
    values.reduce((sum, value) => sum + value, 0) / values.length
  return [...byPhase.entries()].map(([id, results]) => ({
    phase: id,
    durationMs: Math.round(mean(results.map((r) => r.durationMs))),
    steps: results[0].steps,
    framesReceived: Math.round(mean(results.map((r) => r.framesReceived))),
    frames: roundSummary(medianSummary(results.map((r) => r.frames))),
    paintCost: roundPaintCost(medianPaintCost(results.map((r) => r.paintCost))),
  }))
}
