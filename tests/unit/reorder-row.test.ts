import { describe, expect, it } from 'vitest'
import {
  detectReorderableRow,
  dropIndexForCursor,
  reorderRowPositions,
  type Box,
} from '../../src/shared/reorder-row'

/** Build an evenly-spaced horizontal row of equal-width boxes. */
function row(width: number, gap: number, count: number, y = 0): Box[] {
  return Array.from({ length: count }, (_, i) => ({
    id: String.fromCharCode(97 + i),
    x: i * (width + gap),
    y,
    width,
    height: 80,
  }))
}

describe('detectReorderableRow — eligibility', () => {
  it('accepts an equal-gap row', () => {
    const detected = detectReorderableRow(row(100, 20, 4))
    expect(detected).not.toBeNull()
    expect(detected!.axis).toBe('x')
    expect(detected!.order).toEqual(['a', 'b', 'c', 'd'])
    expect(detected!.gap).toBe(20)
    expect(detected!.origin).toEqual({ x: 0, y: 0 })
  })

  it('rejects a single unequal gap', () => {
    const boxes = row(100, 20, 3)
    boxes[2].x += 30 // widen only the last gap
    expect(detectReorderableRow(boxes)).toBeNull()
  })

  it('rejects overlapping boxes', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 80 },
      { id: 'b', x: 50, y: 0, width: 100, height: 80 }, // overlaps a
    ]
    expect(detectReorderableRow(boxes)).toBeNull()
  })

  it('returns null for fewer than two boxes', () => {
    expect(detectReorderableRow([])).toBeNull()
    expect(detectReorderableRow(row(100, 20, 1))).toBeNull()
  })

  it('accepts a two-item row (single gap is always equal)', () => {
    const detected = detectReorderableRow(row(100, 20, 2))
    expect(detected).not.toBeNull()
    expect(detected!.order).toEqual(['a', 'b'])
    expect(detected!.gap).toBe(20)
  })

  it('detects a vertical column as axis y', () => {
    const boxes: Box[] = Array.from({ length: 3 }, (_, i) => ({
      id: String.fromCharCode(97 + i),
      x: 0,
      y: i * 130, // height 80 + gap 50
      width: 100,
      height: 80,
    }))
    const detected = detectReorderableRow(boxes)
    expect(detected).not.toBeNull()
    expect(detected!.axis).toBe('y')
    expect(detected!.order).toEqual(['a', 'b', 'c'])
    expect(detected!.gap).toBe(50)
  })

  it('honors the gap-tolerance boundary', () => {
    const boxes = row(100, 20, 3)
    boxes[2].x += 1 // last gap is 21, spread = 1
    expect(detectReorderableRow(boxes, { gapTolerance: 1 })).not.toBeNull()
    expect(detectReorderableRow(boxes, { gapTolerance: 0.5 })).toBeNull()
  })
})

describe('reorderRowPositions — repack', () => {
  it('moves the correct edges for mixed widths', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 80 },
      { id: 'b', x: 110, y: 0, width: 200, height: 80 },
      { id: 'c', x: 320, y: 0, width: 50, height: 80 },
    ]
    const detected = detectReorderableRow(boxes)!
    expect(detected.gap).toBe(10)
    // Move c to the front → order c, a, b packed from origin.x = 0.
    const changed = reorderRowPositions(detected, 'c', 0)
    expect(changed.get('c')).toEqual({ x: 0, y: 0 }) // 50 wide
    expect(changed.get('a')).toEqual({ x: 60, y: 0 }) // after c (50) + gap (10)
    expect(changed.get('b')).toEqual({ x: 170, y: 0 }) // after a (100) + gap (10)
  })

  it('preserves each item cross-axis coordinate', () => {
    const boxes: Box[] = [
      { id: 'a', x: 0, y: 0, width: 100, height: 80 },
      { id: 'b', x: 120, y: 40, width: 100, height: 80 }, // sits lower
      { id: 'c', x: 240, y: 0, width: 100, height: 80 },
    ]
    const detected = detectReorderableRow(boxes)!
    // Move c before b → order a, c, b. b keeps its y=40.
    const changed = reorderRowPositions(detected, 'c', 1)
    expect(changed.get('c')!.y).toBe(0)
    expect(changed.get('b')).toEqual({ x: 240, y: 40 })
    expect(changed.has('a')).toBe(false) // unchanged, omitted
  })

  it('returns only changed positions (identity move is empty)', () => {
    const detected = detectReorderableRow(row(100, 20, 3))!
    expect(reorderRowPositions(detected, 'b', 1).size).toBe(0)
  })
})

describe('dropIndexForCursor', () => {
  const detected = detectReorderableRow(row(100, 20, 3))!
  // Centers (excluding the moving box): a=50, b=170, c=290.

  it('returns 0 when the cursor is before all other centers', () => {
    expect(dropIndexForCursor(detected, 0, 'a')).toBe(0)
  })

  it('returns the end index when the cursor is past all other centers', () => {
    expect(dropIndexForCursor(detected, 1000, 'a')).toBe(2)
  })

  it('counts other centers left of the cursor', () => {
    // Moving 'a'; others b(170), c(290). Cursor at 200 → past b only.
    expect(dropIndexForCursor(detected, 200, 'a')).toBe(1)
  })
})
