/**
 * Workspace Observers
 *
 * Wires Y.Doc observers for:
 * 1. Undo/redo sync: when UndoManager reverts Y.Doc, rebuild runtime arrays
 * 2. Forward-path sync: after each mutation, diff-sync runtime → Y.Doc
 *
 * The forward-path sync is triggered by `requestDocSync()` which uses
 * queueMicrotask to batch multiple mutations in the same tick.
 */

import { markAllDirty } from './layout-dirty'
import { requestLayout } from './viewport-control'
import { allEntities, forEachEntityKind, getEntityKind } from '../entities/contract'
import { persistPage } from './page-doc-projection'
import type * as Y from 'yjs'
import type {
  Annotation,
  CanvasEntityKind,
  PersistedWorkspaceTab,
  WorkspaceEdge,
  WorkspaceGroup,
} from '../../shared/types'
import type { Page } from './runtime-entities'
import {
  getActiveDoc,
  getDocActiveTabId,
  getDocTabList,
  isDocSyncSuppressed,
  syncRuntimeToDoc,
  withSuppressedDocSync,
  readNoteEntries,
  DOC_MAP_PAGES,
  DOC_MAP_ENTITIES,
  DOC_MAP_GROUPS,
  DOC_MAP_EDGES,
  DOC_MAP_ANNOTATIONS,
} from './workspace-doc'
import { getActiveUndoManager } from './workspace-undo'
import { makeEmptyTabSnapshot } from './workspace-tabs'
import {
  noteContentEntries,
  applyNoteContentsFromDoc,
  projectNoteContentToDisk,
  clearNoteContentState,
} from './note-content-state'

// ---------------------------------------------------------------------------
// Runtime state references (set during initialization)
// ---------------------------------------------------------------------------

interface RuntimeStateRefs {
  pages: Page[]
  workspaceGroups: WorkspaceGroup[]
  workspaceEdges: WorkspaceEdge[]
  workspaceAnnotations: Annotation[]
  getZoom: () => number
  getPan: () => { x: number; y: number }
  cancelActiveInteraction: () => void
  sendInteractiveState: () => void
  // Cross-tab undo: full rebuild when activeTabId changes
  destroyActivePages: () => void
  getActiveTabId: () => string | null
  setActiveTabId: (id: string | null) => void
  // Tab list for undo of create/delete tab
  workspaceTabs: PersistedWorkspaceTab[]
}

let _refs: RuntimeStateRefs | null = null
let _activeObserverDoc: { doc: typeof import('yjs').Doc.prototype; handler: Function } | null = null

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

export function initializeDocObservers(refs: RuntimeStateRefs): void {
  _refs = refs

  // Remove previous observer if re-initializing
  if (_activeObserverDoc) {
    _activeObserverDoc.doc.off('afterTransaction', _activeObserverDoc.handler as any)
    _activeObserverDoc = null
  }

  const doc = getActiveDoc()
  requestDocSyncImmediate()

  const handler = (transaction: { origin: unknown }) => {
    const undoManager = getActiveUndoManager()
    if (!undoManager || transaction.origin !== undoManager) return
    syncDocToRuntime(doc)
  }
  doc.on('afterTransaction', handler)
  _activeObserverDoc = { doc, handler }
}

// ---------------------------------------------------------------------------
// Forward-path sync: runtime → Y.Doc (batched via microtask)
// ---------------------------------------------------------------------------

let _syncScheduled = false
let _batchingActive = false
// One-shot: set by commitAsOneTransaction after it has already synced, so the
// mutation trailer's requestDocSync doesn't open a second (empty) transaction.
let _docSyncSatisfied = false

/**
 * Begin batching: suppress doc sync until endBatch() is called, coalescing a
 * series of fine-grained mutations (e.g. drag increments) into a single
 * Y.Doc transaction. The gesture session (workspace-gesture-session.ts) is
 * the caller — it pairs the batch with the gesture's one undo boundary.
 */
export function beginBatch(): void {
  _batchingActive = true
}

/** End batching: perform one sync for all accumulated mutations. */
export function endBatch(): void {
  _batchingActive = false
  if (!_refs) return
  requestDocSyncImmediate()
}

