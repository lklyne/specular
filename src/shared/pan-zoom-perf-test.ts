export interface PanZoomPerfPhase {
  id:
    | 'slow-pan'
    | 'slow-zoom'
    | 'fast-diagonal-pan'
    | 'slow-pan-zoom'
    | 'fast-pan-zoom'
    | 'zoom-out-then-pan'
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
  /** The per-step interval the run drove input at, in ms — the primary
   *  display's refresh interval, or PAN_ZOOM_PERF_FRAME_MS as a fallback. */
  frameMs: number
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
  // Reproduces the settle/pan collision: a quick zoom-out, then a fast pan
  // that is still running when the snapshot settle (300ms after the last
  // zoom tick), handoff, reveal, and encode all land.
  {
    id: 'zoom-out-then-pan',
    label: 'Zoom out, then fast pan',
    durationMs: 2_400,
    panX: -900,
    panY: 260,
    zoomDeltaY: 320,
  },
] as const

/** Share of the zoom-out-then-pan phase spent zooming before the pan starts. */
const ZOOM_LEAD_FRACTION = 1 / 6

export function buildPanZoomPerfSteps(
  phase: PanZoomPerfPhase,
  frameMs = PAN_ZOOM_PERF_FRAME_MS,
): PanZoomPerfStep[] {
  const stepCount = Math.max(1, Math.ceil(phase.durationMs / frameMs))
  if (phase.id === 'zoom-out-then-pan') {
    const zoomSteps = Math.max(1, Math.round(stepCount * ZOOM_LEAD_FRACTION))
    const panSteps = Math.max(1, stepCount - zoomSteps)
    return [
      ...Array.from({ length: zoomSteps }, () => ({
        panX: 0,
        panY: 0,
        zoomDeltaY: phase.zoomDeltaY / zoomSteps,
      })),
      ...Array.from({ length: panSteps }, () => ({
        panX: phase.panX / panSteps,
        panY: phase.panY / panSteps,
        zoomDeltaY: 0,
      })),
    ]
  }
  return Array.from({ length: stepCount }, () => ({
    panX: phase.panX / stepCount,
    panY: phase.panY / stepCount,
    zoomDeltaY: phase.zoomDeltaY / stepCount,
  }))
}
