/**
 * Pacing of frame-triggered repaints on canvas-bg's item surface
 * (useFramePaintPacing.ts): one page's frame repaints every page, so an
 * arrival is held until its page's own interval has passed since the last
 * paint.
 *
 * Mutation-verified by:
 * - returning `lastPaintAt` from `framePaintDueAt` (every arrival due at
 *   once) — the held-for-its-interval cases fail (thirty unaligned 15fps
 *   pages kept the surface repainting at display rate);
 * - dropping `VSYNC_SLACK_MS` — the full-rate case fails (a 60fps page's
 *   paint slipped a vsync and showed at 30).
 */

import { describe, expect, it } from 'vitest'
import { framePaintDueAt } from '../../src/renderer/canvas-bg/useFramePaintPacing'

const VSYNC_MS = 1000 / 60

describe('framePaintDueAt', () => {
  it('holds a slow page frame for its interval, to the nearest vsync', () => {
    const lastPaintAt = 1_000
    // 15fps: due on the 4th vsync after the last paint, not the 1st.
    const due15 = framePaintDueAt(lastPaintAt, 15)
    expect(due15).toBeGreaterThan(lastPaintAt + 3 * VSYNC_MS)
    expect(due15).toBeLessThanOrEqual(lastPaintAt + 4 * VSYNC_MS)
    // 30fps: due on the 2nd.
    const due30 = framePaintDueAt(lastPaintAt, 30)
    expect(due30).toBeGreaterThan(lastPaintAt + VSYNC_MS)
    expect(due30).toBeLessThanOrEqual(lastPaintAt + 2 * VSYNC_MS)
  })

  it('never makes a full-rate frame miss the next vsync', () => {
    const lastPaintAt = 1_000
    // Due strictly before the next vsync, so the rAF it schedules lands on it.
    expect(framePaintDueAt(lastPaintAt, 60)).toBeLessThan(lastPaintAt + VSYNC_MS - 1)
  })

  it('is due at once for a frame that follows the hand, not a page rate', () => {
    expect(framePaintDueAt(1_000, Infinity)).toBeLessThanOrEqual(1_000)
  })
})