/**
 * Run `mutate` and flush the resulting runtime→Y.Doc sync inside a single Y.Doc
 * transaction, so it collapses to exactly one undo step.
 *
 * Needed when an operation mixes a *direct* doc write (e.g. `writeEntityOrder`,
 * which transacts on its own) with *runtime* mutations that reach the doc only
 * through the diff-sync (e.g. reflow position writes). Left apart these are two
 * transactions — two undo entries, since the UndoManager uses `captureTimeout:
 * 0`. Yjs flattens nested transactions, so wrapping both in one outer transact
 * merges them. Reorder + reflow is the canonical caller (ADR 0015 undo
 * batching).
 */
export function commitAsOneTransaction(mutate: () => void): void {
  if (!_refs) {
    mutate()
    return
  }
  // Suppress the microtask sync that `scheduleWorkspaceAutosave()` would queue
  // from inside `mutate` — we sync once, synchronously, inside the transaction.
  // Without this, that trailing (empty) sync fires a second transaction.
  const wasBatching = _batchingActive
  _batchingActive = true
  try {
    getActiveDoc().transact(() => {
      mutate()
      requestDocSyncImmediate()
    }, 'user')
  } finally {
    _batchingActive = wasBatching
  }
  // The doc now matches runtime, but the `mutateWorkspace` trailer runs
  // *after* this returns and its `requestDocSync` would open a second, empty
  // transaction. Swallow exactly that one request; the flag dies with the
  // current tick so later syncs are unaffected.
  _docSyncSatisfied = true
  queueMicrotask(() => {
    _docSyncSatisfied = false
  })
}

/**
 * Schedule a diff-sync from runtime state to Y.Doc.
 * Uses queueMicrotask so multiple mutations in the same tick become one sync.
 * Call this from scheduleWorkspaceAutosave().
 */
export function requestDocSync(): void {
  if (_docSyncSatisfied) {
    _docSyncSatisfied = false
    return
  }
  if (isDocSyncSuppressed() || _batchingActive || _syncScheduled || !_refs) return
  _syncScheduled = true
  queueMicrotask(() => {
    _syncScheduled = false
    if (!_refs || isDocSyncSuppressed() || _batchingActive) return
    requestDocSyncImmediate()
  })
}

/** Immediate sync (no microtask). Used during initialization and endBatch. */
function requestDocSyncImmediate(): void {
  if (!_refs) return
  const doc = getActiveDoc()
  // Walk the registry once: the entity map takes the map-projectable kinds
  // (page and group mirror to their own maps); stack order takes every entity
  // id in registration order, then edges.
  const registryEntities = allEntities()
  const entities = registryEntities
    .filter(({ kind }) => kind !== 'page' && kind !== 'group')
    .map(({ kind, entity }) => getEntityKind(kind).persist!(entity))
  const entityOrderIds = [
    ...registryEntities.map(({ entity }) => entity.id),
    ..._refs.workspaceEdges.map((edge) => edge.id),
  ]
  syncRuntimeToDoc(doc, {
    pages: _refs.pages,
    entities,
    workspaceGroups: _refs.workspaceGroups,
    workspaceEdges: _refs.workspaceEdges,
    workspaceAnnotations: _refs.workspaceAnnotations,
    entityOrderIds,
    zoom: _refs.getZoom(),
    pan: _refs.getPan(),
    activeTabId: _refs.getActiveTabId(),
    workspaceTabs: _refs.workspaceTabs,
    noteContent: noteContentEntries(),
  }, persistPage as (page: { id: string }) => Record<string, unknown>)
}

// ---------------------------------------------------------------------------
// Undo-path sync: Y.Doc → runtime (on undo/redo)
// ---------------------------------------------------------------------------

function rebuildArrayFromYMap<T>(target: T[], ymap: Y.Map<Y.Map<unknown>>): void {
  target.length = 0
  for (const [, ym] of ymap.entries()) {
    target.push(ym.toJSON() as T)
  }
}

function mapSnapshots(ymap: Y.Map<Y.Map<unknown>>): Record<string, unknown>[] {
  const out: Record<string, unknown>[] = []
  for (const [, ym] of ymap.entries()) {
    out.push(ym.toJSON() as Record<string, unknown>)
  }
  return out
}

