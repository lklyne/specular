import { describe, expect, it } from 'vitest'
import {
  buildPanZoomPerfSteps,
  PAN_ZOOM_PERF_PHASES,
} from '../../src/shared/pan-zoom-perf-test'

describe('pan/zoom performance test plan', () => {
  it('covers the gesture profiles with frame-sized deltas that preserve each total', () => {
    expect(PAN_ZOOM_PERF_PHASES.map((phase) => phase.id)).toEqual([
      'slow-pan',
      'slow-zoom',
      'fast-diagonal-pan',
      'slow-pan-zoom',
      'fast-pan-zoom',
      'zoom-out-then-pan',
    ])

    for (const phase of PAN_ZOOM_PERF_PHASES) {
      const totals = buildPanZoomPerfSteps(phase).reduce(
        (sum, step) => ({
          panX: sum.panX + step.panX,
          panY: sum.panY + step.panY,
          zoomDeltaY: sum.zoomDeltaY + step.zoomDeltaY,
        }),
        { panX: 0, panY: 0, zoomDeltaY: 0 },
      )
      expect(totals.panX).toBeCloseTo(phase.panX)
      expect(totals.panY).toBeCloseTo(phase.panY)
      expect(totals.zoomDeltaY).toBeCloseTo(phase.zoomDeltaY)
    }
  })
})
