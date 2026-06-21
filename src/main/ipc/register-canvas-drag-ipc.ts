import { ipcMain } from 'electron'
import type { CanvasDragStartSelection, CanvasHoverTarget } from '../../shared/types'
import { pages } from '../runtime/page-runtime'
import {
  applyDragDelta,
  finalizeDrag,
  finalizeResizeGuides,
  initializeDrag,
  initializeResizeGuides,
  previewDragGuides,
} from '../runtime/document-commands'
import {
  getSelectedEntityIds,
  selectPage,
  setSelectedEntities,
} from '../runtime/ui-actions'
import {
  updateEdgeDragTarget,
  currentInteractionState,
} from '../runtime/interaction-state'
import { tryEnter, commitActive, cancelActive } from '../runtime/interaction-controller'
import { beginBatch, endBatch } from '../runtime/workspace-observers'
import { markUndoBoundary } from '../runtime/workspace-undo'
import { setHoverEntity } from '../runtime/runtime-core'
import type { EdgeSide } from '../../shared/types'
import type { ResizeHandle } from '../../shared/resize-accumulator'
import {
  canvasOrigin,
  cancelCameraAnimation,
  pan,
  requestLayout,
  setPan,
  setZoom,
  win,
  zoom,
} from '../runtime/surface-layout'
import { isFocusSessionActive } from '../runtime/focus-session'
import { setSelectionOverlayRect } from '../runtime/window-shell'
import {
  resolveEntityKind,
  selectEntity as selectCanvasEntity,
  selectNone,
  selectPageById as selectCanvasPageById,
  selectedDragEntityIds,
} from '../runtime/selection-controller'
import { createEdges } from '../workspace-edges'
import { deleteEdge, updateEdge } from '../runtime/document-commands'
import {
  copyableSelectionPayload,
  pasteEntitiesFromClipboard,
} from '../workspace-clipboard'
import { descendantEntityIdsForGroup } from '../runtime/group-descendants'
import { duplicateGroup } from '../workspace-groups'
import { reflowManagedGroupForChild } from '../managed-layout'

const VIEWPORT_EVENT_FRAME_MS = 16

// The entity currently being resized, captured at resize-begin so resize-end can
// reflow its managed group (if any) before committing the gesture's undo step.
let resizingEntityId: string | null = null

let pendingViewportDelta = {
  zoomDeltaY: 0,
  panDeltaX: 0,
  panDeltaY: 0,
  mouseX: null as number | null,
  mouseY: null as number | null,
}
let pendingViewportTimer: NodeJS.Timeout | null = null

function flushViewportDelta(): void {
  pendingViewportTimer = null

  const { zoomDeltaY, panDeltaX, panDeltaY, mouseX, mouseY } =
    pendingViewportDelta
  pendingViewportDelta = {
    zoomDeltaY: 0,
    panDeltaX: 0,
    panDeltaY: 0,
    mouseX: null,
    mouseY: null,
  }

  if (zoomDeltaY !== 0) {
    const oldZoom = zoom
    setZoom(zoom - zoomDeltaY * 0.002)
    const newZoom = zoom

    if (win && mouseX !== null && mouseY !== null) {
      const contentBounds = win.getContentBounds()
      const mouseClientX = mouseX - contentBounds.x
      const mouseClientY = mouseY - contentBounds.y
      const origin = canvasOrigin()
      const viewportMouseX = mouseClientX - origin.x
      const viewportMouseY = mouseClientY - origin.y
      const canvasX = (viewportMouseX - pan.x) / oldZoom
      const canvasY = (viewportMouseY - pan.y) / oldZoom

      setPan(
        viewportMouseX - canvasX * newZoom,
        viewportMouseY - canvasY * newZoom,
      )
    }
  }

  if (panDeltaX !== 0 || panDeltaY !== 0) {
    setPan(pan.x + panDeltaX, pan.y + panDeltaY)
  }

  // A manual pan/zoom supersedes any in-flight camera animation.
  if (zoomDeltaY !== 0 || panDeltaX !== 0 || panDeltaY !== 0) {
    cancelCameraAnimation()
  }

  requestLayout()
}

