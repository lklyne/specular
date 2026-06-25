import { describe, expect, it } from 'vitest'
import { computeFocusZoomForBounds } from '../../src/shared/focus-camera'

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
})
