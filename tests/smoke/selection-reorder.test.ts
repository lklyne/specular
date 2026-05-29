/**
 * Selection reorder smoke tests — ADR 0015 D7 (Phase B, the commit path).
 *
 * Covers `reorderSelection`: the position-only sibling of `reorderManagedChild`.
 * Geometry is the source of truth — reordering a loose equal-gap selection
 * permutes positions, collapses to one Y.Doc transaction / one undo step, and
 * persists *nothing but the new positions* (no `entityOrder` write, no
 * `managedLayout` group on disk).
 *
 * Mutation-verified by:
 *   - making `reorderSelection` return false without writing (a no-op commit):
 *     the `changed === true` and position-permutation assertions fail.
 *   - swapping the written x/y in `writeReorderedPosition` (canvasX ← pos.y):
 *     every box collapses to the same x and `after[b].x < after[c].x` fails
 *     ("expected 200 to be less than 200").
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  createTextEntities,
  deleteTextEntities,
  flushWorkspaceAutosave,
  getDiskSnapshot,
  getEntityOrder,
  getTextEntities,
  reorderSelection,
  resetSmokeState,
  undoWorkspace,
} from './app-client'
import { observeYDocTransactions, wait } from './test-utils'

const WIDTH = 200
const GAP = 80

async function cleanupTextEntities(): Promise<void> {
  const { textEntities } = await getTextEntities()
  if (textEntities.length) {
    await deleteTextEntities(textEntities.map((t) => t.id))
    await wait(50)
  }
}

// Single creates are synchronous; the multi-item route staggers creation.
async function createText(input: {
  canvasX: number
  canvasY: number
  text: string
  width: number
  height: number
}): Promise<string> {
  const { ids } = await createTextEntities([input])
  return ids[0]
}

async function positionsById(): Promise<Record<string, { x: number; y: number }>> {
  const { textEntities } = await getTextEntities()
  return Object.fromEntries(textEntities.map((t) => [t.id, { x: t.canvasX, y: t.canvasY }]))
}

// A, B, C in an exact equal-gap horizontal row (gap = GAP between every pair).
async function makeEqualGapRow() {
  const a = await createText({ canvasX: 100, canvasY: 200, text: 'A', width: WIDTH, height: 100 })
  const b = await createText({ canvasX: 100 + WIDTH + GAP, canvasY: 200, text: 'B', width: WIDTH, height: 100 })
  const c = await createText({ canvasX: 100 + 2 * (WIDTH + GAP), canvasY: 200, text: 'C', width: WIDTH, height: 100 })
  await wait(50)
  return { a, b, c }
}

describe('selection reorder (position-only commit)', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupTextEntities()
  })

  afterEach(async () => {
    await cleanupTextEntities()
  })

  it('permutes positions in one transaction; one undo restores them; nothing but positions persists', async () => {
    const { a, b, c } = await makeEqualGapRow()
    const before = await positionsById()
    const orderBefore = (await getEntityOrder()).entityOrder
    expect(before[a].x).toBeLessThan(before[b].x)
    expect(before[b].x).toBeLessThan(before[c].x)

    // Baseline group nodes on disk (the shared smoke workspace may already hold
    // groups from other suites). reorderSelection must add none.
    await flushWorkspaceAutosave()
    const groupsBefore = ((await getDiskSnapshot()).doc?.nodes ?? [])
      .filter((n) => n.type === 'group')
      .map((n) => n.id)
      .sort()

    // Move A from the front to the end. One mutation → one Y.Doc transaction.
    const count = await observeYDocTransactions(async () => {
      const { changed } = await reorderSelection({ orderedIds: [a, b, c], movingId: a, dropIndex: 2 })
      expect(changed).toBe(true)
    })
    expect(count).toBe(1)
    await wait(50)

    const after = await positionsById()
    // Sequence is now B, C, A — packed from the same origin by the same gap.
    expect(after[b].x).toBeLessThan(after[c].x)
    expect(after[c].x).toBeLessThan(after[a].x)
    expect(after[c].x).toBe(after[b].x + WIDTH + GAP)
    expect(after[a].x).toBe(after[c].x + WIDTH + GAP)
    // Cross-axis is preserved (Q1): each box keeps its own y.
    expect(after[a].y).toBe(before[a].y)
    expect(after[b].y).toBe(before[b].y)
    expect(after[c].y).toBe(before[c].y)

    // entityOrder is untouched — geometry is the only truth here.
    expect((await getEntityOrder()).entityOrder).toEqual(orderBefore)

    // Nothing persists but the new positions: no managedLayout group is added.
    await flushWorkspaceAutosave()
    const snap = await getDiskSnapshot()
    const groupsAfter = (snap.doc?.nodes ?? [])
      .filter((n) => n.type === 'group')
      .map((n) => n.id)
      .sort()
    expect(groupsAfter).toEqual(groupsBefore)
    const diskA = snap.doc?.nodes.find((n) => n.id === a)
    expect(diskA?.x).toBe(after[a].x)

    // A single undo restores every position.
    await undoWorkspace()
    await wait(50)
    const undone = await positionsById()
    expect(undone[a].x).toBe(before[a].x)
    expect(undone[b].x).toBe(before[b].x)
    expect(undone[c].x).toBe(before[c].x)
  })

  it('is a no-op on an unequal-gap selection (no eligible row, no dots)', async () => {
    const a = await createText({ canvasX: 100, canvasY: 200, text: 'A', width: WIDTH, height: 100 })
    const b = await createText({ canvasX: 400, canvasY: 200, text: 'B', width: WIDTH, height: 100 })
    // C sits far past B — the second gap is much larger than the first.
    const c = await createText({ canvasX: 1200, canvasY: 200, text: 'C', width: WIDTH, height: 100 })
    await wait(50)
    const before = await positionsById()

    const { changed } = await reorderSelection({ orderedIds: [a, b, c], movingId: a, dropIndex: 2 })
    expect(changed).toBe(false)
    await wait(50)

    const after = await positionsById()
    expect(after[a].x).toBe(before[a].x)
    expect(after[b].x).toBe(before[b].x)
    expect(after[c].x).toBe(before[c].x)
  })
})
