import type {
  LabCamera,
  OsrLabBenchmarkPhaseId,
  OsrLabBenchmarkPhaseResult,
} from '../../shared/osr-lab'
import { panCamera, summarizeFrameIntervals, zoomCameraAt } from '../../shared/osr-lab'

interface LabBenchmarkPhase {
  id: OsrLabBenchmarkPhaseId
  durationMs: number
  panX: number
  panY: number
  zoomDeltaY: number
}

/** Mirrors the app's pan/zoom perf profiles (`shared/pan-zoom-perf-test.ts`). */
export const LAB_BENCHMARK_PHASES: readonly LabBenchmarkPhase[] = [
  { id: 'slow-pan', durationMs: 2000, panX: 360, panY: 0, zoomDeltaY: 0 },
  { id: 'slow-zoom', durationMs: 2000, panX: 0, panY: 0, zoomDeltaY: -140 },
  { id: 'fast-pan', durationMs: 450, panX: -360, panY: -280, zoomDeltaY: 0 },
  { id: 'pan-zoom', durationMs: 2000, panX: 300, panY: 180, zoomDeltaY: 100 },
]

const PHASE_GAP_MS = 250

export interface LabBenchmarkHost {
  getCamera: () => LabCamera
  setCamera: (camera: LabCamera) => void
  viewportCenter: () => { x: number; y: number }
  framesReceived: () => number
  refreshMs: number
}

function nextFrame(): Promise<number> {
  return new Promise((resolve) => requestAnimationFrame(resolve))
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Drives the camera through each phase one rAF at a time, recording the rAF
 * cadence while it does. Restores the starting camera afterwards.
 */
export async function runLabBenchmark(host: LabBenchmarkHost): Promise<OsrLabBenchmarkPhaseResult[]> {
  const initial = host.getCamera()
  const results: OsrLabBenchmarkPhaseResult[] = []
  for (const phase of LAB_BENCHMARK_PHASES) {
    const steps = Math.max(1, Math.round(phase.durationMs / host.refreshMs))
    const stepPanX = phase.panX / steps
    const stepPanY = phase.panY / steps
    const stepZoom = phase.zoomDeltaY / steps
    const center = host.viewportCenter()
    const intervals: number[] = []
    const framesBefore = host.framesReceived()
    let last = await nextFrame()
    const started = last
    for (let i = 0; i < steps; i++) {
      let camera = host.getCamera()
      if (stepZoom !== 0) camera = zoomCameraAt(camera, stepZoom, center.x, center.y)
      if (stepPanX !== 0 || stepPanY !== 0) camera = panCamera(camera, stepPanX, stepPanY)
      host.setCamera(camera)
      const now = await nextFrame()
      intervals.push(now - last)
      last = now
    }
    results.push({
      phase: phase.id,
      durationMs: last - started,
      framesReceived: host.framesReceived() - framesBefore,
      ...summarizeFrameIntervals(intervals, host.refreshMs),
    })
    await wait(PHASE_GAP_MS)
  }
  host.setCamera(initial)
  return results
}

/** Estimates the display refresh interval from a short run of rAF callbacks. */
export async function measureRefreshMs(samples = 20): Promise<number> {
  const intervals: number[] = []
  let last = await nextFrame()
  for (let i = 0; i < samples; i++) {
    const now = await nextFrame()
    intervals.push(now - last)
    last = now
  }
  intervals.sort((a, b) => a - b)
  return intervals[Math.floor(intervals.length / 2)] || 16.67
}
