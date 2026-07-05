import { describe, expect, it } from 'vitest'
import { scaleStrokesToBounds } from '../../src/shared/scale-strokes'
import type { AnnotationDrawingStroke } from '../../src/shared/types'

function stroke(
  id: string,
  points: { x: number; y: number }[],
  width = 2,
): AnnotationDrawingStroke {
  return { id, color: '#000', width, points }
}

describe('scaleStrokesToBounds', () => {
  it('scales absolute point coordinates inside moved bounds', () => {
    const strokes = [stroke('a', [{ x: 10, y: 20 }, { x: 30, y: 40 }])]
    const result = scaleStrokesToBounds(
      strokes,
      { canvasX: 10, canvasY: 20, width: 20, height: 20 },
      { canvasX: 100, canvasY: 200, width: 40, height: 60 },
    )
    expect(result[0].points).toEqual([{ x: 100, y: 200 }, { x: 140, y: 260 }])
  })

  it('identity scale returns the same coordinates', () => {
    const strokes = [stroke('a', [{ x: 5, y: 7 }])]
    const result = scaleStrokesToBounds(
      strokes,
      { canvasX: 0, canvasY: 0, width: 10, height: 10 },
      { canvasX: 0, canvasY: 0, width: 10, height: 10 },
    )
    expect(result[0].points).toEqual([{ x: 5, y: 7 }])
  })

  it('non-uniform scale squashes x and y independently', () => {
    const strokes = [stroke('a', [{ x: 100, y: 50 }])]
    const result = scaleStrokesToBounds(
      strokes,
      { canvasX: 0, canvasY: 0, width: 200, height: 100 },
      { canvasX: 0, canvasY: 0, width: 100, height: 200 },
    )
    expect(result[0].points).toEqual([{ x: 50, y: 100 }])
  })

  it('preserves brush width unchanged', () => {
    const strokes = [stroke('a', [{ x: 1, y: 1 }], 5)]
    const result = scaleStrokesToBounds(
      strokes,
      { canvasX: 0, canvasY: 0, width: 10, height: 10 },
      { canvasX: 0, canvasY: 0, width: 30, height: 30 },
    )
    expect(result[0].width).toBe(5)
  })

  it('preserves all other stroke fields (id, color, brushType)', () => {
    const s: AnnotationDrawingStroke = {
      id: 'test-id',
      color: '#ff0000',
      width: 4,
      points: [{ x: 10, y: 10 }],
      brushType: 'marker',
    }
    const [result] = scaleStrokesToBounds(
      [s],
      { canvasX: 0, canvasY: 0, width: 100, height: 100 },
      { canvasX: 0, canvasY: 0, width: 200, height: 200 },
    )
    expect(result.id).toBe('test-id')
    expect(result.color).toBe('#ff0000')
    expect(result.width).toBe(4)
    expect(result.brushType).toBe('marker')
  })

  it('returns empty array for empty input', () => {
    expect(scaleStrokesToBounds(
      [],
      { canvasX: 0, canvasY: 0, width: 10, height: 10 },
      { canvasX: 0, canvasY: 0, width: 20, height: 20 },
    )).toEqual([])
  })

  it('handles a stroke with a single point', () => {
    const strokes = [stroke('a', [{ x: 7, y: 3 }])]
    const result = scaleStrokesToBounds(
      strokes,
      { canvasX: 0, canvasY: 0, width: 10, height: 10 },
      { canvasX: 0, canvasY: 0, width: 20, height: 20 },
    )
    expect(result[0].points).toEqual([{ x: 14, y: 6 }])
  })

  it('does not mutate the original strokes array', () => {
    const original = [stroke('a', [{ x: 10, y: 20 }])]
    scaleStrokesToBounds(
      original,
      { canvasX: 0, canvasY: 0, width: 20, height: 20 },
      { canvasX: 0, canvasY: 0, width: 40, height: 40 },
    )
    expect(original[0].points[0]).toEqual({ x: 10, y: 20 })
  })

  it('keeps points inside a drawing when resizing from a non-zero origin', () => {
    const strokes = [stroke('a', [{ x: 110, y: 220 }, { x: 190, y: 260 }])]
    const result = scaleStrokesToBounds(
      strokes,
      { canvasX: 100, canvasY: 200, width: 100, height: 100 },
      { canvasX: 100, canvasY: 200, width: 200, height: 50 },
    )
    expect(result[0].points).toEqual([{ x: 120, y: 210 }, { x: 280, y: 230 }])
  })

  it('moves points with the top-left corner during nw resize', () => {
    const strokes = [stroke('a', [{ x: 110, y: 220 }, { x: 190, y: 260 }])]
    const result = scaleStrokesToBounds(
      strokes,
      { canvasX: 100, canvasY: 200, width: 100, height: 100 },
      { canvasX: 120, canvasY: 210, width: 80, height: 90 },
    )
    expect(result[0].points).toEqual([{ x: 128, y: 228 }, { x: 192, y: 264 }])
  })
})
