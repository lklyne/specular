import type { PresenceTargetRect } from './types'

export function resolvePresencePagePoint(input: {
  pageX?: number | null
  pageY?: number | null
  targetRect?: PresenceTargetRect | null
  fallbackX: number
  fallbackY: number
}): { x: number; y: number } {
  const targetCenter = input.targetRect
    ? {
        x: input.targetRect.x + input.targetRect.width / 2,
        y: input.targetRect.y + input.targetRect.height / 2,
      }
    : null

  return {
    x:
      typeof input.pageX === 'number'
        ? input.pageX
        : targetCenter?.x ?? input.fallbackX,
    y:
      typeof input.pageY === 'number'
        ? input.pageY
        : targetCenter?.y ?? input.fallbackY,
  }
}

export function pagePointMatchesTargetRect(
  pageX: number | null | undefined,
  pageY: number | null | undefined,
  targetRect: PresenceTargetRect | null | undefined,
  tolerance = 2,
): boolean {
  if (!targetRect || typeof pageX !== 'number' || typeof pageY !== 'number') return true
  return (
    pageX >= targetRect.x - tolerance &&
    pageX <= targetRect.x + targetRect.width + tolerance &&
    pageY >= targetRect.y - tolerance &&
    pageY <= targetRect.y + targetRect.height + tolerance
  )
}

/**
 * Whether a presence cursor reposition toward a click/target point should be
 * suppressed because the cursor is already close enough — either the point
 * already sits inside `targetRect`, or the canvas-space gap between the
 * cursor's current position and the point's resolved canvas position is
 * under `skipDistancePx`. Shared by the CDP proxy's box-model pre-move
 * (issue #318 amortization) and its mousePressed skip check (ADR 0029) so
 * both agree on one definition of "close enough not to re-travel."
 *
 * Event-shaped (points and a rect, not a `PresenceCursorEntry` read) so
 * #319 Phase 5's event-timeline choreography can port it unchanged (ADR
 * 0029, "Future path: presence event timeline").
 */
export function shouldSkipReposition({
  clickPoint,
  targetRect,
  cursorCanvasPoint,
  clickCanvasPoint,
  skipDistancePx,
}: {
  clickPoint: { x: number | null | undefined; y: number | null | undefined }
  targetRect: PresenceTargetRect | null | undefined
  cursorCanvasPoint: { x: number; y: number }
  clickCanvasPoint: { x: number; y: number }
  skipDistancePx: number
}): boolean {
  const withinRect =
    targetRect != null && pagePointMatchesTargetRect(clickPoint.x, clickPoint.y, targetRect, 0)
  const canvasDistance = Math.hypot(
    clickCanvasPoint.x - cursorCanvasPoint.x,
    clickCanvasPoint.y - cursorCanvasPoint.y,
  )
  return withinRect || canvasDistance < skipDistancePx
}
