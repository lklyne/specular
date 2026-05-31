import { ipcMain } from 'electron'
import type { CancelReason } from '../../shared/interaction-types'
import {
  cancelReorderGesture,
  commitReorderGesture,
  moveReorderGesture,
  startReorderGesture,
} from '../reorder-gesture'

/**
 * IPC for the row reorder gesture (ADR 0015 D7). Thin wrappers over the
 * reorder-gesture coordinator; the renderer's `runReorderDrag` dispatches
 * start → move* → commit | cancel. The begin carries only `movingId` — main
 * resolves which door (selection / managed) armed the gesture.
 *
 * `start` enters the interaction mode before any mutation (gesture-begin
 * ordering — see runtime/CLAUDE.md), so the first `requestLayout` reconciles
 * focus against `reordering-row` rather than `idle`.
 */
export function registerCanvasReorderIpc(): void {
  ipcMain.on('canvas-reorder-start', (_event, { movingId }: { movingId: string }) => {
    startReorderGesture(movingId)
  })

  ipcMain.on(
    'canvas-reorder-move',
    (_event, { canvasX, canvasY }: { canvasX: number; canvasY: number }) => {
      moveReorderGesture(canvasX, canvasY)
    },
  )

  ipcMain.on('canvas-reorder-commit', () => {
    commitReorderGesture()
  })

  ipcMain.on('canvas-reorder-cancel', (_event, { reason }: { reason?: CancelReason } = {}) => {
    cancelReorderGesture(reason ?? 'external')
  })
}
