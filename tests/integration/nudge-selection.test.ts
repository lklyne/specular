/**
 * `nudgeSelection` — the arrow-key nudge, driven in-process against the real
 * runtime. Unlike a drag it never snaps: a 5px step off the grid stays off the
 * grid. Each keypress is its own undo step, and a selected group carries its
 * descendants.
 *
 * Mutation-verified by:
 *   - snapping the nudged position to `GRID_SIZE` in `nudgeSelection`
 *     (src/main/runtime/document-commands.ts) — the off-grid case fails.
 *   - skipping the group branch in `moveEntityTo` — the group case fails (the
 *     child stays behind).
 *   - dropping the `mutateWorkspace` wrapper — the undo case fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createShapeEntity,
  createTextEntity,
  getShapeEntities,
  getTextEntities,
  nudgeSelection,
} from '../../src/main/runtime/document-commands'
import { createUserGroup } from '../../src/main/workspace-groups'
import { workspaceGroups } from '../../src/main/runtime/space-model'
import { selectEntities, selectGroup, selectNone } from '../../src/main/runtime/selection-controller'
import { undo } from '../../src/main/runtime/space-undo'

let harness: WorkspaceHarness

function shapePos(id: string) {
  const s = getShapeEntities().find((e) => e.id === id)!
  return { x: s.canvasX, y: s.canvasY }
}

describe('nudge selection', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('moves by an exact delta without snapping, and each nudge undoes on its own', async () => {
    const shape = createShapeEntity({ canvasX: 13, canvasY: 7, width: 100, height: 100 })
    await settleSync()
    selectEntities([shape.id])

    nudgeSelection(5, 0)
    await settleSync()
    expect(shapePos(shape.id)).toEqual({ x: 18, y: 7 })

    nudgeSelection(0, -20)
    await settleSync()
    expect(shapePos(shape.id)).toEqual({ x: 18, y: -13 })

    undo()
    await settleSync()
    expect(shapePos(shape.id)).toEqual({ x: 18, y: 7 })

    undo()
    await settleSync()
    expect(shapePos(shape.id)).toEqual({ x: 13, y: 7 })
  })

  it('carries group descendants', async () => {
    const childA = createTextEntity({ canvasX: 0, canvasY: 0, width: 100, height: 40 })
    const childB = createTextEntity({ canvasX: 200, canvasY: 0, width: 100, height: 40 })
    const group = createUserGroup([childA.id, childB.id], 'g')
    await settleSync()

    const before = workspaceGroups.find((g) => g.id === group.id)!.canvasX
    selectGroup(group.id)

    nudgeSelection(20, 0)
    await settleSync()

    expect(workspaceGroups.find((g) => g.id === group.id)!.canvasX).toBe(before + 20)
    expect(getTextEntities().find((t) => t.id === childA.id)!.canvasX).toBe(20)
  })

  it('does nothing with an empty selection', async () => {
    const shape = createShapeEntity({ canvasX: 40, canvasY: 40, width: 100, height: 100 })
    await settleSync()
    selectNone()

    nudgeSelection(5, 5)
    await settleSync()
    expect(shapePos(shape.id)).toEqual({ x: 40, y: 40 })
  })
})
