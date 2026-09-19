/**
 * Texture-resolution LOD tiers for offscreen pages (page-texture-scale.ts): a
 * page's view shrinks with its on-screen size, never below it, and shrinking
 * is sticky where growing is not.
 *
 * Mutation-verified by:
 * - `scale >= displayScale` → `scale >= displayScale / 2` in
 *   `smallestSharpScale` — the never-thinner-than-the-screen case fails;
 * - dropping `SHRINK_MARGIN` (`smallestSharpScale(displayScale)`) — the
 *   sticky-shrink case fails;
 * - applying the margin to growth too (`sharp >= currentScale` →
 *   `sharp === currentScale`) — the grows-at-once case fails.
 */

import { describe, expect, it } from 'vitest'
import {
  FULL_TEXTURE_SCALE,
  textureScaleForDisplayScale,
} from '../../src/main/runtime/page-texture-scale'

describe('textureScaleForDisplayScale', () => {
  it('grades texture scale by on-screen scale', () => {
    expect(textureScaleForDisplayScale(1, FULL_TEXTURE_SCALE)).toBe(1)
    expect(textureScaleForDisplayScale(0.3, FULL_TEXTURE_SCALE)).toBe(0.5)
    expect(textureScaleForDisplayScale(0.1, FULL_TEXTURE_SCALE)).toBe(0.25)
  })

  it('never paints thinner than the screen shows', () => {
    for (const current of [1, 0.5, 0.25]) {
      for (let display = 0.02; display <= 2; display += 0.02) {
        expect(textureScaleForDisplayScale(display, current)).toBeGreaterThanOrEqual(
          Math.min(display, FULL_TEXTURE_SCALE),
        )
      }
    }
  })

  it('grows at once when the page outgrows its texture', () => {
    expect(textureScaleForDisplayScale(0.26, 0.25)).toBe(0.5)
    expect(textureScaleForDisplayScale(0.51, 0.5)).toBe(1)
    expect(textureScaleForDisplayScale(0.9, 0.25)).toBe(1)
  })

  it('holds the current scale just under a boundary — shrinking is sticky', () => {
    expect(textureScaleForDisplayScale(0.45, 1)).toBe(1)
    expect(textureScaleForDisplayScale(0.22, 0.5)).toBe(0.5)
  })

  it('shrinks once the scale clears the margin, by as many tiers as it cleared', () => {
    expect(textureScaleForDisplayScale(0.39, 1)).toBe(0.5)
    expect(textureScaleForDisplayScale(0.19, 0.5)).toBe(0.25)
    expect(textureScaleForDisplayScale(0.1, 1)).toBe(0.25)
    // Cleared 0.5 but sits inside the 0.25 margin: owes the tier in between.
    expect(textureScaleForDisplayScale(0.22, 1)).toBe(0.5)
  })

  it('is stable: feeding the result back changes nothing', () => {
    for (const display of [0.05, 0.22, 0.26, 0.45, 0.52, 1]) {
      const once = textureScaleForDisplayScale(display, FULL_TEXTURE_SCALE)
      expect(textureScaleForDisplayScale(display, once)).toBe(once)
    }
  })
})
