import { UndoManager } from 'yjs'
import type * as Y from 'yjs'
import {
  DOC_MAP_PAGES,
  DOC_MAP_GROUPS,
  DOC_MAP_EDGES,
  DOC_MAP_ANNOTATIONS,
  DOC_MAP_ENTITIES,
  DOC_MAP_WORKSPACE,
  DOC_MAP_NOTES,
  DOC_ARRAY_ENTITY_ORDER,
} from './space-doc'
import { breadcrumb } from '../sentry-context'

const MAX_UNDO_STACK = 100

let activeUndoManager: UndoManager | null = null

let readSelectionFn: (() => unknown) | null = null
let restoreSelectionFn: ((selection: unknown) => void) | null = null

// File-system side-effects that must reverse alongside an entity mutation
// (ADR 0013 §3 — text/markdown morph). A mutation that creates or deletes a
// note file pushes a paired (undo, redo) callback before scheduling the
// Y.Doc sync; the next `stack-item-added` event drains the queue into the
// fresh stack item's meta. On `stack-item-popped`, we replay whichever side
// matches the event direction.
type SideEffect = { undo: () => void; redo: () => void }
const pendingSideEffects: SideEffect[] = []

export function pushPendingUndoSideEffect(effect: SideEffect): void {
  pendingSideEffects.push(effect)
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

export function createCanvasUndoManager(doc: Y.Doc): UndoManager {
  const undoableTypes: Y.AbstractType<unknown>[] = [
    doc.getMap(DOC_MAP_PAGES) as unknown as Y.AbstractType<unknown>,
    doc.getMap(DOC_MAP_GROUPS) as unknown as Y.AbstractType<unknown>,
    doc.getMap(DOC_MAP_EDGES) as unknown as Y.AbstractType<unknown>,
    doc.getMap(DOC_MAP_ANNOTATIONS) as unknown as Y.AbstractType<unknown>,
    doc.getMap(DOC_MAP_ENTITIES) as unknown as Y.AbstractType<unknown>,
    doc.getMap(DOC_MAP_WORKSPACE) as unknown as Y.AbstractType<unknown>,
    doc.getMap(DOC_MAP_NOTES) as unknown as Y.AbstractType<unknown>,
    doc.getArray(DOC_ARRAY_ENTITY_ORDER) as unknown as Y.AbstractType<unknown>,
  ]

  const manager = new UndoManager(undoableTypes, {
    // Each discrete operation is its own undo step.
    // Use markUndoBoundary() / stopCapturing() between operations.
    captureTimeout: 0,
    // Track local user changes (null origin = default, 'user' = explicit tag)
    trackedOrigins: new Set([null, 'user']),
  })

  manager.on('stack-item-added', (event: { stackItem: { meta: Map<string, unknown> } }) => {
    if (readSelectionFn) {
      event.stackItem.meta.set('selection', readSelectionFn())
    }
    if (pendingSideEffects.length > 0) {
      event.stackItem.meta.set('side-effects', pendingSideEffects.slice())
      pendingSideEffects.length = 0
    }
    if (manager.undoStack.length > MAX_UNDO_STACK) {
      manager.undoStack.splice(0, manager.undoStack.length - MAX_UNDO_STACK)
    }
  })

  manager.on(
    'stack-item-popped',
    (event: { stackItem: { meta: Map<string, unknown> }; type: 'undo' | 'redo' }) => {
      const effects = event.stackItem.meta.get('side-effects') as SideEffect[] | undefined
      if (effects) {
        for (const effect of effects) {
          try {
            if (event.type === 'undo') effect.undo()
            else effect.redo()
          } catch {
            /* best-effort — never let a file side-effect crash undo */
          }
        }
      }
      const saved = event.stackItem.meta.get('selection')
      if (saved && restoreSelectionFn) {
        restoreSelectionFn(saved)
      }
    },
  )

  activeUndoManager = manager
  return manager
}

export function getActiveUndoManager(): UndoManager | null {
  return activeUndoManager
}

export function setActiveUndoManager(manager: UndoManager | null): void {
  activeUndoManager = manager
}

// ---------------------------------------------------------------------------
// Selection hooks (wired during app init)
// ---------------------------------------------------------------------------

export function setUndoSelectionHooks(
  readSelection: () => unknown,
  restoreSelection: (selection: unknown) => void,
): void {
  readSelectionFn = readSelection
  restoreSelectionFn = restoreSelection
}

// ---------------------------------------------------------------------------
// Undo/Redo API
// ---------------------------------------------------------------------------

export function undo(): void {
  breadcrumb('undo', 'undo')
  activeUndoManager?.undo()
}

export function redo(): void {
  breadcrumb('undo', 'redo')
  activeUndoManager?.redo()
}

export function canUndo(): boolean {
  return (activeUndoManager?.undoStack.length ?? 0) > 0
}

export function canRedo(): boolean {
  return (activeUndoManager?.redoStack.length ?? 0) > 0
}

/**
 * Call between discrete tool operations to ensure the next mutation starts
 * a new undo step. For example, call after drag-end so the next operation
 * doesn't merge with the drag.
 */
export function markUndoBoundary(): void {
  activeUndoManager?.stopCapturing()
}

/**
 * Clear both undo and redo stacks. Useful on tab switch or workspace load.
 */
export function clearUndoHistory(): void {
  if (activeUndoManager) {
    activeUndoManager.clear()
  }
}
