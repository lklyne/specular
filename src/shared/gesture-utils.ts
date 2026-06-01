import type { CanvasSceneEntity, LayoutUpdateData } from './types'
import { GRID_SIZE } from './constants'

export {
  canvasToScreenX,
  canvasToScreenY,
  screenPointToCanvasPoint,
  screenRectToCanvasRect,
  toOverlayY,
} from './coords'

type ViewportWheelAction =
  | {
      kind: 'zoom'
      deltaY: number
      mouseX: number
      mouseY: number
    }
  | {
      kind: 'pan'
      deltaX: number
      deltaY: number
    }

export function normalizeRect(
  startX: number,
  startY: number,
  currentX: number,
  currentY: number,
) {
  const left = Math.min(startX, currentX)
  const top = Math.min(startY, currentY)
  return {
    left,
    top,
    width: Math.abs(currentX - startX),
    height: Math.abs(currentY - startY),
  }
}

export function squareConstrainedRect(
  startX: number,
  startY: number,
  currentX: number,
  currentY: number,
  constrainSquare: boolean,
) {
  if (!constrainSquare) return normalizeRect(startX, startY, currentX, currentY)
  const dx = currentX - startX
  const dy = currentY - startY
  const side = Math.max(Math.abs(dx), Math.abs(dy))
  return {
    left: dx < 0 ? startX - side : startX,
    top: dy < 0 ? startY - side : startY,
    width: side,
    height: side,
  }
}

export function snapToGrid(value: number): number {
  return Math.round(value / GRID_SIZE) * GRID_SIZE
}

export function isTypingTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLInputElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  )
}

export function isOverlayUiTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  // Resize handles live inside the selection overlay (which is tagged
  // `data-overlay-ui`) but they ARE routable — the canvas pointer router
  // hit-tests the click position to dispatch begin-resize.
  if (target.closest('[data-resize-handle]')) return false
  return Boolean(target.closest('[data-overlay-ui]'))
}

function isScrollableContainer(el: HTMLElement): boolean {
  const style = getComputedStyle(el)
  const oy = style.overflowY
  const ox = style.overflowX
  return (
    ((oy === 'auto' || oy === 'scroll') && el.scrollHeight > el.clientHeight) ||
    ((ox === 'auto' || ox === 'scroll') && el.scrollWidth > el.clientWidth)
  )
}

/**
 * True when a wheel event's target sits inside a scrollable element belonging
 * to an entity body (a markdown note, etc.). The walk stops at the entity
 * shell (`[data-entity-id]`) so it never reports the canvas surface itself.
 *
 * Used so wheeling over a note's body scrolls the note natively instead of
 * the canvas-mode wheel authority swallowing the event to pan the canvas.
 */
export function canScrollWheelTarget(target: EventTarget | null): boolean {
  let node: Element | null = target instanceof Element ? target : null
  while (node) {
    if (node instanceof HTMLElement && isScrollableContainer(node)) return true
    if (node.hasAttribute('data-entity-id')) break
    node = node.parentElement
  }
  return false
}

function hasNoModifierKeys(
  event: Pick<KeyboardEvent, 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>,
): boolean {
  return !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey
}

export function isPlainShortcutKey(
  event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>,
  key: string,
): boolean {
  return event.key.toLowerCase() === key.toLowerCase() && hasNoModifierKeys(event)
}

function isCommandShortcutKey(
  event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>,
  key: string,
): boolean {
  return (
    event.key.toLowerCase() === key.toLowerCase() &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey
  )
}

export function classifyViewportWheel(event: Pick<WheelEvent, 'metaKey' | 'ctrlKey' | 'deltaX' | 'deltaY' | 'screenX' | 'screenY'>): ViewportWheelAction {
  if (event.metaKey || event.ctrlKey) {
    return {
      kind: 'zoom',
      deltaY: event.deltaY,
      mouseX: event.screenX,
      mouseY: event.screenY,
    }
  }
  return {
    kind: 'pan',
    deltaX: event.deltaX,
    deltaY: event.deltaY,
  }
}

export function shouldStartMouseViewportPan(event: Pick<PointerEvent, 'button'>): boolean {
  return event.button === 1
}

export function middleDragDelta(
  previous: { screenX: number; screenY: number },
  next: Pick<PointerEvent, 'screenX' | 'screenY'>,
) {
  return {
    deltaX: previous.screenX - next.screenX,
    deltaY: previous.screenY - next.screenY,
  }
}

/**
 * Entities whose screen-space bounding box overlaps `rect`. Used by the
 * marquee gesture to publish a "would-be selected" preview each pointermove.
 * Touch-only intersection (>= edge equality) is excluded; matches the old
 * marquee preview hook exactly.
 */
export function entitiesOverlappingRect(
  entities: readonly CanvasSceneEntity[],
  rect: { left: number; top: number; width: number; height: number },
): string[] {
  const ids: string[] = []
  const right = rect.left + rect.width
  const bottom = rect.top + rect.height
  for (const entity of entities) {
    if (
      rect.left < entity.screenX + entity.screenWidth &&
      right > entity.screenX &&
      rect.top < entity.screenY + entity.screenHeight &&
      bottom > entity.screenY
    ) {
      ids.push(entity.id)
    }
  }
  return ids
}
