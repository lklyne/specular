/**
 * I3 invariant: a refused concurrent gesture-begin must not corrupt the
 * undo batch (docs/interaction-layer.md §6).
 *
 * Drives the exact sequence the `canvas-multi-resize-begin/-end` IPC
 * handlers run (src/main/ipc/register-canvas-drag-ipc.ts): tryEnter →
 * beginBatch → resizeMultiSelection ticks → endBatch → markUndoBoundary →
 * commitActive. A second begin while the first is active must be refused
 * WITHOUT opening a second batch — otherwise the batch count is mismatched
 * and undo either no-ops or corrupts a prior step.
 *
 * Mutation-verified by moving `beginBatch()` above the refusal check in this
 * test's begin helper (simulating the pre-fix IPC handler): the undo
 * round-trip assertion fails because endBatch leaves a batch open.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createTextEntity,
  getTextEntities,
  resizeMultiSelection,
} from '../../src/main/runtime/document-commands'
import {
  tryEnter,
  commitActive,
  __resetForTests as resetInteraction,
} from '../../src/main/runtime/interaction-controller'
import { beginBatch, endBatch } from '../../src/main/runtime/workspace-observers'
import { undo, canUndo, markUndoBoundary } from '../../src/main/runtime/workspace-undo'

let harness: WorkspaceHarness

/** The canvas-multi-resize-begin IPC handler body, verbatim. */
function beginMultiResize(): { refused: boolean } {
  const token = tryEnter({ kind: 'resizing-multi-selection' })
  if ('refused' in token) return { refused: true }
  beginBatch()
  return { refused: false }
}

/** The canvas-multi-resize-end IPC handler body, verbatim. */
function endMultiResize(): void {
  endBatch()
  markUndoBoundary()
  commitActive()
}

describe('I3: conflicting gesture-begin does not corrupt batch/undo', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    resetInteraction()
  })

  afterAll(() => harness?.dispose())

  it('second concurrent multi-resize-begin is refused and does not open a dangling batch', async () => {
    const entity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'resize-me', width: 100, height: 50 })
    await settleSync()
    const initialWidth = getTextEntities().find((t) => t.id === entity.id)!.width

    expect(beginMultiResize().refused).toBe(false)
    // A second begin while the first is active must be refused.
    expect(beginMultiResize().refused).toBe(true)

    resizeMultiSelection([
      {
        id: entity.id,
        kind: 'text',
        width: 150,
        height: 50,
        canvasX: entity.canvasX,
        canvasY: entity.canvasY,
      },
    ])
    endMultiResize()
    await settleSync()

    expect(getTextEntities().find((t) => t.id === entity.id)!.width).toBe(150)

    // Undo must round-trip cleanly — a mismatched batch count would make
    // this either a no-op or corrupt the creation step below it.
    expect(canUndo()).toBe(true)
    undo()
    expect(getTextEntities().find((t) => t.id === entity.id)!.width).toBe(initialWidth)
  })
})
