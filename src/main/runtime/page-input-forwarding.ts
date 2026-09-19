/**
 * Page input forwarding — translate window-space pointer/wheel events from
 * aboveView into Electron `sendInputEvent` calls on the target page's
 * webContents, and DOM key events into CDP `Input` calls. Pages render
 * offscreen, so aboveView is the only surface that receives OS input.
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
 *   - The page's own viewport is CSS px at its authored (or focus-session)
 *     size, so the window-space point is translated by the projected rect's
 *     origin and then divided by the on-screen scale that rect implies. A
 *     page paints offscreen at its CSS size, so its rect on screen is the only
 *     statement of that scale.
 */

import { findPageById } from './runtime-context'
import { boundEffectivePageContentSize, boundScreenBoundsForPage } from './runtime-geometry'
import { ensurePageDebugger } from './page-debugger'
import { ensurePageFocusEmulated } from './page-focus-emulation'
import { noteInputToPage } from './page-input-counter'
import { cdpKeyEventParams, type ForwardKeyPayload } from '../../shared/page-key-input'

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

interface PageLocalFrame {
  rect: { x: number; y: number; width: number; height: number }
  size: { width: number; height: number }
  webContents: Electron.WebContents
}

function pageLocal(pageId: string): PageLocalFrame | null {
  const page = findPageById(pageId)
  if (!page) return null
  const wc = page.host.webContents
  if (wc.isDestroyed()) return null
  const rect = boundScreenBoundsForPage(page).page
  if (rect.width <= 0 || rect.height <= 0) return null
  const size = boundEffectivePageContentSize(page)
  if (size.width <= 0 || size.height <= 0) return null
  return { rect, size, webContents: wc }
}

/** Window-space point → the page's own CSS viewport point. */
function toPagePoint(
  target: PageLocalFrame,
  windowX: number,
  windowY: number,
): { x: number; y: number } {
  return {
    x: Math.round((windowX - target.rect.x) * (target.size.width / target.rect.width)),
    y: Math.round((windowY - target.rect.y) * (target.size.height / target.rect.height)),
  }
}

export function forwardWheelToPage(pageId: string, payload: ForwardWheelPayload): boolean {
  const target = pageLocal(pageId)
  if (!target) return false
  const { x, y } = toPagePoint(target, payload.windowX, payload.windowY)
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
    noteInputToPage(pageId)
  } catch (error) {
    console.error('[page-input-forwarding] wheel forward threw', error)
    return false
  }
  return true
}

export function forwardPointerToPage(pageId: string, payload: ForwardPointerPayload): boolean {
  const target = pageLocal(pageId)
  if (!target) return false
  const { x, y } = toPagePoint(target, payload.windowX, payload.windowY)
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
    // sendInputEvent synthesizes the click but leaves the page believing it is
    // unfocused, so the resulting text selection renders with Chromium's
    // inactive (gray) highlight. Emulate focus on mouseDown the way a real
    // click would, ahead of the layout pass that would otherwise do it.
    // A move is not something that dismisses a popup, so it is not input as
    // far as the popup-close inference is concerned.
    if (payload.kind !== 'move') noteInputToPage(pageId)
    if (payload.kind === 'down') ensurePageFocusEmulated(pageId)
  } catch (error) {
    console.error('[page-input-forwarding] pointer forward threw', error)
    return false
  }
  return true
}

/**
 * Pages whose CDP key dispatch has already failed once. A stuck session would
 * otherwise log a line per keystroke.
 */
const loggedKeyFailures = new Set<string>()

/**
 * Input dispatch installs no override, so a detached session leaves nothing to
 * re-apply. One shared handler rather than a closure per call, which would
 * grow the session's detach-handler set by one per keystroke.
 */
const NOTHING_TO_REAPPLY = (): void => {}

function pageCdp(pageId: string, method: string, params: Record<string, unknown>): boolean {
  const page = findPageById(pageId)
  if (!page) return false
  const wc = page.host.webContents
  if (wc.isDestroyed()) return false
  if (!ensurePageDebugger(wc, NOTHING_TO_REAPPLY)) return false
  wc.debugger.sendCommand(method, params).catch((error: unknown) => {
    if (loggedKeyFailures.has(pageId)) return
    loggedKeyFailures.add(pageId)
    console.error(`[page-input-forwarding] ${method} failed`, error)
  })
  return true
}

/**
 * One key event into a page. CDP rather than `sendInputEvent` because an
 * offscreen page's input path never routes a native key event, and
 * `Input.dispatchKeyEvent` is the transport that reaches its renderer.
 */
export function forwardKeyToPage(pageId: string, payload: ForwardKeyPayload): boolean {
  const sent = pageCdp(pageId, 'Input.dispatchKeyEvent', cdpKeyEventParams(payload))
  // Both halves of the press count. A dismissal has to leave the count higher
  // than the popup's own last paint, and a popup that repaints on the way out
  // (Escape's press redrawing it before it goes) would otherwise match it.
  if (sent) noteInputToPage(pageId)
  return sent
}

/** IME commits arrive as whole strings; `Input.insertText` is the only lever. */
export function insertTextIntoPage(pageId: string, text: string): boolean {
  if (!text) return false
  const sent = pageCdp(pageId, 'Input.insertText', { text })
  if (sent) noteInputToPage(pageId)
  return sent
}
