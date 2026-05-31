/**
 * Selection distribute smoke tests — ADR 0015 D7 (Phase B).
 *
 * Covers `distributeSelection`: the position-only on-ramp to a reorderable row.
 * Takes 3+ loosely-spaced entities and evens edge-to-edge gaps, keeping the
 * first and last fixed. Collapses to one Y.Doc transaction / one undo step;
 * persists nothing but the new positions (no entityOrder write, no managedLayout
 * group on disk). The output is reorder-eligible by construction.
 *
 * Mutation-verified by:
 *   - returning false unconditionally from `distributeSelection`: every
 *     `changed === true` and gap-equality assertion fails.
 *   - writing cross-axis instead of on-axis position: box positions collapse and
 *     the gap-equality assertion fails.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  createTextEntities,
  deleteTextEntities,
  distributeSelection,
  flushWorkspaceAutosave,
  getDiskSnapshot,
  getEntityOrder,
  getTextEntities,
  resetSmokeState,
  undoWorkspace,
} from './app-client'
import { detectReorderableRow } from '../../src/shared/reorder-row'
import { observeYDocTransactions, wait } from './test-utils'

const WIDTH = 200
const HEIGHT = 100

async function cleanupTextEntities(): Promise<void> {
  const { textEntities } = await getTextEntities()
  if (textEntities.length) {
    await deleteTextEntities(textEntities.map((t) => t.id))
    await wait(50)
  }
}

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

async function positionsById(): Promise<Record<string, { x: number; y: number; width: number; height: number }>> {
  const { textEntities } = await getTextEntities()
  return Object.fromEntries(
    textEntities.map((t) => [t.id, { x: t.canvasX, y: t.canvasY, width: t.width, height: t.height }]),
  )
}

// A, B, C unevenly spaced — gaps differ so reorder dots don't show yet.
async function makeUnevenRow() {
  const a = await createText({ canvasX: 100, canvasY: 200, text: 'A', width: WIDTH, height: HEIGHT })
  // Gap1 = 50, Gap2 = 200 — deliberately uneven.
  const b = await createText({ canvasX: 100 + WIDTH + 50, canvasY: 200, text: 'B', width: WIDTH, height: HEIGHT })
  const c = await createText({ canvasX: 100 + 2 * WIDTH + 50 + 200, canvasY: 200, text: 'C', width: WIDTH, height: HEIGHT })
  await wait(50)
  return { a, b, c }
}

describe('distribute selection (position-only commit)', () => {
  beforeEach(async () => {
    await resetSmokeState()
    await cleanupTextEntities()
  })

  afterEach(async () => {
    await cleanupTextEntities()
  })

  it('evens gaps in one transaction; one undo restores; nothing but positions persists', async () => {
    const { a, b, c } = await makeUnevenRow()
    const before = await positionsById()
    const orderBefore = (await getEntityOrder()).entityOrder

    // Baseline group nodes — distribute must add none.
    await flushWorkspaceAutosave()
    const groupsBefore = ((await getDiskSnapshot()).doc?.nodes ?? [])
      .filter((n) => n.type === 'group')
      .map((n) => n.id)
      .sort()

    // One mutation → one Y.Doc transaction.
    const count = await observeYDocTransactions(async () => {
      const { changed } = await distributeSelection({ entityIds: [a, b, c] })
      expect(changed).toBe(true)
    })
    expect(count).toBe(1)
    await wait(50)

    const after = await positionsById()

    // Endpoints fixed.
    expect(after[a].x).toBe(before[a].x)
    expect(after[c].x + after[c].width).toBe(before[c].x + before[c].width)

    // Gaps are now equal edge-to-edge.
    const gap1 = after[b].x - (after[a].x + after[a].width)
    const gap2 = after[c].x - (after[b].x + after[b].width)
    expect(Math.abs(gap1 - gap2)).toBeLessThanOrEqual(1)

    // Cross-axis (y) preserved.
    expect(after[a].y).toBe(before[a].y)
    expect(after[b].y).toBe(before[b].y)
    expect(after[c].y).toBe(before[c].y)

    // entityOrder untouched.
    expect((await getEntityOrder()).entityOrder).toEqual(orderBefore)

    // No managedLayout group added.
    await flushWorkspaceAutosave()
    const snap = await getDiskSnapshot()
    const groupsAfter = (snap.doc?.nodes ?? [])
      .filter((n) => n.type === 'group')
      .map((n) => n.id)
      .sort()
    expect(groupsAfter).toEqual(groupsBefore)

    // Output is reorder-eligible (the through-line to detectReorderableRow).
    const boxes = [a, b, c].map((id) => ({
      id,
      x: after[id].x,
      y: after[id].y,
      width: after[id].width,
      height: after[id].height,
    }))
    expect(detectReorderableRow(boxes)).not.toBeNull()

    // A single undo restores every position.
    await undoWorkspace()
    await wait(50)
    const undone = await positionsById()
    expect(undone[a].x).toBe(before[a].x)
    expect(undone[b].x).toBe(before[b].x)
    expect(undone[c].x).toBe(before[c].x)
  })

  it('is a no-op on an already-even selection (no undo step)', async () => {
    // Equal-gap row — gaps are identical, nothing to move.
    const GAP = 80
    const a = await createText({ canvasX: 100, canvasY: 200, text: 'A', width: WIDTH, height: HEIGHT })
    const b = await createText({ canvasX: 100 + WIDTH + GAP, canvasY: 200, text: 'B', width: WIDTH, height: HEIGHT })
    const c = await createText({ canvasX: 100 + 2 * (WIDTH + GAP), canvasY: 200, text: 'C', width: WIDTH, height: HEIGHT })
    await wait(50)
    const before = await positionsById()

    const { changed } = await distributeSelection({ entityIds: [a, b, c] })
    expect(changed).toBe(false)
    await wait(50)

    const after = await positionsById()
    expect(after[a].x).toBe(before[a].x)
    expect(after[b].x).toBe(before[b].x)
    expect(after[c].x).toBe(before[c].x)
  })

  it('is a no-op for fewer than 3 entities', async () => {
    const a = await createText({ canvasX: 100, canvasY: 200, text: 'A', width: WIDTH, height: HEIGHT })
    const b = await createText({ canvasX: 400, canvasY: 200, text: 'B', width: WIDTH, height: HEIGHT })
    await wait(50)

    const { changed } = await distributeSelection({ entityIds: [a, b] })
    expect(changed).toBe(false)
  })
})
