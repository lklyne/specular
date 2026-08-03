/**
 * Forward-sync clear-field unit tests.
 *
 * Exercises `syncRuntimeToDoc()` from src/main/runtime/space-doc.ts against
 * a raw Y.Doc — no Electron — to assert that clearing a runtime field to
 * `undefined` (e.g. `parentGroupId` after ungrouping) deletes the stale key
 * from the Y.Doc entity map instead of leaving it behind, and that the
 * deletion undoes/redoes cleanly via the shared UndoManager.
 *
 * Mutation-verified by reverting the `syncMapFromArray` fix (dropping the
 * `existing.delete(k)` branch back to a bare `continue`) — the "stale key
 * survives clear" assertion fails.
 */

import * as Y from 'yjs'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { syncRuntimeToDoc, DOC_MAP_PAGES } from '../../src/main/runtime/space-doc'
import {
  createCanvasUndoManager,
  setActiveUndoManager,
  markUndoBoundary,
  undo as undoActive,
  redo as redoActive,
  clearUndoHistory,
} from '../../src/main/runtime/space-undo'

interface FakePage {
  id: string
  parentGroupId?: string
}

let doc: Y.Doc
let manager: Y.UndoManager

function makeRuntime(pages: FakePage[]) {
  return {
    pages,
    entities: [],
    workspaceGroups: [],
    workspaceEdges: [],
    workspaceAnnotations: [],
    entityOrderIds: pages.map((p) => p.id),
    zoom: 1,
    pan: { x: 0, y: 0 },
  }
}

function serializePage(page: { id: string }): Record<string, unknown> {
  const p = page as FakePage
  return { id: p.id, parentGroupId: p.parentGroupId }
}

beforeEach(() => {
  doc = new Y.Doc()
  manager = createCanvasUndoManager(doc)
  setActiveUndoManager(manager)
})

afterEach(() => {
  clearUndoHistory()
  manager.destroy()
  setActiveUndoManager(null)
})

describe('syncRuntimeToDoc: parentGroupId clear', () => {
  it('deletes the parentGroupId key from the doc map when cleared to undefined', () => {
    syncRuntimeToDoc(doc, makeRuntime([{ id: 'page-1', parentGroupId: 'group-1' }]), serializePage)

    const yPages = doc.getMap(DOC_MAP_PAGES) as Y.Map<Y.Map<unknown>>
    expect(yPages.get('page-1')?.has('parentGroupId')).toBe(true)
    expect(yPages.get('page-1')?.get('parentGroupId')).toBe('group-1')

    markUndoBoundary()

    syncRuntimeToDoc(doc, makeRuntime([{ id: 'page-1', parentGroupId: undefined }]), serializePage)

    expect(yPages.get('page-1')?.has('parentGroupId')).toBe(false)
  })

  it('undo restores the deleted key; redo removes it again', () => {
    syncRuntimeToDoc(doc, makeRuntime([{ id: 'page-1', parentGroupId: 'group-1' }]), serializePage)
    markUndoBoundary()
    syncRuntimeToDoc(doc, makeRuntime([{ id: 'page-1', parentGroupId: undefined }]), serializePage)

    const yPages = doc.getMap(DOC_MAP_PAGES) as Y.Map<Y.Map<unknown>>
    expect(yPages.get('page-1')?.has('parentGroupId')).toBe(false)

    undoActive()
    expect(yPages.get('page-1')?.has('parentGroupId')).toBe(true)
    expect(yPages.get('page-1')?.get('parentGroupId')).toBe('group-1')

    redoActive()
    expect(yPages.get('page-1')?.has('parentGroupId')).toBe(false)
  })
})
