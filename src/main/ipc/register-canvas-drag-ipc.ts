import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type { CanvasDragStartSelection, CanvasHoverTarget } from '../../shared/types'
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
  setSelectedEntities,
} from '../runtime/ui-actions'
import {
  updateEdgeDragTarget,
  currentInteractionState,
} from '../runtime/interaction-state'
import { tryEnter, commitActive, cancelActive } from '../runtime/interaction-controller'
import {
  beginGestureSession,
  type GestureSession,
} from '../runtime/workspace-gesture-session'
import { setHoverEntity } from '../runtime/runtime-core'
import type { EdgeSide } from '../../shared/types'
import type { ResizeHandle } from '../../shared/resize-accumulator'
import { requestLayout } from '../runtime/viewport-control'
import { enqueueViewportInputDelta } from '../runtime/viewport-input'
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
import { withPageAnchoredEntityIds } from '../runtime/page-anchor-state'
import { duplicateGroup } from '../workspace-groups'
import { reflowManagedGroupForChild } from '../managed-layout'
import { reparentEntitiesInGesture } from '../runtime/group-membership'

// The entity currently being resized, captured at resize-begin so resize-end can
// reflow its managed group (if any) before committing the gesture's undo step.
let resizingEntityId: string | null = null

// One variable serves both resize and multi-resize: the interaction token
// (I2) guarantees the two gestures never overlap.
let resizeSession: GestureSession | null = null

function expandDraggedGroupIds(entityIds: string[]): string[] {
  const expanded = new Set<string>()
  for (const entityId of entityIds) {
    expanded.add(entityId)
    if (resolveEntityKind(entityId) !== 'group') continue
    for (const descendantId of descendantEntityIdsForGroup(entityId)) {
      expanded.add(descendantId)
    }
  }
  return [...expanded]
}

function resolveDraggedSelection(entityId: string): {
  entityIds: string[]
  membershipIds: string[]
} {
  const membershipIds = selectedDragEntityIds(entityId)
  return {
    entityIds: expandDraggedGroupIds(membershipIds),
    membershipIds,
  }
}

let activeDragSession: {
  kind: 'page' | 'entity'
  ids: string[]
  membershipIds: string[]
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
  kind: 'page' | 'entity',
  ids: string[],
  membershipIds: string[] = ids,
): boolean {
  if (!ids.length) return false
  // Entities anchored to a dragged page travel with it (shared/page-anchor.ts).
  const entityIds = withPageAnchoredEntityIds(ids)
  if (activeDragSession && currentInteractionState().kind === 'idle') {
    activeDragSession = null
  }
  if (activeDragSession) return false
  const token = tryEnter({ kind: 'dragging-entities', entityIds })
  if ('refused' in token) return false
  activeDragSession = { kind, ids: [...entityIds], membershipIds: [...membershipIds] }
  initializeDrag(entityIds)
  return true
}

function activeDragIds(
  kind: 'page' | 'entity',
  anchorId: string,
): string[] | null {
  if (!activeDragSession || activeDragSession.kind !== kind) return null
  if (!activeDragSession.ids.includes(anchorId)) return null
  return activeDragSession.ids
}

function endDragSession(
  kind: 'page' | 'entity',
  parentGroupId?: string | null,
  suppressDropBinding = false,
): void {
  if (!activeDragSession || activeDragSession.kind !== kind) return
  if (!suppressDropBinding && parentGroupId !== undefined) {
    reparentEntitiesInGesture(activeDragSession.membershipIds, parentGroupId)
  }
  activeDragSession = null
  finalizeDrag({ reanchor: !suppressDropBinding })
  commitActive()
}

