/**
 * Group + selection behavior against the real runtime, in-process.
 *
 * Guards the selection-controller paths the HTTP routes call (select entity /
 * group, enter group, deselect, ungroup-selects-freed) and the group
 * lifecycle: a created group persists to disk with its members' parentGroupId
 * and round-trips through undo/redo, including restoring prior nested
 * membership.
 *
 * Mutation-verified by: inverting the `parentGroupId` restore in
 * `ungroupUserGroup` (setting freed children to `undefined` instead of
 * `group.parentGroupId` — src/main/workspace-groups.ts) breaks the
 * prior-membership case; removing the `selectEntities(freedIds)` call in
 * `ungroupSelectedGroup` (src/main/runtime/document-commands.ts) breaks the
 * ungroup-selection case.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createShapeEntity,
  createTextEntity,
  getTextEntities,
  ungroupSelectedGroup,
} from '../../src/main/runtime/document-commands'
import { createUserGroup } from '../../src/main/workspace-groups'
import {
  enterGroup,
  selectEntity,
  selectGroup,
  selectNone,
} from '../../src/main/runtime/selection-controller'
import { getSelectionState } from '../../src/main/workspace-entities'
import { workspaceGroups } from '../../src/main/runtime/workspace-model'
import { undo, redo } from '../../src/main/runtime/workspace-undo'

let harness: WorkspaceHarness

async function createTextPair(): Promise<[string, string]> {
  const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'a' })
  const b = createTextEntity({ canvasX: 200, canvasY: 0, text: 'b' })
  await settleSync()
  return [a.id, b.id]
}

describe('groups + selection', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('selecting a group then deselect clears the selection', async () => {
    const [aId, bId] = await createTextPair()
    const group = createUserGroup([aId, bId], 'Select me')
    await settleSync()

    selectEntity(aId, 'text')
    selectGroup(group.id)
    const selection = getSelectionState()
    expect(selection.selectedGroupId).toBe(group.id)
    expect(selection.selectedEntityIds ?? []).toEqual([])

    selectNone()
    const cleared = getSelectionState()
    expect(cleared.selectedGroupId).toBeUndefined()
    expect(cleared.selectedEntityIds ?? []).toEqual([])
  })

  it('ungrouping selects the freed entities', async () => {
    const [aId, bId] = await createTextPair()
    const group = createUserGroup([aId, bId], 'Ungroup me')
    await settleSync()

    selectGroup(group.id)
    const freedIds = ungroupSelectedGroup()
    expect([...(freedIds ?? [])].sort()).toEqual([aId, bId].sort())

    const selection = getSelectionState()
    expect((selection.selectedEntityIds ?? []).slice().sort()).toEqual([aId, bId].sort())
    expect(selection.selectedGroupId).toBeUndefined()
    expect(workspaceGroups.some((g) => g.id === group.id)).toBe(false)
  })

  it('entering a group after group selection selects only its children', async () => {
    const [aId, bId] = await createTextPair()
    const outside = createTextEntity({ canvasX: 600, canvasY: 0, text: 'outside' })
    const group = createUserGroup([aId, bId], 'Enter me')
    await settleSync()

    selectGroup(group.id)
    enterGroup(group.id)

    const selection = getSelectionState()
    expect((selection.selectedEntityIds ?? []).slice().sort()).toEqual([aId, bId].sort())
    expect(selection.selectedEntityIds).not.toContain(outside.id)
    expect(selection.selectedGroupId).toBeUndefined()
  })

  it('persists a created group to disk with member parentGroupId', async () => {
    const s1 = createShapeEntity({ canvasX: 0, canvasY: 0 })
    const s2 = createShapeEntity({ canvasX: 300, canvasY: 0 })
    await settleSync()
    const group = createUserGroup([s1.id, s2.id], 'Persisted group')
    await settleSync()

    const disk = harness.diskDoc()
    const groupNode = disk?.nodes.find((n) => n.id === group.id)
    expect(groupNode).toMatchObject({ type: 'group', label: 'Persisted group' })
    for (const shapeId of [s1.id, s2.id]) {
      expect(disk?.nodes.find((n) => n.id === shapeId)).toMatchObject({
        type: 'shape',
        parentGroupId: group.id,
      })
    }
  })

  it('round-trips group creation through undo/redo', async () => {
    const [aId, bId] = await createTextPair()
    const group = createUserGroup([aId, bId], 'Undoable group')
    await settleSync()

    const membership = () =>
      getTextEntities()
        .filter((t) => t.id === aId || t.id === bId)
        .map((t) => t.parentGroupId)
    expect(membership()).toEqual([group.id, group.id])

    undo()
    expect(workspaceGroups.some((g) => g.id === group.id)).toBe(false)
    // Members survive the ungrouping.
    expect(getTextEntities().map((t) => t.id).sort()).toEqual([aId, bId].sort())
    expect(membership()).toEqual([undefined, undefined])

    redo()
    expect(workspaceGroups.some((g) => g.id === group.id)).toBe(true)
    expect(membership()).toEqual([group.id, group.id])
  })

  it('undo of grouping restores prior nested membership', async () => {
    const [aId, bId] = await createTextPair()
    const inner = createUserGroup([aId, bId], 'Inner')
    await settleSync()
    // Grouping already-grouped members normalizes to grouping the inner group.
    const outer = createUserGroup([aId, bId], 'Outer')
    await settleSync()

    const innerGroup = () => workspaceGroups.find((g) => g.id === inner.id)
    expect(innerGroup()?.parentGroupId).toBe(outer.id)

    undo()
    expect(workspaceGroups.some((g) => g.id === outer.id)).toBe(false)
    expect(innerGroup()?.parentGroupId).toBeUndefined()
    expect(
      getTextEntities()
        .filter((t) => t.id === aId || t.id === bId)
        .every((t) => t.parentGroupId === inner.id),
    ).toBe(true)

    redo()
    expect(workspaceGroups.some((g) => g.id === outer.id)).toBe(true)
    expect(innerGroup()?.parentGroupId).toBe(outer.id)
  })
})
