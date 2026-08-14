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
 *   - We subtract the page WCV's *actual placed bounds* (`pageView.getBounds()`),
 *     which is the single source of truth the layout pass set. Deriving the
 *     origin independently (e.g. via the camera transform) drifts from where
 *     the WCV is really painted in focus/fill mode, offsetting every click.
 */

import { findPageById } from './runtime-context'

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
  // The WCV's own bounds are the source of truth for where its content paints,
  // in the same window coordinate space as windowX/windowY. This tracks the
  // layout pass across every mode (normal, fit/device focus, and fill focus —
  // which pins the WCV to focusFillRegion() rather than the camera transform).
  const bounds = page.pageView.getBounds()
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

// CDP modifier bitmask: Alt=1, Ctrl=2, Meta=4, Shift=8.
function cdpModifiersFor(payload: {
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}): number {
  return (
    (payload.altKey ? 1 : 0) |
    (payload.ctrlKey ? 2 : 0) |
    (payload.metaKey ? 4 : 0) |
    (payload.shiftKey ? 8 : 0)
  )
}

// Which button a mouseMoved reports: the held one (for drags), else none.
function moveButtonFor(buttons: number): 'left' | 'right' | 'middle' | 'none' {
  if (buttons & 1) return 'left'
  if (buttons & 2) return 'right'
  if (buttons & 4) return 'middle'
  return 'none'
}

/** Attaching throws when a DevTools window already owns the page's debugger.
 * Callers fall back to sendInputEvent there: that loses cross-frame routing,
 * but the main frame stays clickable, which beats swallowing the event. */
function ensureCdpAttached(wc: Electron.WebContents): boolean {
  if (wc.debugger.isAttached()) return true
  try {
    wc.debugger.attach('1.3')
    return true
  } catch {
    return false
  }
}

export function forwardPointerToPage(pageId: string, payload: ForwardPointerPayload): boolean {
  const target = pageLocal(pageId)
  if (!target) return false
  const x = Math.round(payload.windowX - target.x)
  const y = Math.round(payload.windowY - target.y)
  try {
    // Dispatch via CDP rather than sendInputEvent: sendInputEvent forwards to
    // the main frame's render widget and skips Chromium's cross-frame input
    // router, so out-of-process iframes (any cross-origin iframe — Google
    // sign-in popovers, payment widgets) never receive the events. CDP
    // Input.dispatchMouseEvent hit-tests across the frame tree in the browser
    // process. Coordinates are the same pre-emulation-scale view-local space
    // both APIs speak (see app-control-server's click path).
    const wc = target.webContents
    const clickCount = payload.clickCount ?? (payload.kind === 'move' ? 0 : 1)
    if (ensureCdpAttached(wc)) {
      const buttons = payload.buttons ?? (payload.kind === 'move' ? 0 : undefined)
      void wc.debugger
        .sendCommand('Input.dispatchMouseEvent', {
          type:
            payload.kind === 'down'
              ? 'mousePressed'
              : payload.kind === 'up'
                ? 'mouseReleased'
                : 'mouseMoved',
          x,
          y,
          button: payload.kind === 'move' ? moveButtonFor(buttons ?? 0) : payload.button,
          ...(buttons !== undefined ? { buttons } : {}),
          clickCount,
          modifiers: cdpModifiersFor(payload),
        })
        .catch((error) => {
          console.error('[page-input-forwarding] pointer forward failed', error)
        })
    } else {
      wc.sendInputEvent({
        type:
          payload.kind === 'down' ? 'mouseDown' : payload.kind === 'up' ? 'mouseUp' : 'mouseMove',
        x,
        y,
        button: payload.button,
        clickCount,
        modifiers: modifiersFor(payload),
      })
    }
    // Dispatching synthesizes the click but does NOT focus the webContents,
    // so the resulting text selection renders with Chromium's inactive (gray)
    // highlight. Focus on mouseDown the way a real click would, so selection is
    // active immediately — matches what runForwardPointer already assumes.
    if (payload.kind === 'down') wc.focus()
  } catch (error) {
    console.error('[page-input-forwarding] pointer forward threw', error)
    return false
  }
  return true
}
