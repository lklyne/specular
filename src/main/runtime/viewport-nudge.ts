import type { ViewportNudge } from '../../shared/types'
import { aboveView, bgView, cursorOverlayWindow } from './view-refs'
import { pan, zoom } from './runtime-context'
import { safeSend } from './safe-send'

/**
 * Push the authoritative viewport to the canvas overlay renderers immediately,
 * bypassing the debounced layout pass. Each renderer reprojects its scene
 * transform from (payloadCamera → liveCamera) so selection chrome, entity
 * bodies, and presence cursors track the natively-positioned page views during
 * pan/zoom, instead of trailing until the next full layout-update rebuild lands
 * (which scales with entity count). Under ADR 0023 Phase 1 pan/zoom stop
 * rebuilding the scene at all, so this nudge is the sole live signal; the
 * transform reconciles to identity when a content-change payload next lands.
 * See #257, ADR 0023.
 */
export function broadcastViewportNudge(): void {
  if (!bgView && !aboveView && !cursorOverlayWindow) return
  const payload: ViewportNudge = { pan: { x: pan.x, y: pan.y }, zoom }
  if (bgView) safeSend(bgView.webContents, 'viewport-nudge', payload)
  if (aboveView) safeSend(aboveView.webContents, 'viewport-nudge', payload)
  if (cursorOverlayWindow && !cursorOverlayWindow.isDestroyed()) {
    safeSend(cursorOverlayWindow.webContents, 'viewport-nudge', payload)
  }
}
