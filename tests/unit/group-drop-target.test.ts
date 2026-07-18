import { describe, expect, it } from 'vitest'
import { groupDropTargetAt } from '../../src/shared/group-drop-target'
import type { CanvasSceneGroupEntity } from '../../src/shared/types'

function group(
  id: string,
  x: number,
  y: number,
  width: number,
  height: number,
): CanvasSceneGroupEntity {
  return {
    id,
    kind: 'group',
    label: id,
    canvasX: x,
    canvasY: y,
    width,
    height,
    screenX: x,
    screenY: y,
    screenWidth: width,
    screenHeight: height,
    layoutMode: 'freeform',
    managedLayout: false,
    entityIds: [],
  }
}

describe('groupDropTargetAt', () => {
  const outer = group('outer', 100, 100, 500, 500)
  const inner = group('inner', 200, 200, 120, 120)

  it('chooses the innermost group under the release pointer', () => {
    expect(groupDropTargetAt([outer, inner], { x: 250, y: 250 }, new Set())).toBe('inner')
  })

  it('uses the containing outer group when the pointer is outside the inner group', () => {
    expect(groupDropTargetAt([outer, inner], { x: 450, y: 450 }, new Set())).toBe('outer')
  })

  it('returns null outside every pre-drag group bound', () => {
    expect(groupDropTargetAt([outer, inner], { x: 50, y: 50 }, new Set())).toBeNull()
  })

  it('never targets a group traveling with the dragged subtree', () => {
    expect(groupDropTargetAt([outer, inner], { x: 250, y: 250 }, new Set(['inner']))).toBe('outer')
  })
})
