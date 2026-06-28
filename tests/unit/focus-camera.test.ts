import { describe, expect, it } from 'vitest'
import {
  computeFocusZoomForBounds,
  computePanToCenterBoundsAtZoom,
} from '../../src/shared/focus-camera'

describe('focus camera', () => {
  it('fits large selections into the viewport with padding', () => {
    expect(
      computeFocusZoomForBounds(
        { x: 0, y: 0, width: 2000, height: 1000 },
        { width: 1000, height: 800 },
      ),
    ).toBeCloseTo(0.436)
  })

  it('caps focus zoom at 100%', () => {
    expect(
      computeFocusZoomForBounds(
        { x: 0, y: 0, width: 100, height: 100 },
        { width: 1000, height: 800 },
      ),
    ).toBe(1)
  })

  it('centers bounds inside the currently available chrome-inset viewport', () => {
    const bounds = { x: 100, y: 50, width: 800, height: 600 }
    const pan = computePanToCenterBoundsAtZoom({
      bounds,
      viewport: { x: 320, y: 44, width: 880, height: 756 },
      canvasOriginX: 0,
      zoom: 1,
    })

    const screenCenterX = bounds.x + bounds.width / 2 + pan.x
    const screenCenterY = bounds.y + bounds.height / 2 + pan.y
    expect(screenCenterX).toBe(320 + 880 / 2)
    expect(screenCenterY).toBe(756 / 2)
  })
})
