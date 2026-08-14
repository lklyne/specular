/**
 * Copy/paste carries groups through the selection scope (ADR 0034, "Groups
 * become copyable" — part 2). Before this, `copyableSelectionPayload` filtered
 * to pages plus the four map-backed kinds; a group anywhere in the selection
 * was silently dropped from the clipboard payload, and its descendants were
 * never expanded into it either.
 *
 * This net copies a mixed selection — an ungrouped sticky plus a group
 * containing a nested group — and asserts paste rebuilds the whole tree: both
 * groups clone with every declared field intact, every child clones with its
 * fields intact, membership remaps to the NEW cloned group ids (not the
 * source ids), the ungrouped item stays ungrouped, and relative offsets
 * between every pasted item are preserved.
 *
 * Mutation-verified two ways: dropping the `group` branch from
 * `entityPayloadAt` (workspace-clipboard.ts) — the pre-fix state, where a
 * selected group was silently skipped and never became a payload entry —
 * fails "copies a mixed selection" below with only 4 of the 6 expected
 * entities in the payload. Separately, disabling the `setEntityParentGroupId`
 * call in `pasteEntitiesInternal`'s remap loop fails the membership-remap
 * assertion with the inner group clone's `parentGroupId` staying `undefined`
 * instead of pointing at the outer group's clone.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import { createUserGroup } from '../../src/main/workspace-groups'
import { selectEntities, selectNone } from '../../src/main/runtime/selection-controller'
import { getEntityKind } from '../../src/main/entities/contract'
import {
  copyableSelectionPayload,
  pasteEntitiesFromClipboard,
} from '../../src/main/workspace-clipboard'
import type { TextEntity, WorkspaceGroup } from '../../src/shared/types'

let harness: WorkspaceHarness

/** Overwrite one text entity's non-placement fields in place through the
 *  registry's own restore, the same seam persistence itself writes through —
 *  not a second hand-rolled field-setter. */
function patchText(id: string, patch: Partial<TextEntity>): void {
  getEntityKind('text').restore(
    getEntityKind('text')
      .entities()
      .map((entity) => (entity.id === id ? { ...entity, ...patch } : entity)) as unknown as Record<
      string,
      unknown
    >[],
  )
}

/** Overwrite one group's non-placement fields in place, mirroring `patchText`. */
function patchGroup(id: string, patch: Partial<WorkspaceGroup>): void {
  getEntityKind('group').restore(
    getEntityKind('group')
      .entities()
      .map((group) => (group.id === id ? { ...group, ...patch } : group)) as unknown as Record<
      string,
      unknown
    >[],
  )
}

function textById(id: string): TextEntity {
  const entity = getEntityKind('text')
    .entities()
    .find((candidate) => candidate.id === id) as TextEntity | undefined
  if (!entity) throw new Error(`text entity ${id} not found`)
  return entity
}

function groupById(id: string): WorkspaceGroup {
  const group = getEntityKind('group')
    .entities()
    .find((candidate) => candidate.id === id) as WorkspaceGroup | undefined
  if (!group) throw new Error(`group ${id} not found`)
  return group
}

/**
 * Builds: a nested group tree (outer > [inner > [leafA, leafB], leafC]) plus
 * one ungrouped `outsider` sticky, selects [outer, outsider] — a mixed
 * selection containing a group with a nested group inside it, the shape the
 * ADR's "Groups become copyable" paragraph targets.
 */
