/**
 * Stack-order mutations against the real runtime, in-process.
 *
 * Guards `reorderStackOrderIds` — the function the /stack-order/* HTTP routes
 * call — through the four single-id mutations, block moves of an ids array,
 * group contiguity normalization, and the undo contract (one reorder = one
 * undo step that restores the prior order without swallowing entity creation).
 *
 * Mutation-verified by: dropping the `enforceGroupContiguity` wrapper in
 * `reorderStackOrderIds` (src/main/runtime/entity-order-state.ts) fails the
 * contiguity case; the same edit inverted (skipping `writeEntityOrder`) fails
 * every reorder assertion.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createTextEntity, getTextEntities } from '../../src/main/runtime/document-commands'
import { createUserGroup } from '../../src/main/workspace-groups'
import {
  currentEntityOrder,
  reorderStackOrderIds,
} from '../../src/main/runtime/entity-order-state'
import { undo } from '../../src/main/runtime/workspace-undo'

let harness: WorkspaceHarness

function idsInOrder(order: string[], ids: string[]): string[] {
  const wanted = new Set(ids)
  return order.filter((id) => wanted.has(id))
}

async function createTexts(count: number): Promise<string[]> {
  const ids = Array.from({ length: count }, (_, i) =>
    createTextEntity({ canvasX: i * 240, canvasY: 200, text: `stack ${i}` }).id,
  )
  await settleSync()
  return ids
}

describe('stack order', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('drives the four stack-order mutations for a single id', async () => {
    const [first, second, third] = await createTexts(3)
    const ids = [first, second, third]

    expect(idsInOrder(currentEntityOrder(), ids)).toEqual([first, second, third])

    expect(reorderStackOrderIds('send-backward', [third])).toBe(true)
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual([first, third, second])

    expect(reorderStackOrderIds('bring-forward', [third])).toBe(true)
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual([first, second, third])

    expect(reorderStackOrderIds('send-to-back', [third])).toBe(true)
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual([third, first, second])

    expect(reorderStackOrderIds('bring-to-front', [third])).toBe(true)
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual([first, second, third])
  })

  it('moves an ids array as one block', async () => {
    const [first, second, third, fourth] = await createTexts(4)
    const ids = [first, second, third, fourth]

    expect(reorderStackOrderIds('send-backward', [second, third])).toBe(true)
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual([second, third, first, fourth])
  })

  it('preserves group contiguity when a member is reordered', async () => {
    const [first, second, third] = await createTexts(3)
    const group = createUserGroup([first, second], 'Stack group')
    await settleSync()
    const ids = [first, second, group.id, third]

    expect(reorderStackOrderIds('send-to-back', [first])).toBe(true)

    const ordered = idsInOrder(currentEntityOrder(), ids)
    const groupRun = ordered.filter((id) => id === first || id === second || id === group.id)
    const runStart = ordered.indexOf(groupRun[0])
    expect(ordered.slice(runStart, runStart + groupRun.length)).toEqual(groupRun)
    expect(groupRun[groupRun.length - 1]).toBe(group.id)
  })

  it('a stack-order mutation is a single undo step restoring prior order', async () => {
    const [first, second, third] = await createTexts(3)
    const ids = [first, second, third]
    const before = idsInOrder(currentEntityOrder(), ids)

    expect(reorderStackOrderIds('bring-to-front', [first])).toBe(true)
    await settleSync()
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual([second, third, first])

    undo()
    expect(idsInOrder(currentEntityOrder(), ids)).toEqual(before)
    // One step: the undo only reverted the reorder, not the creations.
    expect(getTextEntities().map((t) => t.id).sort()).toEqual([...ids].sort())
  })
})
