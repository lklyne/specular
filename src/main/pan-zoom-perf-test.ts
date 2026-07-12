import path from 'path'
import { ipcChannels } from '../shared/ipc-contract'
import {
  buildPanZoomPerfSteps,
  PAN_ZOOM_PERF_FRAME_MS,
  PAN_ZOOM_PERF_PHASE_GAP_MS,
  PAN_ZOOM_PERF_PHASES,
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
import { applyViewportInputDelta } from './runtime/viewport-input'
import { requestLayout, setPan, setZoom } from './runtime/viewport-control'
import { win } from './runtime/view-refs'

const WARMUP_MS = 300
const RESTORE_SETTLE_MS = 300

let state: PanZoomPerfTestState = {
  running: false,
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

async function executePanZoomPerfTest(signal: AbortSignal): Promise<PanZoomPerfTestResult> {
  if (!win) throw new Error('Main window is not ready')
  if (isPerfTraceRecording()) throw new Error('A performance trace is already active')

  const initialPan = { ...pan }
  const initialZoom = zoom
  const contentBounds = win.getContentBounds()
  const anchor = {
    mouseX: contentBounds.x + contentBounds.width / 2,
    mouseY: contentBounds.y + contentBounds.height / 2,
  }
  let traceStarted = false
  let tracePath: string | null = null

  try {
    setPhase('Starting trace')
    await startPerfTrace({ revealOnAutoStop: false })
    traceStarted = true
    await wait(WARMUP_MS, signal)

    for (const phase of PAN_ZOOM_PERF_PHASES) {
      if (signal.aborted) break
      setPhase(phase.label)
      for (const step of buildPanZoomPerfSteps(phase)) {
        if (signal.aborted) break
        applyViewportInputDelta({
          panDeltaX: step.panX,
          panDeltaY: step.panY,
          zoomDeltaY: step.zoomDeltaY,
          ...anchor,
        })
        await wait(PAN_ZOOM_PERF_FRAME_MS, signal)
      }
      if (!signal.aborted) await wait(PAN_ZOOM_PERF_PHASE_GAP_MS, signal)
    }
  } finally {
    setPhase('Restoring camera')
    setZoom(initialZoom)
    setPan(initialPan.x, initialPan.y)
    requestLayout()
    await wait(RESTORE_SETTLE_MS, new AbortController().signal)

    if (traceStarted && isPerfTraceRecording()) {
      setPhase('Saving trace')
      tracePath = await stopPerfTrace({ reveal: false })
    }
  }

  if (!tracePath) throw new Error('The performance trace did not produce a file')
  return {
    cancelled: signal.aborted,
    tracePath,
    fileName: path.basename(tracePath),
  }
}

export function runPanZoomPerfTest(): Promise<PanZoomPerfTestResult> {
  if (activeRun) return activeRun
  abortController = new AbortController()
  setState({ running: true, phase: 'Preparing', startedAt: Date.now() })
  const signal = abortController.signal
  activeRun = executePanZoomPerfTest(signal).finally(() => {
    abortController = null
    activeRun = null
    setState({ running: false, phase: null, startedAt: null })
  })
  return activeRun
}

export async function stopPanZoomPerfTest(): Promise<void> {
  if (!activeRun) return
  abortController?.abort()
  await activeRun
}
