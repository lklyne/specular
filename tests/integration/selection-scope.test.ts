/**
 * `resolveSelectionScope` (ADR 0034) against the real runtime, in-process.
 *
 * The reported bug: a multi-selection containing a group plus ungrouped
 * items did not drag as a unit when grabbed by a member (descendant) of the
 * selected group — every branch of the old `selectedDragEntityIds` missed
 * "anchor is a descendant of a selected group inside a multi-selection" and
 * fell through to `[entityId]`, dragging one item out of the selection.
 *
 * Mutation-verified by reverting `resolveSelectionScope`'s anchor rule to the
 * old `selectedDragEntityIds` branch pile (drop the `currentOperandIds`
 * membership check and only handle `selectedIds.includes(entityId)` /
 * `activeGroupId` single-selection cases) and confirming
 * "anchor on a group descendant inside a mixed selection resolves to the
 * whole selection" and "dragging via that anchor moves every operand" both
 * fail.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createTextEntity,
  getTextEntities,
  applyDragDelta,
  resizeMultiSelection,
} from '../../src/main/runtime/document-commands'
import { workspaceGroups } from '../../src/main/runtime/space-model'
import { undo, markUndoBoundary } from '../../src/main/runtime/space-undo'
import { createUserGroup } from '../../src/main/workspace-groups'
import { selectEntities, selectNone } from '../../src/main/runtime/selection-controller'
import { resolveSelectionScope } from '../../src/main/runtime/selection-scope'
import { reanchorEntityById } from '../../src/main/runtime/page-anchor-state'
import { applyCanvasPatch } from '../../src/main/canvas-apply'

let harness: WorkspaceHarness

async function buildMixedSelection() {
  // A group with two text members, plus one ungrouped text entity — the
  // exact shape of the reported bug (group + ungrouped items selected
  // together).
  const memberA = createTextEntity({ canvasX: 0, canvasY: 0, text: 'member-a' })
  const memberB = createTextEntity({ canvasX: 100, canvasY: 0, text: 'member-b' })
  const outsider = createTextEntity({ canvasX: 400, canvasY: 0, text: 'outsider' })
  await settleSync()
  const group = createUserGroup([memberA.id, memberB.id], 'Mixed group')
  await settleSync()
  selectEntities([group.id, outsider.id])
  return { group, memberA, memberB, outsider }
}

describe('resolveSelectionScope', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('resolves a mixed selection: memberIds unexpanded, operandIds expanded, bounds unioned', async () => {
    const { group, memberA, memberB, outsider } = await buildMixedSelection()

    const scope = resolveSelectionScope()
    expect(scope.memberIds.slice().sort()).toEqual([group.id, outsider.id].sort())
    expect(scope.operandIds.slice().sort()).toEqual(
      [group.id, memberA.id, memberB.id, outsider.id].sort(),
    )
    expect(scope.bounds).not.toBeNull()
    // Union spans from the group's members (x=0) to the outsider's right edge.
    expect(scope.bounds!.x).toBeLessThanOrEqual(0)
    expect(scope.bounds!.x + scope.bounds!.width).toBeGreaterThanOrEqual(
      outsider.canvasX + outsider.width,
    )
  })

  it('anchor on a descendant of a selected group resolves to the whole selection', async () => {
    const { group, memberA, memberB, outsider } = await buildMixedSelection()

    // memberA is not itself a member of the selection (only the group and
    // the outsider are) — it's reachable only as a descendant of the
    // selected group. Pressing it must still resolve to the full selection.
    const scope = resolveSelectionScope(memberA.id)
    expect(scope.memberIds.slice().sort()).toEqual([group.id, outsider.id].sort())
    expect(scope.operandIds.slice().sort()).toEqual(
      [group.id, memberA.id, memberB.id, outsider.id].sort(),
    )
  })

  it('anchor outside the selection resolves to that item alone', async () => {
    const { outsider } = await buildMixedSelection()
    const lonely = createTextEntity({ canvasX: 900, canvasY: 0, text: 'lonely' })
    await settleSync()

    const scope = resolveSelectionScope(lonely.id)
    expect(scope.memberIds).toEqual([lonely.id])
    expect(scope.operandIds).toEqual([lonely.id])
    expect(outsider.id).not.toBe(lonely.id)
  })

  it('anchor on a page carries its page-anchored entity into operandIds but not memberIds', async () => {
    const result = applyCanvasPatch({
      entities: [{ kind: 'page', url: 'https://example.com', canvasX: 0, canvasY: 0, presetIndex: 0 }],
    })
    const pageId = result.created[0]
    await settleSync()
    // A sticky positioned inside the page's body anchors to it on the next
    // reanchor pass (placement-derived — see shared/page-anchor.ts).
    const sticky = createTextEntity({ canvasX: 50, canvasY: 50, text: 'anchored' })
    await settleSync()
    reanchorEntityById(sticky.id)
    expect(getTextEntities().find((t) => t.id === sticky.id)?.pageAnchor?.pageId).toBe(pageId)

    const scope = resolveSelectionScope(pageId)
    expect(scope.memberIds).toEqual([pageId])
    expect(scope.operandIds.slice().sort()).toEqual([pageId, sticky.id].sort())
  })

  it('a drag over resolveSelectionScope operandIds moves every operand, including the group descendant the old logic missed', async () => {
    const { group, memberA, memberB, outsider } = await buildMixedSelection()

    const beforeA = { x: memberA.canvasX, y: memberA.canvasY }
    const beforeB = { x: memberB.canvasX, y: memberB.canvasY }
    const beforeOutsider = { x: outsider.canvasX, y: outsider.canvasY }

    // Press on memberA — a descendant of the selected group, not a direct
    // member of the selection. The old `selectedDragEntityIds` fell through
    // all three of its branches for this case and resolved to `[memberA.id]`
    // alone, so only memberA would move.
    const scope = resolveSelectionScope(memberA.id)
    applyDragDelta(scope.operandIds, 40, 25)
    await settleSync()

    const afterA = getTextEntities().find((t) => t.id === memberA.id)!
    const afterB = getTextEntities().find((t) => t.id === memberB.id)!
    const afterOutsider = getTextEntities().find((t) => t.id === outsider.id)!

    expect(afterA.canvasX).not.toBe(beforeA.x)
    expect(afterA.canvasY).not.toBe(beforeA.y)
    expect(afterB.canvasX).not.toBe(beforeB.x)
    expect(afterB.canvasY).not.toBe(beforeB.y)
    expect(afterOutsider.canvasX).not.toBe(beforeOutsider.x)
    expect(afterOutsider.canvasY).not.toBe(beforeOutsider.y)
    expect(group.id).toBeTruthy()
  })

  it('multi-resize writes the group rect exactly, without re-shifting its descendants', async () => {
    const { group, memberA, memberB } = await buildMixedSelection()
    const before = {
      group: { x: group.canvasX, y: group.canvasY, w: group.width, h: group.height },
      a: { x: memberA.canvasX, y: memberA.canvasY, w: memberA.width, h: memberA.height },
    }
    markUndoBoundary()

    // Halve everything around the bbox origin — the entry set a real gesture
    // produces: the group rides along as one more proportionally-scaled rect,
    // and its descendants are their own entries. If the group write went
    // through the carry-children-on-move path, memberA/memberB would shift
    // twice and land off these exact coordinates.
    const scale = (v: number, origin: number) => origin + (v - origin) / 2
    const entries = [
      {
        id: group.id,
        kind: 'group' as const,
        canvasX: scale(group.canvasX, 0),
        canvasY: scale(group.canvasY, 0),
        width: group.width / 2,
        height: group.height / 2,
      },
      ...[memberA, memberB].map((m) => ({
        id: m.id,
        kind: 'text' as const,
        canvasX: scale(m.canvasX, 0),
        canvasY: scale(m.canvasY, 0),
        width: m.width / 2,
        height: m.height / 2,
      })),
    ]
    resizeMultiSelection(entries)
    await settleSync()

    const groupAfter = workspaceGroups.find((g) => g.id === group.id)!
    expect(groupAfter.width).toBe(before.group.w / 2)
    expect(groupAfter.height).toBe(before.group.h / 2)
    expect(groupAfter.canvasX).toBe(scale(before.group.x, 0))
    const aAfter = getTextEntities().find((t) => t.id === memberA.id)!
    expect(aAfter.canvasX).toBe(scale(before.a.x, 0))
    expect(aAfter.width).toBe(before.a.w / 2)

    // Reverse sync: undo restores both the group rect and the descendants.
    markUndoBoundary()
    undo()
    await settleSync()
    const groupUndone = workspaceGroups.find((g) => g.id === group.id)!
    expect(groupUndone.width).toBe(before.group.w)
    expect(groupUndone.canvasX).toBe(before.group.x)
    const aUndone = getTextEntities().find((t) => t.id === memberA.id)!
    expect(aUndone.canvasX).toBe(before.a.x)
    expect(aUndone.width).toBe(before.a.w)
  })
})
