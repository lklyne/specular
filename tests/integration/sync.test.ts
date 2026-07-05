/**
 * Forward/reverse sync integration tests.
 *
 * Guards the diff-sync pipeline described in src/main/runtime/CLAUDE.md:
 *   mutation → runtime arrays → requestDocSync() → syncRuntimeToDoc
 *   undo     → afterTransaction → syncDocToRuntime (must NOT echo)
 *
 * Transactions are counted directly on the harness Y.Doc via
 * `afterTransaction`, so a forward-sync echo shows up as an extra count.
 *
 * Mutation-verified by:
 *   - commenting out `scheduleWorkspaceAutosave()` in `createTextEntity`
 *     (src/main/runtime/document-commands.ts) — the forward sync never fires,
 *     so both create-count assertions fail (0 transactions instead of 1/2+).
 *   - commenting out the `syncDocToRuntime(doc)` call in the afterTransaction
 *     observer (src/main/runtime/workspace-observers.ts) — "undo does not
 *     re-trigger a forward sync" fails because the runtime still holds the
 *     undone entity.
 * (The smoke suite's mutations — dropping the `'user'` origin or the
 * undo-path `withSuppressedDocSync()` — are not observable in the
 * display-free harness: nothing schedules an autosave during undo here.)
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createTextEntity,
  getTextEntities,
} from '../../src/main/runtime/document-commands'
import { undo } from '../../src/main/runtime/workspace-undo'

let harness: WorkspaceHarness

/**
 * Count Y.Doc afterTransaction events during `fn`, including the
 * microtask-scheduled forward sync it triggers. A single user mutation must
 * produce exactly one transaction; more implies an echo loop.
 */
async function observeTransactions(fn: () => void | Promise<void>): Promise<number> {
  let count = 0
  const handler = () => {
    count += 1
  }
  harness.doc.on('afterTransaction', handler)
  try {
    await fn()
    await settleSync()
  } finally {
    harness.doc.off('afterTransaction', handler)
  }
  return count
}

describe('forward/reverse sync', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('a single text-entity create produces exactly one Y.Doc transaction', async () => {
    const count = await observeTransactions(() => {
      createTextEntity({ canvasX: 0, canvasY: 0, text: 'one tx' })
    })
    // Expect exactly 1: the forward-sync transact wrapped in 'user' origin.
    // Two means a forward-sync echo or duplicate scheduling.
    expect(count).toBe(1)
  })

  it('undo does not re-trigger a forward sync (no echo)', async () => {
    const entity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'echo me' })
    await settleSync()

    const count = await observeTransactions(() => {
      undo()
    })
    // Undo applies a Y.Doc transaction via UndoManager (origin = undoManager).
    // The undo observer's reverse sync must run inside withSuppressedDocSync,
    // so it must NOT add a second 'user' transaction.
    expect(count).toBe(1)
    // Runtime reflects the undo (entity gone).
    expect(getTextEntities().some((t) => t.id === entity.id)).toBe(false)
  })

  it('mutation count stays bounded across rapid mutations', async () => {
    // Catches a runaway feedback loop: if forward sync were re-triggering
    // itself (e.g. by writing inside an afterTransaction handler with no
    // suppression), the count for two creates would balloon.
    const count = await observeTransactions(async () => {
      createTextEntity({ canvasX: 0, canvasY: 0, text: 'a' })
      await settleSync()
      createTextEntity({ canvasX: 100, canvasY: 0, text: 'b' })
    })
    // Strict upper bound: forward sync (1) per mutation + possibly one
    // workspace-metadata sync when the doc tab/runtime tab disagree. More
    // than that means an echo. Lower bound: at minimum each mutation
    // produced one transaction.
    expect(count).toBeGreaterThanOrEqual(2)
    expect(count).toBeLessThanOrEqual(3)
  })
})