function scheduleViewportDelta(): void {
  if (pendingViewportTimer) return
  pendingViewportTimer = setTimeout(flushViewportDelta, VIEWPORT_EVENT_FRAME_MS)
}

function resolveDraggedPageIds(pageId: string): string[] {
  return selectedDragEntityIds(pageId)
}

function resolveDraggedEntityIds(entityId: string): string[] {
  return selectedDragEntityIds(entityId)
}

let activeDragSession: {
  kind: 'page' | 'entity' | 'group'
  ids: string[]
} | null = null

function applyDragStartSelection(
  entityId: string,
  selection: CanvasDragStartSelection | undefined,
): void {
  if (!selection || selection.preserveSelection) return
  if (selection.entityKind === 'page') {
    selectCanvasPageById(entityId, { clearInteraction: false })
    return
  }
  selectCanvasEntity(entityId, selection.entityKind, { clearInteraction: false })
}

function beginDragSession(
  kind: 'page' | 'entity' | 'group',
  entityIds: string[],
): boolean {
  if (!entityIds.length) return false
  if (activeDragSession && currentInteractionState().kind === 'idle') {
    activeDragSession = null
  }
  if (activeDragSession) return false
  const token = tryEnter({ kind: 'dragging-entities', entityIds })
  if ('refused' in token) return false
  activeDragSession = { kind, ids: [...entityIds] }
  initializeDrag(entityIds)
  return true
}

function activeDragIds(
  kind: 'page' | 'entity' | 'group',
  anchorId: string,
): string[] | null {
  if (!activeDragSession || activeDragSession.kind !== kind) return null
  if (!activeDragSession.ids.includes(anchorId)) return null
  return activeDragSession.ids
}

function endDragSession(kind: 'page' | 'entity' | 'group'): void {
  if (!activeDragSession || activeDragSession.kind !== kind) return
  activeDragSession = null
  finalizeDrag()
  commitActive()
}

