/**
 * Frame-rate LOD tiers for offscreen pages (page-frame-rate.ts): a page earns
 * frame rate by its on-screen size, and tier boundaries are sticky so a zoom
 * hovering near one doesn't flap the compositor cadence.
 *
 * Mutation-verified by:
 * - returning `target` unconditionally in `frameRateForDisplayScale`
 *   (dropping hysteresis) — the sticky-boundary cases fail;
 * - swapping the nudge direction (`1 - HYSTERESIS` ↔ `1 + HYSTERESIS`) —
 *   the switch-past-the-margin cases fail;
 * - moving the full-rate boundary back to 0.5 — the 50%-zoom case fails
 *   (zooming in to exactly 50% left pages at 30fps);
 * - returning `currentFps` unless the nudged tier equals the target — the
 *   in-between case fails (a zoom jump to 0.24 left pages at 60fps).
 */

import { describe, expect, it } from 'vitest'
import { FULL_FRAME_RATE, frameRateForDisplayScale } from '../../src/main/runtime/page-frame-rate'

describe('frameRateForDisplayScale', () => {
  it('grades rate by on-screen scale', () => {
    expect(frameRateForDisplayScale(1, FULL_FRAME_RATE)).toBe(60)
    expect(frameRateForDisplayScale(0.35, FULL_FRAME_RATE)).toBe(30)
    expect(frameRateForDisplayScale(0.1, FULL_FRAME_RATE)).toBe(15)
  })

  it('crosses multiple tiers in one step', () => {
    expect(frameRateForDisplayScale(0.05, FULL_FRAME_RATE)).toBe(15)
    expect(frameRateForDisplayScale(1, 15)).toBe(60)
  })

  it('takes the tier in between when a jump lands inside the far boundary margin', () => {
    // 60 → 0.24 crosses 0.5 cleanly but sits inside the 0.25 margin.
    expect(frameRateForDisplayScale(0.24, 60)).toBe(30)
    // 15 → 0.46 crosses 0.25 cleanly but sits inside the 0.44 margin.
    expect(frameRateForDisplayScale(0.46, 15)).toBe(30)
  })

  it('holds the current tier just past a boundary — boundaries are sticky', () => {
    // Just below 0.44 while at full rate: stays at 60.
    expect(frameRateForDisplayScale(0.42, 60)).toBe(60)
    // Just above 0.44 while at 30: stays at 30.
    expect(frameRateForDisplayScale(0.46, 30)).toBe(30)
  })

  it('switches once the scale clears the hysteresis margin', () => {
    expect(frameRateForDisplayScale(0.39, 60)).toBe(30)
    expect(frameRateForDisplayScale(0.49, 30)).toBe(60)
  })

  it('is smooth at 50% zoom from either direction', () => {
    expect(frameRateForDisplayScale(0.5, 60)).toBe(60)
    expect(frameRateForDisplayScale(0.5, 30)).toBe(60)
    expect(frameRateForDisplayScale(0.5, 15)).toBe(60)
  })

  it('is stable: feeding the result back changes nothing', () => {
    for (const scale of [0.05, 0.26, 0.42, 0.46, 0.5, 1]) {
      const once = frameRateForDisplayScale(scale, FULL_FRAME_RATE)
      expect(frameRateForDisplayScale(scale, once)).toBe(once)
    }
  })
})
