/**
 * Undo/redo round-trips against the real runtime, in-process.
 *
 * Drives the same mutators the IPC handlers call (document-commands) and the
 * same undo stack the Cmd+Z binding calls (space-undo), then asserts on
 * runtime arrays and the Y.Doc.
 *
 * Mutation-verified by:
 *   - removing the `setActiveUndoManager(manager)` call inside
 *     `createCanvasUndoManager()` — undo becomes a no-op and every case fails.
 *   - bypassing `requestDocSync()` inside `syncRuntimeToDoc` — "undo of entity
 *     creation removes it" fails because the create never reached the Y.Doc.
 */

import type * as Y from 'yjs'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createTextEntity,
  deleteTextEntity,
  getTextEntities,
  reportContentHeight,
  updateTextEntity,
} from '../../src/main/runtime/document-commands'
import { beginGestureSession } from '../../src/main/runtime/space-gesture-session'
import { undo, redo, canUndo, canRedo } from '../../src/main/runtime/space-undo'
import { DOC_MAP_ENTITIES } from '../../src/main/runtime/space-doc'

let harness: WorkspaceHarness

describe('undo', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('undo of entity creation removes it from runtime and Y.Doc', async () => {
    const entity = createTextEntity({ canvasX: 100, canvasY: 100, text: 'undo me' })
    await settleSync()

    const entitiesMap = harness.doc.getMap(DOC_MAP_ENTITIES)
    expect(entitiesMap.has(entity.id)).toBe(true)
    expect(canUndo()).toBe(true)

    undo()
    expect(getTextEntities().some((t) => t.id === entity.id)).toBe(false)
    expect(entitiesMap.has(entity.id)).toBe(false)
  })

  it('redo replays the creation', async () => {
    const entity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'redo me' })
    await settleSync()

    undo()
    expect(getTextEntities()).toHaveLength(0)
    expect(canRedo()).toBe(true)

    redo()
    expect(getTextEntities().some((t) => t.id === entity.id)).toBe(true)
  })

  it('redo stack clears when a new mutation lands after undo', async () => {
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'first' })
    await settleSync()
    undo()
    expect(canRedo()).toBe(true)

    createTextEntity({ canvasX: 50, canvasY: 50, text: 'second' })
    await settleSync()
    expect(canRedo()).toBe(false)
  })

  it('undo of deletion restores the entity', async () => {
    const entity = createTextEntity({ canvasX: 10, canvasY: 10, text: 'delete me' })
    await settleSync()

    deleteTextEntity(entity.id)
    await settleSync()
    expect(getTextEntities()).toHaveLength(0)

    undo()
    expect(getTextEntities().some((t) => t.id === entity.id)).toBe(true)
  })

  it('a measured content height persists without costing an undo step', async () => {
    const entity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'sticky' })
    await settleSync()

    // The measurement trails the mutation that caused it, so it arrives after
    // the gesture closed. It must not become the step the next undo spends.
    reportContentHeight(entity.id, 340)
    await settleSync()
    expect(getTextEntities()[0].height).toBe(340)
    const docEntity = harness.doc.getMap(DOC_MAP_ENTITIES).get(entity.id) as Y.Map<unknown>
    expect(docEntity.get('height')).toBe(340)

    undo()
    expect(getTextEntities()).toHaveLength(0)
  })

  it('a measured height inside a gesture rides that gesture and reverts with it', async () => {
    const entity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'sticky' })
    await settleSync()

    const session = beginGestureSession()
    updateTextEntity(entity.id, { width: 400 })
    reportContentHeight(entity.id, 120)
    session.finalize()
    await settleSync()

    undo() // the resize reverts as one step — width and the height it produced
    const reverted = getTextEntities()[0]
    expect(reverted.width).toBe(200)
    expect(reverted.height).toBe(200)
  })
})