function syncDocToRuntime(doc: Y.Doc): void {
  if (!_refs) return

  withSuppressedDocSync(() => {
    const docTabId = getDocActiveTabId(doc)
    const currentTabId = _refs!.getActiveTabId()
    const isCrossTabUndo = docTabId !== null && docTabId !== currentTabId

    const docTabs = getDocTabList(doc)
    if (docTabs.length > 0) {
      const docTabIds = new Set(docTabs.map((t) => t.id))
      const runtimeTabIds = new Set(_refs!.workspaceTabs.map((t) => t.id))

      for (let i = _refs!.workspaceTabs.length - 1; i >= 0; i--) {
        if (!docTabIds.has(_refs!.workspaceTabs[i].id)) {
          _refs!.workspaceTabs.splice(i, 1)
        }
      }

      for (const docTab of docTabs) {
        if (!runtimeTabIds.has(docTab.id)) {
          _refs!.workspaceTabs.push({
            id: docTab.id,
            name: docTab.name,
            updatedAt: new Date().toISOString(),
            snapshot: makeEmptyTabSnapshot(),
            annotations: [],
            expanded: true,
          })
        }
      }

      for (const docTab of docTabs) {
        const runtimeTab = _refs!.workspaceTabs.find((t) => t.id === docTab.id)
        if (runtimeTab && runtimeTab.name !== docTab.name) {
          runtimeTab.name = docTab.name
        }
      }
    }

    if (isCrossTabUndo) {
      _refs!.setActiveTabId(docTabId)
      _refs!.destroyActivePages()
    }

    // Reconcile every registered kind's runtime store from its doc map: page
    // and group mirror to their own maps; the rest share the entity map,
    // bucketed by their persisted `kind` field. Each kind's `restore` runs
    // even when its bucket is empty, so undo of a create clears the store.
    // Edges and annotations are not registered kinds; they rebuild below.
    const entitySnapshots = new Map<CanvasEntityKind, Record<string, unknown>[]>()
    const yEntities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>
    for (const [, yEntity] of yEntities.entries()) {
      const data = yEntity.toJSON() as Record<string, unknown>
      const kind = data.kind as CanvasEntityKind
      const bucket = entitySnapshots.get(kind)
      if (bucket) bucket.push(data)
      else entitySnapshots.set(kind, [data])
    }
    const yPages = doc.getMap(DOC_MAP_PAGES) as Y.Map<Y.Map<unknown>>
    const yGroups = doc.getMap(DOC_MAP_GROUPS) as Y.Map<Y.Map<unknown>>
    forEachEntityKind((def) => {
      const snapshots =
        def.kind === 'page'
          ? mapSnapshots(yPages)
          : def.kind === 'group'
            ? mapSnapshots(yGroups)
            : entitySnapshots.get(def.kind) ?? []
      def.restore(snapshots)
    })

    rebuildArrayFromYMap(_refs!.workspaceEdges, doc.getMap(DOC_MAP_EDGES) as Y.Map<Y.Map<unknown>>)
    rebuildArrayFromYMap(_refs!.workspaceAnnotations, doc.getMap(DOC_MAP_ANNOTATIONS) as Y.Map<Y.Map<unknown>>)

    // Note content: pull the reverted `notes` Y.Map back into the runtime
    // mirror, then project just the ids that actually changed to disk.
    const changedNoteIds = applyNoteContentsFromDoc(readNoteEntries(doc))
    for (const id of changedNoteIds) projectNoteContentToDisk(id)

    // Phase 5d-v2 E1: gesture cancellation flows through the controller,
    // which is reentrancy-safe, so the undo observer can cancel + mark
    // dirty + request a layout synchronously. The 16ms layout debounce
    // in requestLayout() provides enough deferral to avoid stepping on
    // Electron's event routing.
    _refs!.cancelActiveInteraction()
    _refs!.sendInteractiveState()
    markAllDirty()
    requestLayout()
  })
}

// ---------------------------------------------------------------------------
// Reset (on tab switch or workspace load)
// ---------------------------------------------------------------------------

export function resetDocSync(): void {
  _syncScheduled = false
  _batchingActive = false
  clearNoteContentState()
}