export function registerCanvasDragIpc(): void {
  ipcMain.on(
    'canvas-zoom',
    (_event, data: { deltaY: number; mouseX: number; mouseY: number }) => {
      // Focus presentation locks the camera on the page; exit is escape/button/dim-click only.
      if (isFocusSessionActive()) return
      pendingViewportDelta.zoomDeltaY += data.deltaY
      pendingViewportDelta.mouseX = data.mouseX
      pendingViewportDelta.mouseY = data.mouseY
      scheduleViewportDelta()
    },
  )

  ipcMain.on('canvas-pan', (_event, { deltaX, deltaY }: { deltaX: number; deltaY: number }) => {
    if (isFocusSessionActive()) return
    pendingViewportDelta.panDeltaX -= deltaX
    pendingViewportDelta.panDeltaY -= deltaY
    scheduleViewportDelta()
  })

  ipcMain.on(
    'canvas-selection-overlay',
    (
      _event,
      overlay: import('../../shared/types').SelectionOverlayPayload | null,
    ) => {
      if (overlay) tryEnter({ kind: 'marquee' })
      else commitActive()
      setSelectionOverlayRect(overlay)
    },
  )

  ipcMain.on(
    'canvas-drag-page-start',
    (
      _event,
      { pageId, selection }: { pageId: string; selection?: CanvasDragStartSelection },
    ) => {
      // Enter drag mode BEFORE mutating selection. commitSelection calls
      // requestLayout(); the debounced pass runs reconcileFocus afterward,
      // and unless interactionState.kind has left 'idle' by then the focus
      // reconciler routes focus to bgView and aboveView blurs, which the
      // drag's window blur listener treats as a cancel.
      const started = beginDragSession('page', resolveDraggedPageIds(pageId))
      if (started) applyDragStartSelection(pageId, selection)
    },
  )

  ipcMain.on(
    'canvas-drag-page',
    (
      _event,
      { pageId, dx, dy, shiftKey }: { pageId: string; dx: number; dy: number; shiftKey?: boolean },
    ) => {
      const pageIds = activeDragIds('page', pageId)
      if (!pageIds) return
      if (pageIds.length === 1) {
        const idx = pages.findIndex((candidate) => candidate.id === pageId)
        if (idx !== -1) selectPage(idx)
      }
      applyDragDelta(pageIds, dx, dy, { shiftKey })
      requestLayout()
    },
  )

  ipcMain.on('canvas-drag-page-end', () => {
    endDragSession('page')
    requestLayout()
  })

  ipcMain.on(
    'canvas-drag-copy-selection',
    (_event, { canvasX, canvasY }: { canvasX: number; canvasY: number }) => {
      const payload = copyableSelectionPayload()
      if (!payload) return
      pasteEntitiesFromClipboard({ payload, canvasX, canvasY })
    },
  )

  ipcMain.on(
    'canvas-drag-copy-group',
    (
      _event,
      { groupId, canvasX, canvasY }: { groupId: string; canvasX: number; canvasY: number },
    ) => {
      duplicateGroup({ groupId, focus: true, placement: { canvasX, canvasY } })
    },
  )

  ipcMain.on(
    'canvas-drag-entity-start',
    (
      _event,
      { entityId, selection }: { entityId: string; selection?: CanvasDragStartSelection },
    ) => {
      // See canvas-drag-page-start: enter drag mode before applying selection
      // so the focus reconciler keeps aboveView focused through the layout pass.
      const started = beginDragSession('entity', resolveDraggedEntityIds(entityId))
      if (started) applyDragStartSelection(entityId, selection)
    },
  )

  ipcMain.on(
    'canvas-drag-entity',
    (
      _event,
      { entityId, dx, dy, shiftKey }: { entityId: string; dx: number; dy: number; shiftKey: boolean },
    ) => {
      const entityIds = activeDragIds('entity', entityId)
      if (!entityIds) return
      applyDragDelta(entityIds, dx, dy, { shiftKey })
      requestLayout()
    },
  )

  ipcMain.on('canvas-drag-entity-end', () => {
    endDragSession('entity')
    requestLayout()
  })

  ipcMain.on(
    'canvas-drag-preview',
    (
      _event,
      { dx, dy, shiftKey }: { dx: number; dy: number; shiftKey?: boolean },
    ) => {
      previewDragGuides(dx, dy, { shiftKey })
    },
  )

  ipcMain.on('canvas-drag-group-start', (_event, { groupId }: { groupId: string }) => {
    const entityIds = [groupId, ...descendantEntityIdsForGroup(groupId)]
    beginDragSession('group', entityIds)
  })

  ipcMain.on(
    'canvas-drag-group',
    (
      _event,
      { groupId, dx, dy, shiftKey }: { groupId: string; dx: number; dy: number; shiftKey?: boolean },
    ) => {
      const entityIds = activeDragIds('group', groupId)
      if (!entityIds) return
      applyDragDelta(entityIds, dx, dy, { shiftKey })
      requestLayout()
    },
  )

  ipcMain.on('canvas-drag-group-end', () => {
    endDragSession('group')
    requestLayout()
  })

  ipcMain.on(
    'canvas-resize-begin',
    (
      _event,
      {
        entityId,
        entityKind,
        handle,
      }: {
        entityId: string
        entityKind: import('../../shared/types').CanvasEntityKind
        handle: ResizeHandle
      },
    ) => {
      // Resize gesture begin. The renderer dispatches this BEFORE its first
      // entity-bounds mutation so the layout pass triggered by that mutation
      // sees `interactionState.kind === 'resizing-entity'` instead of `'idle'`.
      // Without it the focus reconciler routes focus to the selected page on
      // the first move tick, aboveView blurs, and the renderer's window-blur
      // listener cancels the gesture after one pixel. Same gotcha as the
      // drag-start ordering — see runtime/CLAUDE.md.
      const resizeToken = tryEnter({ kind: 'resizing-entity', target: { id: entityId, kind: entityKind } })
      if ('refused' in resizeToken) return
      resizingEntityId = entityId
      initializeResizeGuides(entityId, handle)
      // Coalesce the gesture's per-tick bounds mutations into one Y.Doc
      // transaction / one undo step — mirrors drag (initializeDrag/finalizeDrag).
      beginBatch()
    },
  )

  ipcMain.on('canvas-resize-end', () => {
    finalizeResizeGuides()
    // A managed child changing size reflows its siblings within the same batch,
    // so the row re-packs in one undo step (ADR 0015 D3).
    if (resizingEntityId) reflowManagedGroupForChild(resizingEntityId)
    resizingEntityId = null
    endBatch()
    markUndoBoundary()
    commitActive()
  })

  ipcMain.on('canvas-multi-resize-begin', () => {
    const multiResizeToken = tryEnter({ kind: 'resizing-multi-selection' })
    if ('refused' in multiResizeToken) return
    beginBatch()
  })

  ipcMain.on('canvas-multi-resize-end', () => {
    endBatch()
    markUndoBoundary()
    commitActive()
  })

  ipcMain.on(
    'canvas-edge-drag-begin',
    (
      _event,
      { fromEntityId, fromSide }: { fromEntityId: string; fromSide: EdgeSide },
    ) => {
      const edgeToken = tryEnter({
        kind: 'dragging-edge',
        from: { id: fromEntityId, kind: resolveEntityKind(fromEntityId) },
        fromSide,
      })
      if ('refused' in edgeToken) return
      setHoverEntity(null)
      requestLayout()
    },
  )

  ipcMain.on(
    'canvas-edge-drag-target-change',
    (
      _event,
      {
        targetEntityId,
        targetSide,
      }: { targetEntityId: string | null; targetSide: EdgeSide | null },
    ) => {
      const target: CanvasHoverTarget =
        targetEntityId && targetSide
          ? { id: targetEntityId, kind: resolveEntityKind(targetEntityId) }
          : null
      updateEdgeDragTarget(target, targetSide)
      requestLayout()
    },
  )

  ipcMain.on('canvas-edge-drag-cancel', () => {
    cancelActive('escape')
    setHoverEntity(null)
    requestLayout()
  })

  ipcMain.on(
    'canvas-edge-drag-commit',
    (
      _event,
      {
        fromEntityId,
        toEntityId,
        fromSide,
        toSide,
      }: {
        fromEntityId: string
        toEntityId: string
        fromSide: EdgeSide
        toSide: EdgeSide
      },
    ) => {
      const previousSelectedEntityIds = getSelectedEntityIds()
      createEdges({
        edges: [
          {
            fromEntityId,
            toEntityId,
            fromSide,
            toSide,
            toEnd: 'arrow',
            kind: 'connection',
          },
        ],
      })
      commitActive()
      setHoverEntity(null)
      if (previousSelectedEntityIds.includes(fromEntityId)) {
        setSelectedEntities(previousSelectedEntityIds)
        return
      }
      selectNone()
    },
  )

  ipcMain.on(
    'canvas-edge-edit-commit',
    (
      _event,
      {
        edgeId,
        movingEnd,
        targetEntityId,
        targetSide,
      }: {
        edgeId: string
        movingEnd: 'from' | 'to'
        targetEntityId: string
        targetSide: EdgeSide
      },
    ) => {
      if (movingEnd === 'from') {
        updateEdge(edgeId, { fromEntityId: targetEntityId, fromSide: targetSide })
      } else {
        updateEdge(edgeId, { toEntityId: targetEntityId, toSide: targetSide })
      }
      commitActive()
      setHoverEntity(null)
      requestLayout()
    },
  )

  ipcMain.on(
    'canvas-edge-edit-discard',
    (_event, { edgeId }: { edgeId: string }) => {
      deleteEdge(edgeId)
      cancelActive('escape')
      setHoverEntity(null)
      requestLayout()
    },
  )
}
