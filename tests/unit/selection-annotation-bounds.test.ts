/**
 * Mutation-verified by restoring the `selectionBbox` delegation in
 * selectionAnnotationBounds (the single-entity cases below then return null)
 * and by dropping the group branch's rect (the group case returns null).
 */

import { describe, expect, it } from 'vitest'
import { selectionAnnotationBounds } from '../../src/renderer/above-view/annotationMath'
import type { CanvasSceneEntity } from '../../src/shared/types'

function entity(
  id: string,
  kind: CanvasSceneEntity['kind'],
  canvasX: number,
  canvasY: number,
  width = 100,
  height = 100,
): CanvasSceneEntity {
  return {
    id,
    kind,
    canvasX,
    canvasY,
    width,
    height,
    screenX: 0,
    screenY: 0,
    screenWidth: 0,
    screenHeight: 0,
  } as CanvasSceneEntity
}

const ENTITIES: CanvasSceneEntity[] = [
  entity('file-1', 'file', 0, 0),
  entity('shape-1', 'shape', 200, 200),
  entity('group-1', 'group', 500, 500, 300, 300),
]

describe('selectionAnnotationBounds', () => {
  // A lone file entity is what gives the fix loop a selectionTarget it can
  // duplicate beside; suppressing the button here made that path unreachable
  // from the canvas.
  it('returns the entity rect for a single selected item', () => {
    expect(selectionAnnotationBounds(ENTITIES, ['file-1'])).toEqual({
      x: 0,
      y: 0,
      width: 100,
      height: 100,
    })
  })

  it('unions across a multi-selection', () => {
    expect(selectionAnnotationBounds(ENTITIES, ['file-1', 'shape-1'])).toEqual({
      x: 0,
      y: 0,
      width: 300,
      height: 300,
    })
  })

  it('uses a lone group own rect rather than skipping it', () => {
    expect(selectionAnnotationBounds(ENTITIES, ['group-1'])).toEqual({
      x: 500,
      y: 500,
      width: 300,
      height: 300,
    })
  })

  it('returns null when no id resolves against the layout', () => {
    expect(selectionAnnotationBounds(ENTITIES, ['ghost'])).toBeNull()
    expect(selectionAnnotationBounds(ENTITIES, [])).toBeNull()
  })
})
