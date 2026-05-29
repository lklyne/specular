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
import type * as Y from 'yjs'
import type { Annotation, PersistedWorkspaceTab, WorkspaceEdge, WorkspaceGroup } from '../../shared/types'
import type { Page } from './runtime-entities'
import type { TextEntity } from './text-entity-state'
import type { FileEntity } from './file-entity-state'
import type { DrawingEntity } from './drawing-entity-state'
import type { ShapeEntity } from './shape-entity-state'
import {
  getActiveDoc,
  getDocActiveTabId,
  getDocTabList,
  isDocSyncSuppressed,
  syncRuntimeToDoc,
  withSuppressedDocSync,
  DOC_MAP_PAGES,
  DOC_MAP_ENTITIES,
  DOC_MAP_GROUPS,
  DOC_MAP_EDGES,
  DOC_MAP_ANNOTATIONS,
} from './workspace-doc'
import { getActiveUndoManager } from './workspace-undo'
import { makeEmptyTabSnapshot } from './workspace-tabs'

// ---------------------------------------------------------------------------
// Runtime state references (set during initialization)
// ---------------------------------------------------------------------------

interface RuntimeStateRefs {
  pages: Page[]
  textEntities: TextEntity[]
  fileEntities: FileEntity[]
  drawingEntities: DrawingEntity[]
  shapeEntities: ShapeEntity[]
  workspaceGroups: WorkspaceGroup[]
  workspaceEdges: WorkspaceEdge[]
  workspaceAnnotations: Annotation[]
  getZoom: () => number
  getPan: () => { x: number; y: number }
  serializePage: (page: Page) => Record<string, unknown>
  cancelActiveInteraction: () => void
  sendInteractiveState: () => void
  // Page lifecycle for undo of create/delete
  createPage: (data: Record<string, unknown>) => void
  removePageById: (id: string) => void
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

/**
 * Begin batching: suppress doc sync until endBatch() is called.
 * Use this to coalesce a series of fine-grained mutations (e.g. drag
 * increments) into a single Y.Doc transaction / undo step.
 */
export function beginBatch(): void {
  _batchingActive = true
}

/**
 * End batching: perform one sync for all accumulated mutations,
 * then mark an undo boundary so the batch is one undo step.
 */
export function endBatch(): void {
  _batchingActive = false
  if (!_refs) return
  requestDocSyncImmediate()
}

/**
 * Schedule a diff-sync from runtime state to Y.Doc.
 * Uses queueMicrotask so multiple mutations in the same tick become one sync.
 * Call this from scheduleWorkspaceAutosave().
 */
export function requestDocSync(): void {
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
  syncRuntimeToDoc(doc, {
    pages: _refs.pages,
    textEntities: _refs.textEntities,
    fileEntities: _refs.fileEntities,
    drawingEntities: _refs.drawingEntities,
    shapeEntities: _refs.shapeEntities,
    workspaceGroups: _refs.workspaceGroups,
    workspaceEdges: _refs.workspaceEdges,
    workspaceAnnotations: _refs.workspaceAnnotations,
    zoom: _refs.getZoom(),
    pan: _refs.getPan(),
    activeTabId: _refs.getActiveTabId(),
    workspaceTabs: _refs.workspaceTabs,
  }, _refs.serializePage as (page: { id: string }) => Record<string, unknown>)
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

    // Reconcile pages: remove deleted, add restored
    const yPages = doc.getMap(DOC_MAP_PAGES) as Y.Map<Y.Map<unknown>>
    const runtimePageIds = new Set(_refs!.pages.map((p) => p.id))
    const docPageIds = new Set(yPages.keys())

    for (const page of [..._refs!.pages]) {
      if (!docPageIds.has(page.id)) {
        _refs!.removePageById(page.id)
      }
    }

    for (const [id, yPage] of yPages.entries()) {
      if (!runtimePageIds.has(id)) {
        _refs!.createPage(yPage.toJSON() as Record<string, unknown>)
      }
    }

    // Update properties on existing pages
    for (const page of _refs!.pages) {
      const yPage = yPages.get(page.id)
      if (!yPage) continue
      page.canvasX = (yPage.get('canvasX') as number) ?? page.canvasX
      page.canvasY = (yPage.get('canvasY') as number) ?? page.canvasY
      page.presetIndex = (yPage.get('presetIndex') as number) ?? page.presetIndex
      page.linked = (yPage.get('linked') as boolean) ?? page.linked
      page.parentGroupId = yPage.get('parentGroupId') as string | undefined
      page.name = yPage.get('name') as string | undefined
      if (yPage.get('metadata') !== undefined) {
        page.metadata = yPage.get('metadata') as Record<string, unknown> | undefined
      }
    }

    const yEntities = doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>
    _refs!.textEntities.length = 0
    _refs!.fileEntities.length = 0
    _refs!.drawingEntities.length = 0
    _refs!.shapeEntities.length = 0
    for (const [, yEntity] of yEntities.entries()) {
      const data = yEntity.toJSON() as Record<string, unknown>
      const kind = data.kind as string
      if (kind === 'text') {
        _refs!.textEntities.push(data as unknown as TextEntity)
      } else if (kind === 'file') {
        _refs!.fileEntities.push(data as unknown as FileEntity)
      } else if (kind === 'drawing') {
        _refs!.drawingEntities.push(data as unknown as DrawingEntity)
      } else if (kind === 'shape') {
        _refs!.shapeEntities.push(data as unknown as ShapeEntity)
      }
    }

    rebuildArrayFromYMap(_refs!.workspaceGroups, doc.getMap(DOC_MAP_GROUPS) as Y.Map<Y.Map<unknown>>)
    rebuildArrayFromYMap(_refs!.workspaceEdges, doc.getMap(DOC_MAP_EDGES) as Y.Map<Y.Map<unknown>>)
    rebuildArrayFromYMap(_refs!.workspaceAnnotations, doc.getMap(DOC_MAP_ANNOTATIONS) as Y.Map<Y.Map<unknown>>)

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
}
