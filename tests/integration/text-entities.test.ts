/**
 * Text entity CRUD + lifecycle against the real runtime, in-process.
 *
 * Drives the same mutators the IPC handlers and routes call
 * (document-commands) and asserts on the three production surfaces: runtime
 * arrays, the Y.Doc, and the .canvas file on disk. Guards create/update/
 * delete behavior, disk persistence of a created text entity, and the
 * undo/redo round-trip of a creation.
 *
 * Mutation-verified by:
 *   - passing `{}` instead of the snapped patch to `updateTextEntityInState`
 *     in `updateTextEntity` (src/main/runtime/document-commands.ts) —
 *     "updates a text entity" fails.
 *   - removing the `scheduleWorkspaceAutosave()` call from `createTextEntity`
 *     — "round-trips a created text entity through undo/redo" fails because
 *     the create never syncs to the Y.Doc, so there is nothing to undo.
 *     (The disk case survives that mutation: `diskDoc()`'s flush still
 *     syncs-and-writes on its own.)
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import {
  createTextEntity,
  deleteTextEntity,
  getTextEntities,
  updateTextEntity,
} from '../../src/main/runtime/document-commands'
import { undo, redo } from '../../src/main/runtime/workspace-undo'
import { DOC_MAP_ENTITIES } from '../../src/main/runtime/workspace-doc'

let harness: WorkspaceHarness

describe('text entities', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('creates a text entity', () => {
    const entity = createTextEntity({ canvasX: 200, canvasY: 200, text: 'Integration note' })

    const match = getTextEntities().find((e) => e.id === entity.id)
    expect(match).toBeDefined()
    expect(match!.text).toBe('Integration note')
    expect(match!.canvasX).toBe(200)
    expect(match!.canvasY).toBe(200)
  })

  it('updates a text entity', () => {
    const entity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'before' })

    const updated = updateTextEntity(entity.id, { text: 'Updated note' })
    expect(updated).not.toBeNull()

    const match = getTextEntities().find((e) => e.id === entity.id)
    expect(match!.text).toBe('Updated note')
  })

  it('deletes a text entity', () => {
    const entity = createTextEntity({ canvasX: 0, canvasY: 0, text: 'doomed' })

    expect(deleteTextEntity(entity.id)).toBe(true)
    expect(getTextEntities().find((e) => e.id === entity.id)).toBeUndefined()
  })

  it('persists a created text entity to disk', async () => {
    const entity = createTextEntity({ canvasX: 320, canvasY: 320, text: 'persisted text' })
    await settleSync()

    const disk = harness.diskDoc()
    expect(disk).not.toBeNull()
    const node = disk!.nodes.find((n) => n.id === entity.id) as
      | { type: string; text: string; x: number; y: number }
      | undefined
    expect(node).toBeDefined()
    expect(node!.type).toBe('text')
    expect(node!.text).toBe('persisted text')
    expect(node!.x).toBe(320)
    expect(node!.y).toBe(320)
  })

  it('round-trips a created text entity through undo/redo', async () => {
    const entity = createTextEntity({ canvasX: 360, canvasY: 360, text: 'undoable text' })
    await settleSync()

    const entitiesMap = harness.doc.getMap(DOC_MAP_ENTITIES)
    expect(entitiesMap.has(entity.id)).toBe(true)

    undo()
    expect(getTextEntities().some((e) => e.id === entity.id)).toBe(false)
    expect(entitiesMap.has(entity.id)).toBe(false)

    redo()
    expect(getTextEntities().some((e) => e.id === entity.id)).toBe(true)
    expect(entitiesMap.has(entity.id)).toBe(true)
  })
})
