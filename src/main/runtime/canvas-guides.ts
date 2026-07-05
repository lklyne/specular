import { ipcChannels } from '../../shared/ipc-contract'
import type { CanvasGuidesPayload } from '../../shared/canvas-guides'
import { safeSend } from './safe-send'
import { aboveView } from './view-refs'

const EMPTY_GUIDES: CanvasGuidesPayload = { alignmentGuides: [], distributionGuides: [] }
let lastCanvasGuides: CanvasGuidesPayload = EMPTY_GUIDES

export function broadcastCanvasGuides(payload: CanvasGuidesPayload): void {
  lastCanvasGuides = payload
  if (!aboveView || aboveView.webContents.isDestroyed()) return
  safeSend(aboveView.webContents, ipcChannels.canvasGuides, payload)
}

export function clearCanvasGuides(): void {
  broadcastCanvasGuides(EMPTY_GUIDES)
}

export function currentCanvasGuides(): CanvasGuidesPayload {
  return lastCanvasGuides
}
