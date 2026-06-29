/**
 * Page input forwarding — translate window-space pointer/wheel events from
 * aboveView into Electron `sendInputEvent` calls on the target page's page
 * webContents. PoC for the "aboveView is the always-visible interactive
 * layer" endpoint (docs/plans/aboveview-interactive-layer-poc.md).
 *
 * Pure plumbing: caller gives us window-space coords (the same coordinate
 * page the canvas-pointer-router already speaks); we resolve the target
 * page, subtract its content-rect origin, and dispatch the synthesized
 * Chromium input event.
 *
 * Coordinate space:
 *   - Renderer event.clientX is window-X.
 *   - aboveView's WCV starts at canvasOrigin.y, so the renderer adds that
 *     before calling us → windowY is window-Y.
 *   - The page WCV's content rect is `screenBoundsForPage(page).page` in
 *     the same window coordinate space, so page-local = window − bounds.page.
 */

import { findPageById } from './runtime-context'
import { boundScreenBoundsForPage as screenBoundsForPage } from './runtime-geometry'

export type ForwardWheelPayload = {
  windowX: number
  windowY: number
  deltaX: number
  deltaY: number
  /** Trackpad pixel-precise vs mouse-wheel ticks. */
  hasPreciseScrollingDeltas: boolean
  /** Continuous events (`canScroll`) vs pinch (`!canScroll`). */
  canScroll: boolean
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}

export type ForwardPointerKind = 'down' | 'up' | 'move'
export type ForwardPointerButton = 'left' | 'middle' | 'right'

export type ForwardPointerPayload = {
  kind: ForwardPointerKind
  windowX: number
  windowY: number
  button: ForwardPointerButton
  /** Active button mask while moving (matches Electron's `globalX/Y` siblings). */
  buttons?: number
  clickCount?: number
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}

type Modifier = 'shift' | 'control' | 'alt' | 'meta'

function modifiersFor(payload: {
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}): Modifier[] {
  const out: Modifier[] = []
  if (payload.shiftKey) out.push('shift')
  if (payload.ctrlKey) out.push('control')
  if (payload.altKey) out.push('alt')
  if (payload.metaKey) out.push('meta')
  return out
}

function pageLocal(pageId: string): {
  x: number
  y: number
  webContents: Electron.WebContents
} | null {
  const page = findPageById(pageId)
  if (!page) return null
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return null
  const bounds = screenBoundsForPage(page).page
  return { x: bounds.x, y: bounds.y, webContents: wc }
}

export function forwardWheelToPage(pageId: string, payload: ForwardWheelPayload): boolean {
  const target = pageLocal(pageId)
  if (!target) return false
  const x = Math.round(payload.windowX - target.x)
  const y = Math.round(payload.windowY - target.y)
  // Out-of-bounds coords still scroll the document root in practice, but the
  // router gates this on a page-body hit so we'll be inside the rect anyway.
  try {
    const wheelEvent: Electron.MouseWheelInputEvent = {
      type: 'mouseWheel',
      x,
      y,
      deltaX: -payload.deltaX,
      deltaY: -payload.deltaY,
      // wheelTicks: empirically required for line-mode mouse wheels to
      // scroll. For trackpads (precise deltas) Chromium ignores it.
      wheelTicksX: payload.hasPreciseScrollingDeltas ? 0 : -payload.deltaX,
      wheelTicksY: payload.hasPreciseScrollingDeltas ? 0 : -payload.deltaY,
      hasPreciseScrollingDeltas: payload.hasPreciseScrollingDeltas,
      canScroll: payload.canScroll,
      modifiers: modifiersFor(payload),
    }
    target.webContents.sendInputEvent(wheelEvent)
  } catch (error) {
    console.error('[page-input-forwarding] wheel forward threw', error)
    return false
  }
  return true
}

export function forwardPointerToPage(pageId: string, payload: ForwardPointerPayload): boolean {
  const target = pageLocal(pageId)
  if (!target) return false
  const x = Math.round(payload.windowX - target.x)
  const y = Math.round(payload.windowY - target.y)
  const eventType =
    payload.kind === 'down' ? 'mouseDown' : payload.kind === 'up' ? 'mouseUp' : 'mouseMove'
  try {
    const pointerEvent: Electron.MouseInputEvent = {
      type: eventType,
      x,
      y,
      button: payload.button,
      clickCount: payload.clickCount ?? (payload.kind === 'move' ? 0 : 1),
      modifiers: modifiersFor(payload),
    }
    target.webContents.sendInputEvent(pointerEvent)
    // sendInputEvent synthesizes the click but does NOT focus the webContents,
    // so the resulting text selection renders with Chromium's inactive (gray)
    // highlight. Focus on mouseDown the way a real click would, so selection is
    // active immediately — matches what runForwardPointer already assumes.
    if (payload.kind === 'down') target.webContents.focus()
  } catch (error) {
    console.error('[page-input-forwarding] pointer forward threw', error)
    return false
  }
  return true
}
