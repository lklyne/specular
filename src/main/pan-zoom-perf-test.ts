import path from 'path'
import { screen } from 'electron'
import { ipcChannels } from '../shared/ipc-contract'
import {
  buildPanZoomPerfSteps,
  PAN_ZOOM_PERF_FRAME_MS,
  PAN_ZOOM_PERF_PHASE_GAP_MS,
  PAN_ZOOM_PERF_PHASES,
  type PanZoomPerfPhase,
  type PanZoomPerfTestResult,
  type PanZoomPerfTestState,
} from '../shared/pan-zoom-perf-test'
import { getDebugWebContents } from './debug-window'
import {
  isPerfTraceRecording,
  startPerfTrace,
  stopPerfTrace,
} from './perf-trace'
import { pan, zoom } from './runtime/runtime-context'
import { isFocusSessionActive } from './runtime/focus-session'
import { applyViewportInputDelta } from './runtime/viewport-input'
import { requestLayout, setPan, setZoom } from './runtime/viewport-control'
import { setBuildMsSink } from './runtime/layout-engine'
import { win } from './runtime/view-refs'
import { activeSpaceTabId } from './runtime/space-model'

export interface RunPanZoomPerfTestOptions {
  phaseIds?: PanZoomPerfPhase['id'][]
  /** Overrides each selected phase's duration, which keeps the trace under
   * the summarizable size cap in tight measurement loops. */
  durationMs?: number
}

const WARMUP_MS = 300
const RESTORE_SETTLE_MS = 300

let state: PanZoomPerfTestState = {
  running: false,
  stopping: false,
  phase: null,
  startedAt: null,
}
let abortController: AbortController | null = null
let activeRun: Promise<PanZoomPerfTestResult> | null = null

export function getPanZoomPerfTestState(): PanZoomPerfTestState {
  return { ...state }
}

function setState(next: PanZoomPerfTestState): void {
  state = next
  getDebugWebContents()?.send(ipcChannels.debugPerfPanZoomStateChanged, getPanZoomPerfTestState())
}

function setPhase(phase: string): void {
  setState({ ...state, phase })
}

function wait(ms: number, signal: AbortSignal): Promise<void> {
  if (signal.aborted) return Promise.resolve()
  return new Promise((resolve) => {
    const timer = setTimeout(done, ms)
    signal.addEventListener('abort', done, { once: true })

    function done(): void {
      clearTimeout(timer)
      signal.removeEventListener('abort', done)
      resolve()
    }
  })
}

interface PerfTestContext {
  initialPan: { x: number; y: number }
  initialZoom: number
  initialTabId: string | null
  anchor: { mouseX: number; mouseY: number }
}

function createPerfTestContext(): PerfTestContext {
  if (!win) throw new Error('Main window is not ready')
  if (isPerfTraceRecording()) throw new Error('A performance trace is already active')
  if (isFocusSessionActive()) {
    throw new Error('Exit frame focus before running the pan/zoom performance test')
  }
  const contentBounds = win.getContentBounds()
  return {
    initialPan: { ...pan },
    initialZoom: zoom,
    initialTabId: activeSpaceTabId,
    anchor: {
      mouseX: contentBounds.x + contentBounds.width / 2,
      mouseY: contentBounds.y + contentBounds.height / 2,
    },
  }
}

/** Drives input at the display's own refresh interval, so the test paces the
 *  same way the renderer's rAF does. Falls back to `PAN_ZOOM_PERF_FRAME_MS`
 *  when the display reports no frequency. */
function resolvePrimaryDisplayFrameMs(): number {
  const hz = screen.getPrimaryDisplay().displayFrequency
  return hz ? 1000 / hz : PAN_ZOOM_PERF_FRAME_MS
}

