/**
 * Unit tests for `arrangeInSpan` — the row / column / grid "tidy in place"
 * kernel behind the popup toolbar. All three keep the cluster's footprint and
 * regularize spacing inside it.
 *
 * Mutation-verified by:
 *   - returning the input positions unchanged: the even-gap and align
 *     assertions fail.
 *   - collapsing to a fixed gap from the origin (the old pack behavior): the
 *     "last item's trailing edge stays put" assertions fail.
 *   - not clustering grid bands (treating every box as its own column): the
 *     "column-aligned items share an x" assertion fails.
 */

import { describe, expect, it } from 'vitest'
import { arrangeInSpan } from '../../src/shared/span-arrange'
import type { Box } from '../../src/shared/reorder-row'

/** Merge a box with its arranged target for readback. */
function placed(
  targets: Map<string, { x: number; y: number }>,
  boxes: Box[],
  id: string,
): Box {
  const orig = boxes.find((b) => b.id === id)!
  return { ...orig, ...(targets.get(id) ?? {}) }
}

describe('arrangeInSpan — row', () => {
  it('evens x-gaps across the current extent, endpoints pinned, tops aligned', () => {
    // a at 0, b at 150, c at 400 — uneven gaps, ragged y.
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 10, width: 100, height: 80 },
      { id: 'b', x: 150, y: 40, width: 100, height: 80 },
      { id: 'c', x: 400, y: 5, width: 100, height: 80 },
    ]
    const t = arrangeInSpan(boxes, 'row')!
    const a = placed(t, boxes, 'a')
    const b = placed(t, boxes, 'b')
    const c = placed(t, boxes, 'c')

    // Footprint pinned: first leading edge and last trailing edge unchanged.
    expect(a.x).toBe(0)
    expect(c.x + c.width).toBe(500)
    // Gaps even.
    expect(b.x - (a.x + a.width)).toBeCloseTo(c.x - (b.x + b.width), 5)
    // Aligned to the top edge (min y = 5).
    expect(a.y).toBe(5)
    expect(b.y).toBe(5)
    expect(c.y).toBe(5)
  })
})

describe('arrangeInSpan — column', () => {
  it('evens y-gaps across the current extent, left edges aligned', () => {
    const boxes: Box[] = [
      { id: 'a', x: 10, y: 0, width: 80, height: 50 },
      { id: 'b', x: 40, y: 100, width: 80, height: 50 },
      { id: 'c', x: 5, y: 500, width: 80, height: 50 },
    ]
    const t = arrangeInSpan(boxes, 'column')!
    const a = placed(t, boxes, 'a')
    const b = placed(t, boxes, 'b')
    const c = placed(t, boxes, 'c')

    expect(a.y).toBe(0)
    expect(c.y + c.height).toBe(550)
    expect(b.y - (a.y + a.height)).toBeCloseTo(c.y - (b.y + b.height), 5)
    // Aligned to the left edge (min x = 5).
    expect(a.x).toBe(5)
    expect(b.x).toBe(5)
    expect(c.x).toBe(5)
  })
})

describe('arrangeInSpan — grid', () => {
  it('keeps 2-D structure: shared bands snap to one anchor, holes preserved', () => {
    // Two rows, two columns, with the bottom-right cell empty (a hole).
    //   a  b
    //   c
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 100 },
      { id: 'b', x: 130, y: 8, width: 100, height: 100 },
      { id: 'c', x: 12, y: 220, width: 100, height: 100 },
    ]
    const t = arrangeInSpan(boxes, 'grid')!
    const a = placed(t, boxes, 'a')
    const b = placed(t, boxes, 'b')
    const c = placed(t, boxes, 'c')

    // a and c share a column → same x. a and b share a row → same y.
    expect(a.x).toBe(c.x)
    expect(a.y).toBe(b.y)
    // Only two columns exist; b keeps its own column (no third item fills the hole).
    expect(b.x).not.toBe(a.x)
    // Footprint pinned on both axes.
    expect(Math.min(a.x, b.x, c.x)).toBe(0)
    expect(Math.min(a.y, b.y, c.y)).toBe(0)
  })

  it('degenerate single row: distributes x, leaves y untouched', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 80 },
      { id: 'b', x: 150, y: 0, width: 100, height: 80 },
      { id: 'c', x: 400, y: 0, width: 100, height: 80 },
    ]
    const t = arrangeInSpan(boxes, 'grid')!
    const b = placed(t, boxes, 'b')
    // Even gaps → b's leading edge moves to the midpoint slot (200).
    expect(b.x).toBe(200)
    expect(b.y).toBe(0)
  })
})

it('arrangeInSpan returns null below 2 boxes', () => {
  expect(arrangeInSpan([], 'row')).toBeNull()
  expect(arrangeInSpan([{ id: 'a', x: 0, y: 0, width: 10, height: 10 }], 'grid')).toBeNull()
})
