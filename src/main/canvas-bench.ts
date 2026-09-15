import { ipcMain } from 'electron'
import path from 'path'
import {
  BENCH_DEFAULT_RUNS,
  BENCH_DEFAULT_WARMUP_RUNS,
  BENCH_NOMINAL_FRAME_MS,
  type CanvasBenchDriveRequest,
  type CanvasBenchDriveResult,
  diffPageHostStats,
  medianPhases,
  type CanvasBenchRequest,
  type CanvasBenchResult,
} from '../shared/canvas-bench'
import { ipcChannels } from '../shared/ipc-contract'
import { PAN_ZOOM_PERF_PHASES } from '../shared/pan-zoom-perf-test'
import { isPerfTraceRecording, startPerfTrace, stopPerfTrace } from './perf-trace'
import { isFocusSessionActive } from './runtime/focus-session'
import { pageHostStats } from './runtime/page-host'
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