async function buildNestedGroupSelection() {
  const leafA = createTextEntity({ canvasX: 0, canvasY: 0, text: 'leaf-a' })
  const leafB = createTextEntity({ canvasX: 100, canvasY: 0, text: 'leaf-b' })
  await settleSync()
  patchText(leafA.id, { color: '5', textStyle: 'sticky', widthMode: 'fixed', textSize: 28, label: 'Leaf A label' })
  patchText(leafB.id, { color: '2', textStyle: 'plain', widthMode: 'auto', textSize: 16, label: 'Leaf B label' })

  const inner = createUserGroup([leafA.id, leafB.id], 'Inner group')
  await settleSync()
  patchGroup(inner.id, { color: '4', layoutMode: 'column', managedLayout: true, layoutGap: 12, sourceTaskId: 'task_inner', metadata: { note: 'inner' } })

  const leafC = createTextEntity({ canvasX: 300, canvasY: 0, text: 'leaf-c' })
  await settleSync()
  patchText(leafC.id, { color: '7', textStyle: 'plain', widthMode: 'fixed', textSize: 20, label: 'Leaf C label' })

  const outer = createUserGroup([inner.id, leafC.id], 'Outer group')
  await settleSync()
  patchGroup(outer.id, { color: '1', layoutMode: 'row', managedLayout: false, layoutGap: 24, sourceTaskId: 'task_outer', metadata: { note: 'outer' } })

  const outsider = createTextEntity({ canvasX: 900, canvasY: 0, text: 'outsider' })
  await settleSync()
  patchText(outsider.id, { color: '3', textStyle: 'sticky', widthMode: 'auto', textSize: 22, label: 'Outsider label' })

  selectEntities([outer.id, outsider.id])

  return { leafA, leafB, leafC, inner, outer, outsider }
}

