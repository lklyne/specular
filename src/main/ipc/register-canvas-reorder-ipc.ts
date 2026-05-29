import { ipcMain } from 'electron'
import type { CancelReason } from '../../shared/interaction-types'
import {
  cancelReorderGesture,
  commitReorderGesture,
  moveReorderGesture,
  startReorderGesture,
} from '../reorder-gesture'

/**
 * IPC for the auto-layout reorder gesture (ADR 0015 Phase 3). Thin wrappers over
 * the reorder-gesture coordinator; the renderer's `runReorderDrag` dispatches
 * start → move* → commit | cancel.
 *
 * `start` enters the interaction mode before any mutation (gesture-begin
 * ordering — see runtime/CLAUDE.md), so the first `requestLayout` reconciles
 * focus against `reordering-child` rather than `idle`.
 */
export function registerCanvasReorderIpc(): void {
  ipcMain.on(
    'canvas-reorder-child-start',
    (_event, { childId, groupId }: { childId: string; groupId: string }) => {
      startReorderGesture(childId, groupId)
    },
  )

  ipcMain.on(
    'canvas-reorder-child-move',
    (_event, { canvasX }: { canvasX: number; canvasY: number }) => {
      moveReorderGesture(canvasX)
    },
  )

  ipcMain.on('canvas-reorder-child-commit', () => {
    commitReorderGesture()
  })

  ipcMain.on('canvas-reorder-child-cancel', (_event, { reason }: { reason?: CancelReason } = {}) => {
    cancelReorderGesture(reason ?? 'external')
  })
}