export function registerCanvasDragIpc(): void {
  ipcMain.on(
    ipcChannels.canvasZoom,
    (_event, data: { deltaY: number; mouseX: number; mouseY: number }) => {
      // Focus presentation locks the camera on the page; exit is escape/button/dim-click only.
      if (isFocusSessionActive()) return
      enqueueViewportInputDelta({
        zoomDeltaY: data.deltaY,
        mouseX: data.mouseX,
        mouseY: data.mouseY,
      })
    },
  )

  ipcMain.on(ipcChannels.canvasPan, (_event, { deltaX, deltaY }: { deltaX: number; deltaY: number }) => {
    if (isFocusSessionActive()) return
    enqueueViewportInputDelta({ panDeltaX: -deltaX, panDeltaY: -deltaY })
  })

  ipcMain.on(
    ipcChannels.canvasSelectionOverlay,
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
    ipcChannels.canvasDragPageStart,
    (
      _event,
      { pageId, selection }: { pageId: string; selection?: CanvasDragStartSelection },
    ) => {
      // Enter drag mode BEFORE mutating selection. commitSelection calls
      // requestLayout(); the debounced pass runs reconcileFocus afterward,
      // and unless interactionState.kind has left 'idle' by then the focus
      // reconciler routes focus to bgView and aboveView blurs, which the
      // drag's window blur listener treats as a cancel.
      const dragSelection = resolveDraggedSelection(pageId)
      const started = beginDragSession(
        'page',
        dragSelection.entityIds,
        dragSelection.membershipIds,
      )
      if (started) applyDragStartSelection(pageId, selection)
    },
  )

  ipcMain.on(
    ipcChannels.canvasDragPage,
    (
      _event,
      { pageId, dx, dy, shiftKey }: { pageId: string; dx: number; dy: number; shiftKey?: boolean },
    ) => {
      const pageIds = activeDragIds('page', pageId)
      if (!pageIds) return
      applyDragDelta(pageIds, dx, dy, { shiftKey })
      requestLayout()
    },
  )

  ipcMain.on(
    ipcChannels.canvasDragPageEnd,
    (
      _event,
      payload?: {
        parentGroupId?: string | null
        suppressDropBinding?: boolean
      },
    ) => {
      endDragSession(
        'page',
        payload?.parentGroupId,
        payload?.suppressDropBinding,
      )
      requestLayout()
    },
  )

  ipcMain.on(
    ipcChannels.canvasDragCopySelection,
    (_event, { canvasX, canvasY }: { canvasX: number; canvasY: number }) => {
      const payload = copyableSelectionPayload()
      if (!payload) return
      pasteEntitiesFromClipboard({ payload, canvasX, canvasY })
    },
  )

  ipcMain.on(
    ipcChannels.canvasDragCopyGroup,
    (
      _event,
      { groupId, canvasX, canvasY }: { groupId: string; canvasX: number; canvasY: number },
    ) => {
      duplicateGroup({ groupId, focus: true, placement: { canvasX, canvasY } })
    },
  )

  ipcMain.on(
    ipcChannels.canvasDragEntityStart,
    (
      _event,
      { entityId, selection }: { entityId: string; selection?: CanvasDragStartSelection },
    ) => {
      // See canvas-drag-page-start: enter drag mode before applying selection
      // so the focus reconciler keeps aboveView focused through the layout pass.
      const dragSelection = resolveDraggedSelection(entityId)
      const started = beginDragSession(
        'entity',
        dragSelection.entityIds,
        dragSelection.membershipIds,
      )
      if (started) applyDragStartSelection(entityId, selection)
    },
  )

  ipcMain.on(
    ipcChannels.canvasDragEntity,
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

  ipcMain.on(
    ipcChannels.canvasDragEntityEnd,
    (
      _event,
      payload?: {
        parentGroupId?: string | null
        suppressDropBinding?: boolean
      },
    ) => {
      endDragSession(
        'entity',
        payload?.parentGroupId,
        payload?.suppressDropBinding,
      )
      requestLayout()
    },
  )

  ipcMain.on(
    ipcChannels.canvasDragPreview,
    (
      _event,
      { dx, dy, shiftKey }: { dx: number; dy: number; shiftKey?: boolean },
    ) => {
      previewDragGuides(dx, dy, { shiftKey })
    },
  )

  ipcMain.on(
    ipcChannels.canvasResizeBegin,
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
      resizeSession = beginGestureSession()
    },
  )

  ipcMain.on(ipcChannels.canvasResizeEnd, () => {
    finalizeResizeGuides()
    // A managed child changing size reflows its siblings within the same batch,
    // so the row re-packs in one undo step (ADR 0015 D3).
    if (resizingEntityId) reflowManagedGroupForChild(resizingEntityId)
    resizingEntityId = null
    resizeSession?.finalize()
    resizeSession = null
    commitActive()
  })

  ipcMain.on(ipcChannels.canvasMultiResizeBegin, () => {
    const multiResizeToken = tryEnter({ kind: 'resizing-multi-selection' })
    if ('refused' in multiResizeToken) return
    resizeSession = beginGestureSession()
  })

  ipcMain.on(ipcChannels.canvasMultiResizeEnd, () => {
    resizeSession?.finalize()
    resizeSession = null
    commitActive()
  })

  ipcMain.on(
    ipcChannels.canvasEdgeDragBegin,
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
    ipcChannels.canvasEdgeDragTargetChange,
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

  ipcMain.on(ipcChannels.canvasEdgeDragCancel, () => {
    cancelActive('escape')
    setHoverEntity(null)
    requestLayout()
  })

  ipcMain.on(
    ipcChannels.canvasEdgeDragCommit,
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
    ipcChannels.canvasEdgeEditCommit,
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
    ipcChannels.canvasEdgeEditDiscard,
    (_event, { edgeId }: { edgeId: string }) => {
      deleteEdge(edgeId)
      cancelActive('escape')
      setHoverEntity(null)
      requestLayout()
    },
  )
}
