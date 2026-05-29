/**
 * Auto-layout (managed group) smoke tests — ADR 0015.
 *
 * Covers the Milestone 1 runtime: make-auto-layout from a selection (O1),
 * row reflow, drag-reorder, and the undo/persistence round-trips required by
 * the test contract for new runtime mutators (reflowManagedGroup,
 * reorderManagedChild).
 *
 * Mutation-verified by:
 *   - changing `commitAsOneTransaction` to call `mutate()` then a separate
 *     `requestDocSyncImmediate()` outside the transaction: the reorder
 *     transaction-count assertion goes from 1 to 2.
 *   - dropping the `reflowManagedGroup(groupId)` call inside `reorderManagedChild`:
 *     positions stop following the reordered sequence and the "B,C,A after
 *     reorder" assertion fails.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  createTextEntities,
  deleteTextEntities,
  flushWorkspaceAutosave,
  getDiskSnapshot,
  getEntityOrder,
  getInteractionMode,
  getTextEntities,
  makeAutoLayout,
  reorderGestureCancel,
  reorderGestureCommit,
  reorderGestureMove,
  reorderGestureStart,
  reorderManagedChild,
  resetSmokeState,
  undoWorkspace,
} from './app-client'
import { observeYDocTransactions, wait } from './test-utils'

const GAP = 80

async function cleanupTextEntities(): Promise<void> {
  const { textEntities } = await getTextEntities()
  if (textEntities.length) {
    await deleteTextEntities(textEntities.map((t) => t.id))
    await wait(50)
  }
}

// Create one at a time: the multi-item create route staggers entity creation
// asynchronously, which would race make-auto-layout. Single creates are sync.
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

async function positionsById(): Promise<Record<string, { x: number; y: number; width: number }>> {
  const { textEntities } = await getTextEntities()
  return Object.fromEntries(
    textEntities.map((t) => [t.id, { x: t.canvasX, y: t.canvasY, width: t.width }]),
  )
}

describe('auto-layout managed groups', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupTextEntities()
  })

  afterEach(async () => {
    await cleanupTextEntities()
  })

  it('packs a multi-selection into a row in visual order and persists positions', async () => {
    // Scattered both ways: visual left-to-right is B(100), C(350), A(600).
    const a = await createText({ canvasX: 600, canvasY: 200, text: 'A', width: 200, height: 100 })
    const b = await createText({ canvasX: 100, canvasY: 260, text: 'B', width: 150, height: 100 })
    const c = await createText({ canvasX: 350, canvasY: 220, text: 'C', width: 120, height: 100 })
    const ids = [a, b, c]

    const group = await makeAutoLayout({ entityIds: ids })
    expect(group.managedLayout).toBe(true)
    expect(group.layoutMode).toBe('row')
    await wait(50)

    const pos = await positionsById()
    // All children share the row's top (start-alignment, Milestone 1).
    expect(pos[b].y).toBe(pos[c].y)
    expect(pos[c].y).toBe(pos[a].y)
    // Packed in visual order, each after the previous by width + gap.
    expect(pos[c].x).toBe(pos[b].x + pos[b].width + GAP)
    expect(pos[a].x).toBe(pos[c].x + pos[c].width + GAP)

    // Reflowed positions persist to disk.
    await flushWorkspaceAutosave()
    const snap = await getDiskSnapshot()
    const diskB = snap.doc?.nodes.find((n) => n.id === b)
    expect(diskB?.x).toBe(pos[b].x)
    expect(diskB?.y).toBe(pos[b].y)
  })

  it('reorder is one Y.Doc transaction and one undo step restoring order + positions', async () => {
    const a = await createText({ canvasX: 100, canvasY: 200, text: 'A', width: 200, height: 100 })
    const b = await createText({ canvasX: 400, canvasY: 200, text: 'B', width: 200, height: 100 })
    const c = await createText({ canvasX: 700, canvasY: 200, text: 'C', width: 200, height: 100 })

    const group = await makeAutoLayout({ entityIds: [a, b, c] })
    await wait(50)

    const before = await positionsById()
    const orderBefore = (await getEntityOrder()).entityOrder
    expect(before[a].x).toBeLessThan(before[b].x)
    expect(before[b].x).toBeLessThan(before[c].x)

    // Move A from the front to the end. One mutation → one Y.Doc transaction.
    const count = await observeYDocTransactions(async () => {
      await reorderManagedChild({ groupId: group.id, childId: a, toIndex: 2 })
    })
    expect(count).toBe(1)
    await wait(50)

    const after = await positionsById()
    // Sequence is now B, C, A.
    expect(after[b].x).toBeLessThan(after[c].x)
    expect(after[c].x).toBeLessThan(after[a].x)

    // A single undo restores both the layout sequence and the positions.
    await undoWorkspace()
    await wait(50)
    const undone = await positionsById()
    expect(undone[a].x).toBe(before[a].x)
    expect(undone[b].x).toBe(before[b].x)
    expect(undone[c].x).toBe(before[c].x)
    expect((await getEntityOrder()).entityOrder).toEqual(orderBefore)
  })
})

describe('auto-layout reorder gesture', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupTextEntities()
  })

  afterEach(async () => {
    await cleanupTextEntities()
  })

  async function makeRowOfThree() {
    const a = await createText({ canvasX: 100, canvasY: 200, text: 'A', width: 200, height: 100 })
    const b = await createText({ canvasX: 400, canvasY: 200, text: 'B', width: 200, height: 100 })
    const c = await createText({ canvasX: 700, canvasY: 200, text: 'C', width: 200, height: 100 })
    const group = await makeAutoLayout({ entityIds: [a, b, c] })
    await wait(50)
    return { a, b, c, group }
  }

  it('begin → move → commit reorders and returns to idle', async () => {
    const { a, b, c } = await makeRowOfThree()
    const before = await positionsById()

    const started = await reorderGestureStart({ movingId: a })
    expect(started.ok).toBe(true)
    expect(started.mode.kind).toBe('reordering-row')

    // Drag A's dot far to the right, past C's center.
    await reorderGestureMove(before[c].x + before[c].width + 500)
    const committed = await reorderGestureCommit()
    expect(committed.changed).toBe(true)
    expect(committed.mode.kind).toBe('idle')
    await wait(50)

    const after = await positionsById()
    // Sequence is now B, C, A.
    expect(after[b].x).toBeLessThan(after[c].x)
    expect(after[c].x).toBeLessThan(after[a].x)
  })

  it('begin → cancel leaves order and positions untouched', async () => {
    const { a, b, c } = await makeRowOfThree()
    const before = await positionsById()

    await reorderGestureStart({ movingId: a })
    await reorderGestureMove(before[c].x + 500)
    const cancelled = await reorderGestureCancel('escape')
    expect(cancelled.mode.kind).toBe('idle')
    await wait(50)

    const after = await positionsById()
    expect(after[a].x).toBe(before[a].x)
    expect(after[b].x).toBe(before[b].x)
    expect(after[c].x).toBe(before[c].x)
  })

  it('cancel-on-blur and cancel-on-undo abort without mutation', async () => {
    for (const reason of ['blur', 'undo'] as const) {
      const { a, b, c } = await makeRowOfThree()
      const before = await positionsById()
      await reorderGestureStart({ movingId: a })
      await reorderGestureMove(before[c].x + 500)
      await reorderGestureCancel(reason)
      await wait(20)
      const after = await positionsById()
      expect(after[a].x).toBe(before[a].x)
      expect(after[b].x).toBe(before[b].x)
      expect(after[c].x).toBe(before[c].x)
      expect((await getInteractionMode()).mode.kind).toBe('idle')
      await cleanupTextEntities()
    }
  })
})
