import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type { CancelReason } from '../../shared/interaction-types'
import {
  cancelGapGesture,
  commitGapGesture,
  moveGapGesture,
  startGapGesture,
} from '../gap-gesture'

/**
 * IPC for the gap-resize gesture (ADR 0015 Milestone 2). Thin
 * wrappers over the gap-gesture coordinator; the renderer's `runGapDrag`
 * dispatches start → move* → commit | cancel.
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
}
