import { describe, expect, it } from 'vitest'
import { scaleStrokes } from '../../src/shared/scale-strokes'
import type { AnnotationDrawingStroke } from '../../src/shared/types'

function stroke(
  id: string,
  points: { x: number; y: number }[],
  width = 2,
): AnnotationDrawingStroke {
  return { id, color: '#000', width, points }
}

describe('scaleStrokes', () => {
  it('scales point coordinates by (scaleX, scaleY)', () => {
    const strokes = [stroke('a', [{ x: 10, y: 20 }, { x: 30, y: 40 }])]
    const result = scaleStrokes(strokes, 2, 3)
    expect(result[0].points).toEqual([{ x: 20, y: 60 }, { x: 60, y: 120 }])
  })

  it('identity scale returns the same coordinates', () => {
    const strokes = [stroke('a', [{ x: 5, y: 7 }])]
    const result = scaleStrokes(strokes, 1, 1)
    expect(result[0].points).toEqual([{ x: 5, y: 7 }])
  })

  it('non-uniform scale squashes x and y independently', () => {
    const strokes = [stroke('a', [{ x: 100, y: 50 }])]
    const result = scaleStrokes(strokes, 0.5, 2)
    expect(result[0].points).toEqual([{ x: 50, y: 100 }])
  })

  it('preserves brush width unchanged', () => {
    const strokes = [stroke('a', [{ x: 1, y: 1 }], 5)]
    const result = scaleStrokes(strokes, 3, 3)
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
    const [result] = scaleStrokes([s], 2, 2)
    expect(result.id).toBe('test-id')
    expect(result.color).toBe('#ff0000')
    expect(result.width).toBe(4)
    expect(result.brushType).toBe('marker')
  })

  it('returns empty array for empty input', () => {
    expect(scaleStrokes([], 2, 2)).toEqual([])
  })

  it('handles a stroke with a single point', () => {
    const strokes = [stroke('a', [{ x: 7, y: 3 }])]
    const result = scaleStrokes(strokes, 2, 2)
    expect(result[0].points).toEqual([{ x: 14, y: 6 }])
  })

  it('does not mutate the original strokes array', () => {
    const original = [stroke('a', [{ x: 10, y: 20 }])]
    scaleStrokes(original, 2, 2)
    expect(original[0].points[0]).toEqual({ x: 10, y: 20 })
  })
})