function computeBuildStats(samples: number[]): { n: number; mean: number; p95: number; max: number } {
  const n = samples.length
  if (n === 0) return { n: 0, mean: 0, p95: 0, max: 0 }
  const sorted = [...samples].sort((a, b) => a - b)
  const mean = samples.reduce((sum, v) => sum + v, 0) / n
  const p95 = sorted[Math.ceil(0.95 * n) - 1]
  const max = sorted[n - 1]
  const round = (v: number): number => Math.round(v * 100) / 100
  return { n, mean: round(mean), p95: round(p95), max: round(max) }
}

async function runGesturePhases(
  signal: AbortSignal,
  context: PerfTestContext,
  phases: readonly PanZoomPerfPhase[],
  frameMs: number,
): Promise<void> {
  for (const phase of phases) {
    if (signal.aborted) return
    setPhase(phase.label)
    for (const step of buildPanZoomPerfSteps(phase, frameMs)) {
      if (activeSpaceTabId !== context.initialTabId) abortController?.abort()
      if (signal.aborted) return
      applyViewportInputDelta({
        panDeltaX: step.panX,
        panDeltaY: step.panY,
        zoomDeltaY: step.zoomDeltaY,
        ...context.anchor,
      })
      await wait(frameMs, signal)
    }
    await wait(PAN_ZOOM_PERF_PHASE_GAP_MS, signal)
  }
}

async function restoreCamera(context: PerfTestContext): Promise<void> {
  if (activeSpaceTabId !== context.initialTabId) return
  setPhase('Restoring camera')
  setZoom(context.initialZoom)
  setPan(context.initialPan.x, context.initialPan.y)
  requestLayout()
  await wait(RESTORE_SETTLE_MS, new AbortController().signal)
}

async function saveTrace(traceStarted: boolean): Promise<string | null> {
  if (!traceStarted || !isPerfTraceRecording()) return null
  setPhase('Saving trace')
  return stopPerfTrace({ reveal: false, owner: 'pan-zoom-test' })
}

async function executePanZoomPerfTest(
  signal: AbortSignal,
  phases: readonly PanZoomPerfPhase[],
): Promise<PanZoomPerfTestResult> {
  const context = createPerfTestContext()
  const frameMs = resolvePrimaryDisplayFrameMs()
  let traceStarted = false
  let tracePath: string | null = null
  const buildSamples: number[] = []
  setBuildMsSink((ms) => buildSamples.push(ms))

  try {
    setPhase('Starting trace')
    await startPerfTrace({ revealOnAutoStop: false, owner: 'pan-zoom-test' })
    traceStarted = true
    await wait(WARMUP_MS, signal)
    await runGesturePhases(signal, context, phases, frameMs)
  } finally {
    setBuildMsSink(null)
    await restoreCamera(context)
    tracePath = await saveTrace(traceStarted)
  }

  if (!tracePath) throw new Error('The performance trace did not produce a file')
  return {
    cancelled: signal.aborted,
    tracePath,
    fileName: path.basename(tracePath),
    buildStats: computeBuildStats(buildSamples),
    frameMs,
  }
}

export function runPanZoomPerfTest(
  options?: RunPanZoomPerfTestOptions,
): Promise<PanZoomPerfTestResult> {
  if (activeRun) return activeRun
  const phaseIds = options?.phaseIds
  const selected =
    phaseIds && phaseIds.length > 0
      ? PAN_ZOOM_PERF_PHASES.filter((phase) => phaseIds.includes(phase.id))
      : PAN_ZOOM_PERF_PHASES
  const durationMs = options?.durationMs
  const phases =
    durationMs && durationMs > 0
      ? selected.map((phase) => ({ ...phase, durationMs }))
      : selected
  abortController = new AbortController()
  setState({ running: true, stopping: false, phase: 'Preparing', startedAt: Date.now() })
  const signal = abortController.signal
  activeRun = executePanZoomPerfTest(signal, phases).finally(() => {
    abortController = null
    activeRun = null
    setState({ running: false, stopping: false, phase: null, startedAt: null })
  })
  return activeRun
}

export async function stopPanZoomPerfTest(): Promise<void> {
  if (!activeRun) return
  setState({ ...state, stopping: true, phase: 'Stopping' })
  abortController?.abort()
  await activeRun
}
