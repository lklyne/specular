export interface PanZoomPerfPhase {
  id:
    | 'slow-pan'
    | 'slow-zoom'
    | 'fast-diagonal-pan'
    | 'slow-pan-zoom'
    | 'fast-pan-zoom'
  label: string
  durationMs: number
  panX: number
  panY: number
  zoomDeltaY: number
}

export interface PanZoomPerfStep {
  panX: number
  panY: number
  zoomDeltaY: number
}

export interface PanZoomPerfTestState {
  running: boolean
  stopping: boolean
  phase: string | null
  startedAt: number | null
}

export interface PanZoomPerfTestResult {
  cancelled: boolean
  tracePath: string
  fileName: string
  buildStats?: { n: number; mean: number; p95: number; max: number }
}

export const PAN_ZOOM_PERF_FRAME_MS = 16
export const PAN_ZOOM_PERF_PHASE_GAP_MS = 250

export const PAN_ZOOM_PERF_PHASES: readonly PanZoomPerfPhase[] = [
  {
    id: 'slow-pan',
    label: 'Slow pan',
    durationMs: 2_000,
    panX: 360,
    panY: 0,
    zoomDeltaY: 0,
  },
  {
    id: 'slow-zoom',
    label: 'Slow zoom',
    durationMs: 2_000,
    panX: 0,
    panY: 0,
    zoomDeltaY: -140,
  },
  {
    id: 'fast-diagonal-pan',
    label: 'Fast diagonal pan',
    durationMs: 450,
    panX: -360,
    panY: -280,
    zoomDeltaY: 0,
  },
  {
    id: 'slow-pan-zoom',
    label: 'Slow pan + zoom',
    durationMs: 2_000,
    panX: 300,
    panY: 180,
    zoomDeltaY: 100,
  },
  {
    id: 'fast-pan-zoom',
    label: 'Fast pan + zoom',
    durationMs: 450,
    panX: -340,
    panY: 220,
    zoomDeltaY: -120,
  },
] as const

export function buildPanZoomPerfSteps(
  phase: PanZoomPerfPhase,
  frameMs = PAN_ZOOM_PERF_FRAME_MS,
): PanZoomPerfStep[] {
  const stepCount = Math.max(1, Math.ceil(phase.durationMs / frameMs))
  return Array.from({ length: stepCount }, () => ({
    panX: phase.panX / stepCount,
    panY: phase.panY / stepCount,
    zoomDeltaY: phase.zoomDeltaY / stepCount,
  }))
}
