import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { createGroupEntity } from '../../src/main/runtime/group-entity-state'
import { updateGroupEntity } from '../../src/main/runtime/document-commands'
import { commitActive, tryEnter } from '../../src/main/runtime/interaction-controller'

let harness: WorkspaceHarness

describe('single-item resize geometry', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('does not independently grid-snap origin and size during an active resize', () => {
    const group = createGroupEntity({
      id: 'resize-group',
      canvasX: 0,
      canvasY: 0,
      width: 100,
      height: 100,
    })
    const token = tryEnter({
      kind: 'resizing-entity',
      target: { id: group.id, kind: 'group' },
    })
    expect(token).not.toHaveProperty('refused')

    updateGroupEntity(group.id, {
      canvasX: 13,
      canvasY: 17,
      width: 102,
      height: 103,
    })

    expect(group).toMatchObject({
      canvasX: 13,
      canvasY: 17,
      width: 102,
      height: 103,
    })
    expect(group.canvasX + group.width).toBe(115)
    expect(group.canvasY + group.height).toBe(120)
    commitActive()
  })
})
