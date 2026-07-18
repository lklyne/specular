import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { createGroupEntity } from '../../src/main/runtime/group-entity-state'
import { updateGroupEntity } from '../../src/main/runtime/document-commands'
import { commitActive, tryEnter } from '../../src/main/runtime/interaction-controller'
import { applyHandleDelta, startResize } from '../../src/shared/resize-accumulator'

let harness: WorkspaceHarness

describe('single-item resize geometry', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('accepts an edge-consistent snapped resize patch without moving the opposite edges', () => {
    const group = createGroupEntity({
      id: 'resize-group',
      canvasX: 13,
      canvasY: 17,
      width: 102,
      height: 103,
    })
    const token = tryEnter({
      kind: 'resizing-entity',
      target: { id: group.id, kind: 'group' },
    })
    expect(token).not.toHaveProperty('refused')

    const patch = applyHandleDelta(
      startResize(group),
      'nw',
      { screenDx: 17, screenDy: 17, zoom: 1, shiftKey: false },
      { minWidth: 10, minHeight: 10, aspectRatioResizeMode: 'off' },
    )
    updateGroupEntity(group.id, patch)

    expect(group).toMatchObject({
      canvasX: 40,
      canvasY: 40,
      width: 75,
      height: 80,
    })
    expect(group.canvasX + group.width).toBe(115)
    expect(group.canvasY + group.height).toBe(120)
    commitActive()
  })
})
