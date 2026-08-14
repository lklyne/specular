import { describe, expect, it } from 'vitest'
import type { CanvasSceneEntity } from '../../src/shared/types'
import { selectionBbox } from '../../src/shared/selection-bbox'

function entity(
  id: string,
  canvasX: number,
  canvasY: number,
  width: number,
  height: number,
  screen: { x: number; y: number; width: number; height: number } = {
    x: canvasX,
    y: canvasY,
    width,
    height,
  },
  kind: string = 'shape',
): CanvasSceneEntity {
  return {
    id,
    kind,
    canvasX,
    canvasY,
    width,
    height,
    screenX: screen.x,
    screenY: screen.y,
    screenWidth: screen.width,
    screenHeight: screen.height,
    rotation: 0,
    color: '#000',
    shape: 'rectangle',
    text: '',
  } as unknown as CanvasSceneEntity
}

describe('selectionBbox', () => {
  it('unions canvas-space rects of the selected entities', () => {
    const entities = [entity('a', 10, 20, 50, 30), entity('b', 100, 200, 80, 40)]
    expect(selectionBbox(entities, ['a', 'b'], 'canvas')).toEqual({
      x: 10,
      y: 20,
      width: 170,
      height: 220,
    })
  })

  it('unions screen-space rects of the selected entities', () => {
    const entities = [
      entity('a', 0, 0, 50, 30, { x: 5, y: 10, width: 25, height: 15 }),
      entity('b', 0, 0, 80, 40, { x: 50, y: 100, width: 40, height: 20 }),
    ]
    expect(selectionBbox(entities, ['a', 'b'], 'screen')).toEqual({
      x: 5,
      y: 10,
      width: 85,
      height: 110,
    })
  })

  it('returns null with fewer than two matching selections', () => {
    const entities = [entity('a', 0, 0, 10, 10)]
    expect(selectionBbox(entities, [], 'canvas')).toBeNull()
    expect(selectionBbox(entities, ['a'], 'canvas')).toBeNull()
  })

  it('includes a group rect — the box wraps the group border, padding and all', () => {
    const entities = [
      entity('a', 0, 0, 10, 10),
      entity('b', 50, 50, 10, 10),
      entity('g', 0, 0, 100, 100, undefined, 'group'),
    ]
    expect(selectionBbox(entities, ['a', 'b', 'g'], 'canvas')).toEqual({
      x: 0,
      y: 0,
      width: 100,
      height: 100,
    })
  })

  it('ignores ids that are not in the entity list', () => {
    const entities = [entity('a', 0, 0, 10, 10), entity('b', 20, 0, 10, 10)]
    expect(selectionBbox(entities, ['a', 'b', 'missing'], 'canvas')).toEqual({
      x: 0,
      y: 0,
      width: 30,
      height: 10,
    })
  })
})
