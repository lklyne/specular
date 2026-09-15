import { ipcMain } from 'electron'
import path from 'path'
import {
  BENCH_DEFAULT_RUNS,
  BENCH_DEFAULT_WARMUP_RUNS,
  BENCH_NOMINAL_FRAME_MS,
  type CanvasBenchDriveRequest,
  type CanvasBenchDriveResult,
  type CanvasBenchPageHostTotals,
  type CanvasBenchPhaseResult,
  type CanvasBenchRequest,
  type CanvasBenchResult,
  type CanvasBenchRunResult,
} from '../shared/canvas-bench'
import {
  medianPaintCost,
  medianSummary,
  roundPaintCost,
  roundSummary,
} from '../shared/frame-stats'
import { ipcChannels } from '../shared/ipc-contract'
import {
  PAN_ZOOM_PERF_PHASES,
  type PanZoomPerfPhase,
} from '../shared/pan-zoom-perf-test'
import { isPerfTraceRecording, startPerfTrace, stopPerfTrace } from './perf-trace'
import { isFocusSessionActive } from './runtime/focus-session'
import { pageHostStats, type PageHostStats } from './runtime/page-host'
import { pages, pan, zoom } from './runtime/runtime-context'
import { bgView, win } from './runtime/view-refs'
import { requestLayout, setPan, setZoom } from './runtime/viewport-control'

/**
 * How long the renderer gets to finish every run before the benchmark gives
 * up. Generous because a 40-page run at six phases is minutes of real
 * gesturing; the timeout exists so a renderer that died mid-run fails the
 * call instead of hanging it.
 */
const DRIVE_TIMEOUT_MS = 15 * 60_000

/** Camera settle after the run, before the trace stops. */
const RESTORE_SETTLE_MS = 300

let activeRun: Promise<CanvasBenchResult> | null = null

export function isCanvasBenchRunning(): boolean {
  return activeRun !== null
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Counters over the run. Cumulative fields are differenced against their
 * pre-run values; `maxOutstandingTextures` is a high-water mark, so it is
 * taken as the max rather than a delta, and a host created mid-run simply
 * contributes its whole count.
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
      Math.round((latencies.reduce((sum, v) => sum + v, 0) / latencies.length) * 100) / 100
  }
  return totals
}

/** Collapses one phase across the measured runs to its median. */
export function medianPhases(runs: readonly CanvasBenchRunResult[]): CanvasBenchPhaseResult[] {
  const byPhase = new Map<PanZoomPerfPhase['id'], CanvasBenchPhaseResult[]>()
  for (const run of runs) {
    for (const phase of run.phases) {
      const bucket = byPhase.get(phase.phase)
      if (bucket) bucket.push(phase)
      else byPhase.set(phase.phase, [phase])
    }
  }
  return [...byPhase.entries()].map(([id, results]) => ({
    phase: id,
    durationMs: Math.round(results.reduce((sum, r) => sum + r.durationMs, 0) / results.length),
    steps: results[0].steps,
    framesReceived: Math.round(
      results.reduce((sum, r) => sum + r.framesReceived, 0) / results.length,
    ),
    frames: roundSummary(medianSummary(results.map((r) => r.frames))),
    paintCost: roundPaintCost(medianPaintCost(results.map((r) => r.paintCost))),
  }))
}

/** Asks canvas-bg to drive the gesture and waits for its one reply. */
function driveRenderer(request: CanvasBenchDriveRequest): Promise<CanvasBenchDriveResult> {
  const contents = bgView?.webContents
  if (!contents || contents.isDestroyed()) {
    return Promise.reject(new Error('The canvas renderer is not ready'))
  }
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      ipcMain.removeListener(ipcChannels.canvasBenchResult, onResult)
      reject(new Error('The canvas renderer did not finish the benchmark in time'))
    }, DRIVE_TIMEOUT_MS)

    function onResult(_event: unknown, result: CanvasBenchDriveResult): void {
      clearTimeout(timer)
      ipcMain.removeListener(ipcChannels.canvasBenchResult, onResult)
      resolve(result)
    }

    ipcMain.on(ipcChannels.canvasBenchResult, onResult)
    contents.send(ipcChannels.canvasBenchRun, request)
  })
}

async function executeCanvasBench(request: CanvasBenchRequest): Promise<CanvasBenchResult> {
  if (!win) throw new Error('Main window is not ready')
  if (isPerfTraceRecording()) throw new Error('A performance trace is already active')
  if (isFocusSessionActive()) {
    throw new Error('Exit frame focus before running the canvas benchmark')
  }

  const phaseIds =
    request.phaseIds && request.phaseIds.length > 0
      ? PAN_ZOOM_PERF_PHASES.filter((phase) => request.phaseIds?.includes(phase.id)).map((p) => p.id)
      : PAN_ZOOM_PERF_PHASES.map((phase) => phase.id)
  if (phaseIds.length === 0) throw new Error('No known phase ids were requested')

  const runs = Math.max(1, request.runs ?? BENCH_DEFAULT_RUNS)
  const warmupRuns = Math.max(0, request.warmupRuns ?? BENCH_DEFAULT_WARMUP_RUNS)
  const contentBounds = win.getContentBounds()
  const initialPan = { ...pan }
  const initialZoom = zoom
  const statsBefore = pageHostStats()

  let tracePath: string | null = null
  let traceStarted = false
  if (request.trace) {
    await startPerfTrace({ revealOnAutoStop: false, owner: 'canvas-bench' })
    traceStarted = true
  }

  let drive: CanvasBenchDriveResult
  try {
    drive = await driveRenderer({
      phaseIds,
      runs: warmupRuns + runs,
      anchor: {
        mouseX: contentBounds.x + contentBounds.width / 2,
        mouseY: contentBounds.y + contentBounds.height / 2,
      },
    })
  } finally {
    setZoom(initialZoom)
    setPan(initialPan.x, initialPan.y)
    requestLayout()
    await wait(RESTORE_SETTLE_MS)
    if (traceStarted && isPerfTraceRecording()) {
      tracePath = await stopPerfTrace({ reveal: false, owner: 'canvas-bench' })
    }
  }

  const measured = drive.runs.slice(warmupRuns)
  if (measured.length === 0) {
    throw new Error('The canvas renderer reported no measured runs')
  }

  return {
    config: { phaseIds, runs: measured.length, warmupRuns, nominalFrameMs: BENCH_NOMINAL_FRAME_MS },
    pageCount: pages.length,
    refreshMs: Math.round(drive.refreshMs * 100) / 100,
    phases: medianPhases(measured),
    runs: measured,
    pageHosts: diffPageHostStats(statsBefore, pageHostStats()),
    tracePath: tracePath ? path.basename(tracePath) : null,
  }
}

export function runCanvasBench(request: CanvasBenchRequest = {}): Promise<CanvasBenchResult> {
  if (activeRun) return activeRun
  activeRun = executeCanvasBench(request).finally(() => {
    activeRun = null
  })
  return activeRun
}