describe('clipboard copy/paste carries groups', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('copies a mixed selection (nested group + ungrouped sticky) and pastes the whole tree', async () => {
    const { leafA, leafB, leafC, inner, outer, outsider } = await buildNestedGroupSelection()

    const payload = copyableSelectionPayload()
    expect(payload, 'selection with a group produced no clipboard payload').toBeTruthy()
    // outer, inner, leafA, leafB, leafC, outsider — six operands total.
    expect(payload!.entities).toHaveLength(6)

    const pasteAt = { canvasX: 5000, canvasY: 5000 }
    const { entityIds } = pasteEntitiesFromClipboard({ payload: payload!, ...pasteAt })
    expect(entityIds).toHaveLength(6)

    const pastedGroups = getEntityKind('group')
      .entities()
      .filter((group) => group.id !== inner.id && group.id !== outer.id) as WorkspaceGroup[]
    expect(pastedGroups, 'both groups should have been cloned').toHaveLength(2)
    const pastedOuter = pastedGroups.find((group) => group.label === 'Outer group')!
    const pastedInner = pastedGroups.find((group) => group.label === 'Inner group')!
    expect(pastedOuter).toBeTruthy()
    expect(pastedInner).toBeTruthy()
    expect(pastedOuter.id).not.toBe(outer.id)
    expect(pastedInner.id).not.toBe(inner.id)

    // Every declared, non-placement field of the outer and inner group
    // survives the clone — this is the "group cloned with label/color/layout
    // fields intact" assertion (net item 3).
    const srcOuterFields = groupById(outer.id)
    const srcInnerFields = groupById(inner.id)
    const groupFieldsToCheck: (keyof WorkspaceGroup)[] = [
      'label', 'color', 'layoutMode', 'managedLayout', 'layoutGap', 'sourceTaskId', 'metadata',
    ]
    for (const field of groupFieldsToCheck) {
      expect(pastedOuter[field], `pasted outer group lost ${field}`).toEqual(srcOuterFields[field])
      expect(pastedInner[field], `pasted inner group lost ${field}`).toEqual(srcInnerFields[field])
    }

    // Membership remaps to the NEW cloned parent, never the source parent.
    expect(pastedInner.parentGroupId, 'inner clone should rejoin the outer CLONE').toBe(pastedOuter.id)
    expect(pastedInner.parentGroupId).not.toBe(outer.id)

    const pastedLeafA = getEntityKind('text')
      .entities()
      .find((entity) => entity.id !== leafA.id && entity.label === 'Leaf A label') as TextEntity
    const pastedLeafB = getEntityKind('text')
      .entities()
      .find((entity) => entity.id !== leafB.id && entity.label === 'Leaf B label') as TextEntity
    const pastedLeafC = getEntityKind('text')
      .entities()
      .find((entity) => entity.id !== leafC.id && entity.label === 'Leaf C label') as TextEntity
    const pastedOutsider = getEntityKind('text')
      .entities()
      .find((entity) => entity.id !== outsider.id && entity.label === 'Outsider label') as TextEntity

    expect(pastedLeafA, 'leaf A was not cloned').toBeTruthy()
    expect(pastedLeafB, 'leaf B was not cloned').toBeTruthy()
    expect(pastedLeafC, 'leaf C was not cloned').toBeTruthy()
    expect(pastedOutsider, 'outsider was not cloned').toBeTruthy()

    // leafA/leafB rejoin the cloned INNER group; leafC rejoins the cloned
    // OUTER group directly; outsider stays ungrouped — every child cloned
    // with membership remapped to the new group ids (net item 3/5).
    expect(pastedLeafA.parentGroupId).toBe(pastedInner.id)
    expect(pastedLeafB.parentGroupId).toBe(pastedInner.id)
    expect(pastedLeafC.parentGroupId).toBe(pastedOuter.id)
    expect(pastedOutsider.parentGroupId).toBeUndefined()

    // Every declared, non-placement field of every child survives the clone.
    const textFieldsToCheck: (keyof TextEntity)[] = ['text', 'color', 'textStyle', 'widthMode', 'textSize', 'label']
    for (const field of textFieldsToCheck) {
      expect(pastedLeafA[field]).toEqual(textById(leafA.id)[field])
      expect(pastedLeafB[field]).toEqual(textById(leafB.id)[field])
      expect(pastedLeafC[field]).toEqual(textById(leafC.id)[field])
      expect(pastedOutsider[field]).toEqual(textById(outsider.id)[field])
    }

    // Relative offsets between every pasted operand match the source's —
    // paste translates the whole selection uniformly (net item 3/5).
    const srcLeafA = textById(leafA.id)
    const srcLeafB = textById(leafB.id)
    const srcLeafC = textById(leafC.id)
    const srcOuter = groupById(outer.id)
    const srcInner = groupById(inner.id)

    expect(pastedLeafB.canvasX - pastedLeafA.canvasX).toBeCloseTo(srcLeafB.canvasX - srcLeafA.canvasX)
    expect(pastedLeafC.canvasX - pastedLeafA.canvasX).toBeCloseTo(srcLeafC.canvasX - srcLeafA.canvasX)
    expect(pastedOuter.canvasX - pastedLeafA.canvasX).toBeCloseTo(srcOuter.canvasX - srcLeafA.canvasX)
    expect(pastedInner.canvasX - pastedLeafA.canvasX).toBeCloseTo(srcInner.canvasX - srcLeafA.canvasX)
    expect(pastedOuter.canvasY - pastedLeafA.canvasY).toBeCloseTo(srcOuter.canvasY - srcLeafA.canvasY)
  })

  it('a group copied without its parent pastes unparented (parent outside the copy is not rejoined)', async () => {
    const child = createTextEntity({ canvasX: 0, canvasY: 0, text: 'child' })
    await settleSync()
    const inner = createUserGroup([child.id], 'Solo group')
    await settleSync()
    const outer = createUserGroup([inner.id], 'Parent group not copied')
    await settleSync()

    // Select only the inner group — its parent (outer) is not part of the copy.
    selectEntities([inner.id])
    const payload = copyableSelectionPayload()
    expect(payload!.entities).toHaveLength(2) // inner group + child

    const { entityIds } = pasteEntitiesFromClipboard({ payload: payload!, canvasX: 4000, canvasY: 4000 })
    expect(entityIds).toHaveLength(2)

    const pastedInner = getEntityKind('group')
      .entities()
      .find((group) => group.id !== inner.id && group.id !== outer.id) as WorkspaceGroup
    expect(pastedInner, 'inner group was not cloned').toBeTruthy()
    expect(pastedInner.parentGroupId, "clone should not rejoin the source's outer group").toBeUndefined()
    expect(pastedInner.parentGroupId).not.toBe(outer.id)
  })
})
