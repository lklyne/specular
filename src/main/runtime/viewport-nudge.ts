import type { ViewportNudge } from '../../shared/types'
import { aboveView, bgView } from './view-refs'
import { pan, zoom } from './runtime-context'
import { safeSend } from './safe-send'

/**
 * Push the authoritative viewport to the canvas overlay renderers immediately,
 * bypassing the debounced layout pass. The canvas-bg and above-view scene
 * layers translate by (livePan − payloadPan) so selection chrome and entity
 * bodies track the natively-positioned page views during a pan, instead of
 * trailing until the next full layout-update rebuild lands (which scales with
 * entity count). The eventual layout-update reconciles the offset back to zero.
 * See #257.
 */
export function broadcastViewportNudge(): void {
  if (!bgView && !aboveView) return
  const payload: ViewportNudge = { pan: { x: pan.x, y: pan.y }, zoom }
  if (bgView) safeSend(bgView.webContents, 'viewport-nudge', payload)
  if (aboveView) safeSend(aboveView.webContents, 'viewport-nudge', payload)
}
