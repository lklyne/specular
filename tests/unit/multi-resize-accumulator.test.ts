import { describe, expect, it } from 'vitest'
import type { AnnotationDrawingStroke, CanvasSceneEntity } from '../../src/shared/types'
import {
  applyMultiHandleDelta,
  computeMultiSelectionBbox,
  MIN_MULTI_BBOX,
  startMultiResize,
  type MultiResizeAccumulator,
  type MultiResizeStart,
} from '../../src/shared/multi-resize-accumulator'

function entity(
  id: string,
  canvasX: number,
  canvasY: number,
  width: number,
  height: number,
): CanvasSceneEntity {
  return {
    id,
    kind: 'shape',
    canvasX,
    canvasY,
    width,
    height,
    screenX: 0,
    screenY: 0,
    screenWidth: width,
    screenHeight: height,
    rotation: 0,
    color: '#000',
    shape: 'rectangle',
    text: '',
  } as unknown as CanvasSceneEntity
}

function stroke(
  id: string,
  points: { x: number; y: number }[],
): AnnotationDrawingStroke {
  return { id, color: '#000', width: 2, points }
}

function fresh(): MultiResizeAccumulator {
  const start: MultiResizeStart = {
    bbox: { x: 0, y: 0, width: 200, height: 100 },
    entities: [
      { id: 'a', kind: 'shape', canvasX: 0, canvasY: 0, width: 100, height: 100 },
      { id: 'b', kind: 'shape', canvasX: 100, canvasY: 0, width: 100, height: 100 },
    ],
  }
  return startMultiResize(start)
}

