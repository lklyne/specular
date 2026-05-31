/**
 * Unit tests for `distributeRowPositions` (ADR 0015 D7, Phase A).
 *
 * Mutation-verified by:
 *   - returning null unconditionally: the "3 uneven items → equal gaps" and
 *     round-trip assertions fail immediately.
 *   - computing gap from the wrong extent (e.g. using only the first two
 *     boxes): the equal-gap assertion on the output fails.
 */

import { describe, expect, it } from 'vitest'
import { distributeRowPositions } from '../../src/shared/distribute-row'
import { detectReorderableRow, dominantAxis, type Box } from '../../src/shared/reorder-row'

/** Build a horizontal row of equal-width boxes. */
function hrow(count: number, width = 100, gap = 20, y = 0): Box[] {
  return Array.from({ length: count }, (_, i) => ({
    id: String.fromCharCode(97 + i),
    x: i * (width + gap),
    y,
    width,
    height: 80,
  }))
}

/** Edge-to-edge gap between two consecutive sorted boxes along axis x. */
function gapBetween(a: Box, b: Box): number {
  return b.x - (a.x + a.width)
}

describe('dominantAxis', () => {
  it('returns x for a horizontal spread', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 50, height: 50 },
      { id: 'b', x: 200, y: 0, width: 50, height: 50 },
    ]
    expect(dominantAxis(boxes)).toBe('x')
  })

  it('returns y for a vertical spread', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 50, height: 50 },
      { id: 'b', x: 0, y: 200, width: 50, height: 50 },
    ]
    expect(dominantAxis(boxes)).toBe('y')
  })
})

describe('distributeRowPositions', () => {
  it('evens gaps for 3 uneven items; endpoints fixed', () => {
    // A at 0, B at 150, C at 400 — gaps are 50 and 200.
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 80 },
      { id: 'b', x: 150, y: 0, width: 100, height: 80 },
      { id: 'c', x: 400, y: 0, width: 100, height: 80 },
    ]
    const result = distributeRowPositions(boxes)
    expect(result).not.toBeNull()
    expect(result!.axis).toBe('x')

    // Apply new positions (merge with originals for unchanged boxes).
    const placed = (id: string): Box => {
      const pos = result!.positions.get(id)
      const orig = boxes.find((b) => b.id === id)!
      return { ...orig, ...(pos ?? {}) }
    }
    const a = placed('a')
    const b = placed('b')
    const c = placed('c')

    // Endpoints fixed.
    expect(a.x).toBe(0)
    expect(c.x).toBe(400)

    // Gaps are equal.
    const gap1 = gapBetween(a, b)
    const gap2 = gapBetween(b, c)
    expect(Math.abs(gap1 - gap2)).toBeLessThanOrEqual(1)
  })

  it('handles mixed widths — gaps equal edge-to-edge, not center-to-center', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 80 },
      { id: 'b', x: 200, y: 0, width: 200, height: 80 },  // wide
      { id: 'c', x: 600, y: 0, width: 50, height: 80 },
    ]
    const result = distributeRowPositions(boxes)!
    const placed = (id: string): Box => {
      const pos = result.positions.get(id)
      const orig = boxes.find((b) => b.id === id)!
      return { ...orig, ...(pos ?? {}) }
    }
    const a = placed('a')
    const b = placed('b')
    const c = placed('c')

    const gap1 = gapBetween(a, b)
    const gap2 = gapBetween(b, c)
    expect(Math.abs(gap1 - gap2)).toBeLessThanOrEqual(1)
    // Endpoints fixed.
    expect(a.x).toBe(0)
    expect(c.x + c.width).toBeCloseTo(650, 0) // trailing edge of c stays
  })

  it('auto-detects vertical column as axis y', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 80, height: 50 },
      { id: 'b', x: 0, y: 100, width: 80, height: 50 },
      { id: 'c', x: 0, y: 500, width: 80, height: 50 },
    ]
    const result = distributeRowPositions(boxes)!
    expect(result.axis).toBe('y')

    // Gaps along y should now be equal.
    const sorted = [
      { ...boxes[0], ...(result.positions.get('a') ?? {}) },
      { ...boxes[1], ...(result.positions.get('b') ?? {}) },
      { ...boxes[2], ...(result.positions.get('c') ?? {}) },
    ]
    const yGap1 = sorted[1].y - (sorted[0].y + sorted[0].height)
    const yGap2 = sorted[2].y - (sorted[1].y + sorted[1].height)
    expect(Math.abs(yGap1 - yGap2)).toBeLessThanOrEqual(1)
  })

  it('preserves cross-axis coordinate', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 10, width: 100, height: 80 },
      { id: 'b', x: 200, y: 40, width: 100, height: 80 },
      { id: 'c', x: 600, y: 5, width: 100, height: 80 },
    ]
    const result = distributeRowPositions(boxes)!
    // b moved along x but its y must stay 40.
    const bPos = result.positions.get('b')
    expect(bPos).toBeDefined()
    expect(bPos!.y).toBe(40)
  })

  it('returns null for fewer than 3 boxes', () => {
    expect(distributeRowPositions([])).toBeNull()
    expect(distributeRowPositions([{ id: 'a', x: 0, y: 0, width: 100, height: 80 }])).toBeNull()
    expect(
      distributeRowPositions([
        { id: 'a', x: 0, y: 0, width: 100, height: 80 },
        { id: 'b', x: 200, y: 0, width: 100, height: 80 },
      ]),
    ).toBeNull()
  })

  it('returns null when already even within tolerance (no-op)', () => {
    // Equal-gap row — nothing to move.
    const boxes = hrow(4, 100, 20)
    expect(distributeRowPositions(boxes)).toBeNull()
  })

  it('respects explicit axis override', () => {
    // Horizontal spread but forced to y — should distribute along y.
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 80, height: 50 },
      { id: 'b', x: 200, y: 60, width: 80, height: 50 },
      { id: 'c', x: 400, y: 300, width: 80, height: 50 },
    ]
    const result = distributeRowPositions(boxes, { axis: 'y' })!
    expect(result.axis).toBe('y')
  })

  it('round-trip — output passes detectReorderableRow (through-line to reorder)', () => {
    // Unevenly-spaced selection — distribute output must pass the reorder gate.
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 80 },
      { id: 'b', x: 200, y: 0, width: 100, height: 80 }, // gap 100
      { id: 'c', x: 600, y: 0, width: 100, height: 80 }, // gap 300
    ]
    const result = distributeRowPositions(boxes)!
    // Apply positions.
    const distributed: Box[] = boxes.map((b) => {
      const pos = result.positions.get(b.id)
      return pos ? { ...b, ...pos } : b
    })
    expect(detectReorderableRow(distributed)).not.toBeNull()
  })

  it('allows negative gap (overlapping items) — distributes evenly', () => {
    // Items that overlap: gap is negative.
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 80 },
      { id: 'b', x: 50, y: 0, width: 100, height: 80 },  // overlaps a
      { id: 'c', x: 200, y: 0, width: 100, height: 80 }, // big positive gap
    ]
    const result = distributeRowPositions(boxes)
    // Should return a result (not null), even for negative gap scenario.
    expect(result).not.toBeNull()
  })
})
