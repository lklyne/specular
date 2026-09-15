/**
 * The renderer half of the pan/zoom benchmark: drives a scripted gesture one
 * step per animation frame and records its own cadence while it does.
 *
 * Frame-locked rather than timer-paced. A `setTimeout`-driven gesture delivers
 * fewer steps when the app is slow, so it shrinks the workload exactly when a
 * regression should be enlarging it, and reports against a wall-clock
 * denominator that moved. Stepping on rAF fixes the step count: a slow build
 * takes longer to cover the same ground instead of covering less of it.
 *
 * Input goes through the same bridge calls a trackpad gesture makes, so the
 * measured path is the production one, and nothing waits on main between steps
 * — the recorded intervals are the renderer's own cadence, not IPC latency.
 */

import {
  BENCH_NOMINAL_FRAME_MS,
  BENCH_PHASE_GAP_MS,
  type CanvasBenchDriveRequest,
  type CanvasBenchDriveResult,
  type CanvasBenchPhaseResult,
  type CanvasBenchRunResult,
} from '../../shared/canvas-bench'
import { summarizeFrameIntervals, summarizePaintCosts } from '../../shared/frame-stats'
import {
  buildPanZoomPerfSteps,
  PAN_ZOOM_PERF_PHASES,
  type PanZoomPerfPhase,
} from '../../shared/pan-zoom-perf-test'
import {
  armPaintRecorder,
  disarmPaintRecorder,
  drainPaintCosts,
  framesReceivedSoFar,
} from './paintInstrumentation'

/** The bridge calls the benchmark drives the camera with. */
export interface CanvasBenchHost {
  pan: (deltaX: number, deltaY: number) => void
  zoom: (deltaY: number, mouseX: number, mouseY: number) => void
}

function nextFrame(): Promise<number> {
  return new Promise((resolve) => requestAnimationFrame(resolve))
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * The display's refresh interval, measured from rAF rather than read from
 * `screen.displayFrequency`. Recorded so long-frame thresholds scale to the
 * machine — never used to size the workload, which is what keeps a 60Hz and a
 * 120Hz run comparable.
 */
export async function measureRefreshMs(samples = 20): Promise<number> {
  const intervals: number[] = []
  let last = await nextFrame()
  for (let i = 0; i < samples; i++) {
    const now = await nextFrame()
    intervals.push(now - last)
    last = now
  }
  intervals.sort((a, b) => a - b)
  return intervals[intervals.length >> 1] || BENCH_NOMINAL_FRAME_MS
}

async function runPhase(
  host: CanvasBenchHost,
  phase: PanZoomPerfPhase,
  anchor: { mouseX: number; mouseY: number },
  refreshMs: number,
): Promise<CanvasBenchPhaseResult> {
  const steps = buildPanZoomPerfSteps(phase, BENCH_NOMINAL_FRAME_MS)
  const intervals: number[] = []
  // Anything painted during the inter-phase gap belongs to no phase.
  drainPaintCosts()
  const framesBefore = framesReceivedSoFar()
  let last = await nextFrame()
  const started = last
  for (const step of steps) {
    if (step.zoomDeltaY !== 0) host.zoom(step.zoomDeltaY, anchor.mouseX, anchor.mouseY)
    // The pan bridge negates on the way to the viewport, so a phase's own
    // pan delta has to go in inverted to cover the ground it describes.
    if (step.panX !== 0 || step.panY !== 0) host.pan(-step.panX, -step.panY)
    const now = await nextFrame()
    intervals.push(now - last)
    last = now
  }
  return {
    phase: phase.id,
    durationMs: last - started,
    steps: steps.length,
    framesReceived: framesReceivedSoFar() - framesBefore,
    frames: summarizeFrameIntervals(intervals, refreshMs),
    paintCost: summarizePaintCosts(drainPaintCosts()),
  }
}

/**
 * Drives every requested phase `request.runs` times. The camera is left where
 * the last phase put it — main owns the camera and restores it, so the
 * renderer never writes a viewport it does not own.
 */
export async function runCanvasBench(
  host: CanvasBenchHost,
  request: CanvasBenchDriveRequest,
): Promise<CanvasBenchDriveResult> {
  const refreshMs = await measureRefreshMs()
  const phases = PAN_ZOOM_PERF_PHASES.filter((phase) => request.phaseIds.includes(phase.id))
  const runs: CanvasBenchRunResult[] = []
  armPaintRecorder()
  try {
    for (let run = 0; run < request.runs; run++) {
      const results: CanvasBenchPhaseResult[] = []
      for (const phase of phases) {
        results.push(await runPhase(host, phase, request.anchor, refreshMs))
        await wait(BENCH_PHASE_GAP_MS)
      }
      runs.push({ refreshMs, phases: results })
    }
  } finally {
    disarmPaintRecorder()
  }
  return { refreshMs, runs }
}
