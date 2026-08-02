/**
 * UndoManager batching unit tests.
 *
 * Exercises `createCanvasUndoManager()` from src/main/runtime/space-undo.ts
 * against a raw Y.Doc — no Electron — so we can assert the batching
 * invariants the runtime depends on:
 *
 *   - mutations issued inside a single `doc.transact()` collapse to one
 *     undo step (used by drag finalization),
 *   - distinct user actions (separate transacts, or `markUndoBoundary()`
 *     between them) stay as separate undo steps,
 *   - only origins in `trackedOrigins` (null + 'user') participate.
 *
 * Mutation-verified by dropping `'user'` from `trackedOrigins` inside
 * `createCanvasUndoManager()` — four of the five cases below fail (the
 * `'user'`-origin transactions stop landing on the undo stack).
 */

import * as Y from 'yjs'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  createCanvasUndoManager,
  setActiveUndoManager,
  markUndoBoundary,
  undo as undoActive,
  redo as redoActive,
  canUndo,
  canRedo,
  clearUndoHistory,
} from '../../src/main/runtime/space-undo'
import { DOC_MAP_ENTITIES } from '../../src/main/runtime/space-doc'

let doc: Y.Doc
let manager: Y.UndoManager

beforeEach(() => {
  doc = new Y.Doc()
  manager = createCanvasUndoManager(doc)
})

afterEach(() => {
  clearUndoHistory()
  manager.destroy()
  setActiveUndoManager(null)
})

function setEntity(id: string, payload: Record<string, unknown>, origin: string | null = null): void {
  doc.transact(() => {
    const entities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>
    const ymap = new Y.Map<unknown>()
    for (const [k, v] of Object.entries(payload)) ymap.set(k, v)
    entities.set(id, ymap)
  }, origin)
}

describe('UndoManager batching', () => {
  it('mutations inside a single transact() collapse to one undo step', () => {
    doc.transact(() => {
      const entities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>
      for (const id of ['a', 'b', 'c']) {
        const ymap = new Y.Map<unknown>()
        ymap.set('id', id)
        entities.set(id, ymap)
      }
    }, 'user')

    expect(manager.undoStack.length).toBe(1)

    undoActive()
    const entities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<unknown>
    expect(entities.size).toBe(0)
  })

  it('distinct user actions remain distinct undo steps', () => {
    setEntity('a', { id: 'a' }, 'user')
    markUndoBoundary()
    setEntity('b', { id: 'b' }, 'user')

    expect(manager.undoStack.length).toBe(2)

    undoActive()
    const entities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<unknown>
    expect(entities.has('a')).toBe(true)
    expect(entities.has('b')).toBe(false)

    undoActive()
    expect(entities.has('a')).toBe(false)
  })

  it('redo replays popped steps in order', () => {
    setEntity('a', { id: 'a' }, 'user')
    markUndoBoundary()
    setEntity('b', { id: 'b' }, 'user')

    undoActive() // removes b
    undoActive() // removes a

    expect(canUndo()).toBe(false)
    expect(canRedo()).toBe(true)

    redoActive() // restores a
    const entities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<unknown>
    expect(entities.has('a')).toBe(true)
    expect(entities.has('b')).toBe(false)

    redoActive() // restores b
    expect(entities.has('b')).toBe(true)
  })

  it('untracked origins are ignored by the undo stack', () => {
    setEntity('a', { id: 'a' }, 'remote') // not in trackedOrigins
    expect(manager.undoStack.length).toBe(0)

    setEntity('b', { id: 'b' }, null) // null origin is tracked by default
    expect(manager.undoStack.length).toBe(1)
  })

  it('clearUndoHistory drops both stacks', () => {
    setEntity('a', { id: 'a' }, 'user')
    markUndoBoundary()
    setEntity('b', { id: 'b' }, 'user')
    undoActive()

    expect(canUndo()).toBe(true)
    expect(canRedo()).toBe(true)

    clearUndoHistory()

    expect(canUndo()).toBe(false)
    expect(canRedo()).toBe(false)
  })
})
