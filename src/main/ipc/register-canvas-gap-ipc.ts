import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type { CancelReason } from '../../shared/interaction-types'
import {
  cancelGapGesture,
  commitGapGesture,
  moveGapGesture,
  startGapGesture,
} from '../gap-gesture'
import {
  cancelEdgeRoutingGesture,
  commitEdgeRoutingGesture,
  moveEdgeRoutingGesture,
  startEdgeRoutingGesture,
} from '../edge-routing-gesture'
import type { EdgeSplitAxis } from '../../shared/types'

/**
 * IPC for the two crossbar-shaped drags: the gap-resize gesture (ADR 0015
 * Milestone 2) and the elbow edge's routing split. Thin wrappers over their
 * coordinators; the renderer dispatches start → move* → commit | cancel.
 *
 * `start` enters the interaction mode before any mutation (gesture-begin
 * ordering — see runtime/CLAUDE.md), so the first `requestLayout` reconciles
 * focus against `resizing-gap` rather than `idle`.
 */
export function registerCanvasGapIpc(): void {
  ipcMain.on(
    ipcChannels.canvasGapResizeStart,
    (_event, { groupId, canvasX, canvasY }: { groupId: string | null; canvasX: number; canvasY: number }) => {
      startGapGesture(groupId, canvasX, canvasY)
    },
  )

  ipcMain.on(
    ipcChannels.canvasGapResizeMove,
    (_event, { canvasX, canvasY }: { canvasX: number; canvasY: number }) => {
      moveGapGesture(canvasX, canvasY)
    },
  )

  ipcMain.on(ipcChannels.canvasGapResizeCommit, () => {
    commitGapGesture()
  })

  ipcMain.on(ipcChannels.canvasGapResizeCancel, (_event, { reason }: { reason?: CancelReason } = {}) => {
    cancelGapGesture(reason ?? 'external')
  })

  // Elbow crossbar drag — same start/move/commit/cancel shape as the gap
  // gesture above, and the same gesture-begin ordering.
  ipcMain.on(
    ipcChannels.canvasEdgeRoutingStart,
    (_event, { edgeId, split, axis }: { edgeId: string; split: number; axis: EdgeSplitAxis }) => {
      startEdgeRoutingGesture(edgeId, split, axis)
    },
  )

  ipcMain.on(ipcChannels.canvasEdgeRoutingMove, (_event, { split }: { split: number }) => {
    moveEdgeRoutingGesture(split)
  })

  ipcMain.on(ipcChannels.canvasEdgeRoutingCommit, () => {
    commitEdgeRoutingGesture()
  })

  ipcMain.on(
    ipcChannels.canvasEdgeRoutingCancel,
    (_event, { reason }: { reason?: CancelReason } = {}) => {
      cancelEdgeRoutingGesture(reason ?? 'external')
    },
  )
}
