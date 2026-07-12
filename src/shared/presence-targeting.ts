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
 * suppressed because the cursor is already on the target — the point sits
 * inside `targetRect`. Shared by the CDP proxy's box-model pre-move (issue
 * #318 amortization) and its mousePressed skip check (ADR 0029) so both agree
 * on one definition of "already there, don't re-travel."
 *
 * Only within-rect counts: a distance heuristic was measured in canvas units,
 * so it suppressed legitimate travel more aggressively the further the canvas
 * was zoomed out (#319). Every hop to a different element now animates.
 *
 * Event-shaped (a point and a rect, not a `PresenceCursorEntry` read) so
 * #319 Phase 5's event-timeline choreography can port it unchanged (ADR
 * 0029, "Future path: presence event timeline").
 */
export function shouldSkipReposition({
  clickPoint,
  targetRect,
}: {
  clickPoint: { x: number | null | undefined; y: number | null | undefined }
  targetRect: PresenceTargetRect | null | undefined
}): boolean {
  return targetRect != null && pagePointMatchesTargetRect(clickPoint.x, clickPoint.y, targetRect, 0)
}
