/**
 * Group + selection behavior against the real runtime, in-process.
 *
 * Guards the selection-controller paths the HTTP routes call (select entity /
 * group, enter group, deselect, ungroup-selects-freed) and the group
 * lifecycle: a created group persists to disk with its members' parentGroupId
 * and round-trips through undo/redo, including restoring prior nested
 * membership.
 *
 * Mutation-verified by: deleting the `setEntityParentGroupId(entityId,
 * group.id)` loop in `createUserGroup` (src/main/workspace-groups.ts) — 5 of
 * 6 cases fail (only the pure select/deselect case survives).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import {
  createShapeEntity,
  createTextEntity,
  getShapeEntities,
  getTextEntities,
  setPageColorScheme,
  ungroupSelectedGroup,
  updateGroupEntity,
} from '../../src/main/runtime/document-commands'
import { createUserGroup, duplicateGroup } from '../../src/main/workspace-groups'
import { reparentEntities } from '../../src/main/runtime/group-membership'
import {
  enterGroup,
  selectEntity,
  selectGroup,
  selectNone,
} from '../../src/main/runtime/selection-controller'
import {
  getSelectionState,
  selectEntitiesInRect,
} from '../../src/main/workspace-entities'
import { workspaceGroups } from '../../src/main/runtime/space-model'
import { undo, redo } from '../../src/main/runtime/space-undo'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { findPageById } from '../../src/main/runtime/runtime-context'

let harness: WorkspaceHarness

function linkNode(id: string, x: number): JsonCanvasLinkNode {
  return {
    id,
    type: 'link',
    x,
    y: 120,
    width: 375,
    height: 667,
    url: `https://example.com/${id}`,
    presetIndex: 0,
  }
}

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

  it('box-selecting the full group bounds selects the group instead of its children', async () => {
    const [aId, bId] = await createTextPair()
    const group = createUserGroup([aId, bId], 'Box me')
    await settleSync()

    selectEntitiesInRect({
      x: group.canvasX,
      y: group.canvasY,
      width: group.width,
      height: group.height,
    })

    const selection = getSelectionState()
    expect(selection.selectedGroupId).toBe(group.id)
    expect(selection.selectedEntityIds ?? []).toEqual([])
  })

  it('box-selecting multiple full groups batches the groups without their children', async () => {
    const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'a' })
    const b = createTextEntity({ canvasX: 200, canvasY: 0, text: 'b' })
    const c = createTextEntity({ canvasX: 600, canvasY: 0, text: 'c' })
    const d = createTextEntity({ canvasX: 800, canvasY: 0, text: 'd' })
    await settleSync()
    const first = createUserGroup([a.id, b.id], 'First')
    const second = createUserGroup([c.id, d.id], 'Second')
    await settleSync()

    const left = Math.min(first.canvasX, second.canvasX)
    const top = Math.min(first.canvasY, second.canvasY)
    const right = Math.max(first.canvasX + first.width, second.canvasX + second.width)
    const bottom = Math.max(first.canvasY + first.height, second.canvasY + second.height)
    selectEntitiesInRect({ x: left, y: top, width: right - left, height: bottom - top })

    const selection = getSelectionState()
    expect((selection.selectedEntityIds ?? []).slice().sort()).toEqual(
      [first.id, second.id].sort(),
    )
    expect(selection.selectedGroupId).toBeUndefined()
  })

  it('batches a full group with intersected children from a partial group', async () => {
    const a = createTextEntity({ canvasX: 0, canvasY: 0, text: 'a' })
    const b = createTextEntity({ canvasX: 200, canvasY: 0, text: 'b' })
    const c = createTextEntity({ canvasX: 600, canvasY: 0, text: 'c' })
    const d = createTextEntity({ canvasX: 1000, canvasY: 0, text: 'd' })
    await settleSync()
    const full = createUserGroup([a.id, b.id], 'Full')
    const partial = createUserGroup([c.id, d.id], 'Partial')
    await settleSync()

    const left = Math.min(full.canvasX, partial.canvasX)
    const top = Math.min(full.canvasY, partial.canvasY)
    const right = c.canvasX + c.width
    const bottom = Math.max(full.canvasY + full.height, c.canvasY + c.height)
    selectEntitiesInRect({ x: left, y: top, width: right - left, height: bottom - top })

    const selection = getSelectionState()
    expect((selection.selectedEntityIds ?? []).slice().sort()).toEqual(
      [full.id, c.id].sort(),
    )
    expect(selection.selectedEntityIds).not.toContain(a.id)
    expect(selection.selectedEntityIds).not.toContain(b.id)
    expect(selection.selectedEntityIds).not.toContain(partial.id)
    expect(selection.selectedEntityIds).not.toContain(d.id)
  })

  it('persists a created group to disk with member parentGroupId', async () => {
    harness.loadFixture({
      name: 'Group persist',
      doc: {
        nodes: [linkNode('page-a', 120), linkNode('page-b', 620)],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })
    const group = createUserGroup(['page-a', 'page-b'], 'Persisted group')
    await settleSync()

    const disk = harness.diskDoc()
    const groupNode = disk?.nodes.find((n) => n.id === group.id)
    expect(groupNode).toMatchObject({ type: 'group', label: 'Persisted group' })
    for (const pageId of ['page-a', 'page-b']) {
      expect(disk?.nodes.find((n) => n.id === pageId)).toMatchObject({
        type: 'link',
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

  it('groups shape entities and round-trips membership through undo/redo', async () => {
    const a = createShapeEntity({ canvasX: 0, canvasY: 0, shapeKind: 'rectangle' })
    const b = createShapeEntity({ canvasX: 200, canvasY: 0, shapeKind: 'rectangle' })
    await settleSync()
    const group = createUserGroup([a.id, b.id], 'Shapes')
    await settleSync()

    const membership = () =>
      getShapeEntities()
        .filter((s) => s.id === a.id || s.id === b.id)
        .map((s) => s.parentGroupId)
    expect(membership()).toEqual([group.id, group.id])

    undo()
    expect(workspaceGroups.some((g) => g.id === group.id)).toBe(false)
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

  it('reparents an entity on group drop and removes it on a drop outside', async () => {
    const member = createTextEntity({ canvasX: 0, canvasY: 0, text: 'member' })
    const seed = createTextEntity({ canvasX: 400, canvasY: 0, text: 'seed' })
    await settleSync()
    const group = createUserGroup([seed.id], 'Drop target')
    await settleSync()

    expect(reparentEntities([member.id], group.id)).toEqual([member.id])
    await settleSync()
    expect(getTextEntities().find((entity) => entity.id === member.id)?.parentGroupId).toBe(group.id)

    expect(reparentEntities([member.id], null)).toEqual([member.id])
    await settleSync()
    expect(getTextEntities().find((entity) => entity.id === member.id)?.parentGroupId).toBeUndefined()

    undo()
    expect(getTextEntities().find((entity) => entity.id === member.id)?.parentGroupId).toBe(group.id)
    redo()
    expect(getTextEntities().find((entity) => entity.id === member.id)?.parentGroupId).toBeUndefined()
  })

  it('resizing a group from its top-left leaves member positions unchanged', async () => {
    const member = createTextEntity({ canvasX: 100, canvasY: 100, text: 'stationary' })
    await settleSync()
    const group = createUserGroup([member.id], 'Resizable')
    await settleSync()
    const before = getTextEntities().find((entity) => entity.id === member.id)!

    updateGroupEntity(group.id, {
      canvasX: group.canvasX - 40,
      canvasY: group.canvasY - 20,
      width: group.width + 40,
      height: group.height + 20,
    })

    const after = getTextEntities().find((entity) => entity.id === member.id)!
    expect({ x: after.canvasX, y: after.canvasY }).toEqual({
      x: before.canvasX,
      y: before.canvasY,
    })
  })
})

describe('group duplicate', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  // Mutation-verified by deleting `colorScheme: page.colorScheme` from the
  // page-cloning branch of `duplicateGroupInternal` (src/main/workspace-groups.ts).
  it('carries a member page colorScheme override to its clone', async () => {
    const result = applyCanvasPatch({
      entities: [
        { kind: 'page', url: 'https://example.com', canvasX: 0, canvasY: 0, presetIndex: 0 },
      ],
    })
    const pageId = result.created[0]
    await settleSync()
    setPageColorScheme(pageId, 'dark')
    await settleSync()

    const group = createUserGroup([pageId], 'Page group')
    await settleSync()

    const { entityIds } = duplicateGroup({ groupId: group.id })
    await settleSync()

    expect(entityIds).toHaveLength(1)
    expect(findPageById(entityIds[0])?.colorScheme).toBe('dark')
  })

  it('duplicates all members and preserves nested group membership for option-copy', async () => {
    const note = createTextEntity({ canvasX: 0, canvasY: 0, text: 'inside' })
    const shape = createShapeEntity({
      canvasX: 220,
      canvasY: 0,
      shapeKind: 'rectangle',
    })
    await settleSync()
    const inner = createUserGroup([note.id], 'Inner')
    const outer = createUserGroup([note.id, shape.id], 'Outer')
    await settleSync()

    const result = duplicateGroup({
      groupId: outer.id,
      placement: { canvasX: 800, canvasY: 600 },
    })
    await settleSync()

    const clonedOuter = workspaceGroups.find((group) => group.id === result.groupId)
    const clonedInner = workspaceGroups.find(
      (group) => group.id !== inner.id && group.parentGroupId === result.groupId,
    )
    expect(clonedOuter).toBeDefined()
    expect(clonedInner).toBeDefined()
    expect(result.entityIds).toHaveLength(2)
    expect(
      getTextEntities().some(
        (entity) => entity.id !== note.id && entity.parentGroupId === clonedInner?.id,
      ),
    ).toBe(true)
    expect(
      getShapeEntities().some(
        (entity) => entity.id !== shape.id && entity.parentGroupId === result.groupId,
      ),
    ).toBe(true)
  })
})