describe('multi-resize-accumulator', () => {
  describe('computeMultiSelectionBbox', () => {
    it('aggregates the bbox over all selected entities', () => {
      const entities = [
        entity('a', 10, 20, 50, 30),
        entity('b', 100, 200, 80, 40),
      ]
      const result = computeMultiSelectionBbox(entities, ['a', 'b'])
      expect(result).not.toBeNull()
      expect(result!.bbox).toEqual({ x: 10, y: 20, width: 170, height: 220 })
      expect(result!.entities).toHaveLength(2)
    })

    it('returns null with fewer than two selections', () => {
      const entities = [entity('a', 0, 0, 10, 10)]
      expect(computeMultiSelectionBbox(entities, [])).toBeNull()
      expect(computeMultiSelectionBbox(entities, ['a'])).toBeNull()
    })

    it('skips groups (they own a separate selection overlay)', () => {
      const group = entity('g', 0, 0, 100, 100)
      ;(group as { kind: string }).kind = 'group'
      const entities = [entity('a', 0, 0, 10, 10), entity('b', 50, 50, 10, 10), group]
      const result = computeMultiSelectionBbox(entities, ['a', 'b', 'g'])
      expect(result!.entities.map((e) => e.id)).toEqual(['a', 'b'])
    })

    it('ignores ids that are not in the entity list', () => {
      const entities = [entity('a', 0, 0, 10, 10), entity('b', 20, 0, 10, 10)]
      const result = computeMultiSelectionBbox(entities, ['a', 'b', 'missing'])
      expect(result!.entities).toHaveLength(2)
    })
  })

  describe('startMultiResize', () => {
    it('snapshots the bbox and entities so later moves recompute from start', () => {
      const acc = fresh()
      expect(acc.initialBbox).toEqual({ x: 0, y: 0, width: 200, height: 100 })
      expect(acc.accW).toBe(200)
      expect(acc.accH).toBe(100)
      expect(acc.initialEntities).toHaveLength(2)
    })
  })

  describe('applyMultiHandleDelta — corners', () => {
    it('se grows width and height proportionally; entities scale and shift in place', () => {
      const acc = fresh()
      const out = applyMultiHandleDelta(acc, 'se', { screenDx: 100, screenDy: 50, zoom: 1 })
      expect(acc.accW).toBe(300)
      expect(acc.accH).toBe(150)
      expect(acc.accX).toBe(0)
      expect(acc.accY).toBe(0)
      expect(out[0]).toMatchObject({ id: 'a', canvasX: 0, canvasY: 0, width: 150, height: 150 })
      expect(out[1]).toMatchObject({ id: 'b', canvasX: 150, canvasY: 0, width: 150, height: 150 })
    })

    it('nw shrinks width and shifts the bbox origin toward the cursor', () => {
      const acc = fresh()
      const out = applyMultiHandleDelta(acc, 'nw', { screenDx: 20, screenDy: 10, zoom: 1 })
      expect(acc.accW).toBe(180)
      expect(acc.accH).toBe(90)
      expect(acc.accX).toBe(20)
      expect(acc.accY).toBe(10)
      const scaleX = 180 / 200
      const scaleY = 90 / 100
      expect(out[0]).toMatchObject({
        id: 'a',
        canvasX: Math.round(20 + 0 * scaleX),
        canvasY: Math.round(10 + 0 * scaleY),
        width: Math.round(100 * scaleX),
        height: Math.round(100 * scaleY),
      })
      expect(out[1]).toMatchObject({
        id: 'b',
        canvasX: Math.round(20 + 100 * scaleX),
        canvasY: Math.round(10 + 0 * scaleY),
      })
    })

    it('zoom rescales the screen delta into canvas space', () => {
      const acc = fresh()
      applyMultiHandleDelta(acc, 'se', { screenDx: 100, screenDy: 0, zoom: 2 })
      expect(acc.accW).toBe(250)
    })

    it('resizes drawing strokes from their entity bounds instead of canvas origin', () => {
      const acc = startMultiResize({
        bbox: { x: 100, y: 200, width: 200, height: 100 },
        entities: [
          {
            id: 'drawing-a',
            kind: 'drawing',
            canvasX: 100,
            canvasY: 200,
            width: 100,
            height: 100,
            strokes: [stroke('ink', [{ x: 110, y: 220 }, { x: 190, y: 260 }])],
          },
          { id: 'shape-b', kind: 'shape', canvasX: 200, canvasY: 200, width: 100, height: 100 },
        ],
      })

      const [drawing] = applyMultiHandleDelta(acc, 'se', { screenDx: 200, screenDy: 100, zoom: 1 })

      expect(drawing).toMatchObject({ canvasX: 100, canvasY: 200, width: 200, height: 200 })
      expect(drawing.strokes?.[0].points).toEqual([{ x: 120, y: 240 }, { x: 280, y: 320 }])
    })
  })

  describe('applyMultiHandleDelta — edges', () => {
    it('e grows width only', () => {
      const acc = fresh()
      const out = applyMultiHandleDelta(acc, 'e', { screenDx: 50, screenDy: 999, zoom: 1 })
      expect(acc.accW).toBe(250)
      expect(acc.accH).toBe(100)
      expect(out[0].height).toBe(100)
    })

    it('w shrinks width and shifts origin', () => {
      const acc = fresh()
      const out = applyMultiHandleDelta(acc, 'w', { screenDx: 40, screenDy: 0, zoom: 1 })
      expect(acc.accW).toBe(160)
      expect(acc.accX).toBe(40)
      expect(out[1]).toMatchObject({ id: 'b' })
    })

    it('n shifts y origin', () => {
      const acc = fresh()
      applyMultiHandleDelta(acc, 'n', { screenDx: 0, screenDy: 25, zoom: 1 })
      expect(acc.accH).toBe(75)
      expect(acc.accY).toBe(25)
      expect(acc.accW).toBe(200)
    })
  })

  describe('applyMultiHandleDelta — clamps', () => {
    it('clamps the bbox below MIN_MULTI_BBOX', () => {
      const acc = fresh()
      applyMultiHandleDelta(acc, 'se', { screenDx: -1000, screenDy: -1000, zoom: 1 })
      expect(acc.accW).toBe(MIN_MULTI_BBOX)
      expect(acc.accH).toBe(MIN_MULTI_BBOX)
    })

    it('floors entity dimensions at 1 even when scale is tiny', () => {
      const acc = fresh()
      const out = applyMultiHandleDelta(acc, 'se', { screenDx: -1000, screenDy: -1000, zoom: 1 })
      for (const entry of out) {
        expect(entry.width).toBeGreaterThanOrEqual(1)
        expect(entry.height).toBeGreaterThanOrEqual(1)
      }
    })
  })

  describe('applyMultiHandleDelta — composition', () => {
    it('successive ticks accumulate against the initial snapshot, not last result', () => {
      const acc = fresh()
      applyMultiHandleDelta(acc, 'se', { screenDx: 50, screenDy: 25, zoom: 1 })
      const out = applyMultiHandleDelta(acc, 'se', { screenDx: 50, screenDy: 25, zoom: 1 })
      // Total drag: dx=100, dy=50 → bbox (0,0,300,150). Each entity at 1.5x.
      expect(acc.accW).toBe(300)
      expect(acc.accH).toBe(150)
      expect(out[0]).toMatchObject({ width: 150, height: 150 })
    })
  })
})
