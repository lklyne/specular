import type { EdgeSide } from './types'

export const EDGE_ANCHOR_DOT_OFFSET_PX = 8
export const EDGE_ANCHOR_HIT_ALONG_PX = 68
export const EDGE_ANCHOR_HIT_ACROSS_PX = 32
export const EDGE_ANCHOR_HIT_GAP_PX = 4
export const EDGE_ANCHOR_HIT_CORNER_PX = 2
export const EDGE_ANCHOR_HIT_MIN_SCALE = 0.35

export const RESIZE_HANDLE_VISUAL_PX = 8
export const RESIZE_HANDLE_HIT_PX = 12

// Auto-layout reorder dot (ADR 0015). Visual radius is small at rest and grows
// on hover (rendered in aboveView); the hit square stays comfortably grabbable.
export const REORDER_DOT_VISUAL_RADIUS_PX = 4
export const REORDER_DOT_HOVER_RADIUS_PX = 7
export const REORDER_HANDLE_HIT_PX = REORDER_DOT_HOVER_RADIUS_PX * 2
export const REORDER_HANDLE_MAX_ENTITY_FRACTION = 0.4

/**
 * Screen-space size of a reorder dot's grabbable square — the hovered circle,
 * exactly. Capped at a fraction of the entity's own screen box so a zoomed-out
 * item is never swallowed whole by its center handle: the body must stay
 * draggable to move the group.
 */
export function reorderHandleHitPx(screenWidth: number, screenHeight: number): number {
  const shortest = Math.min(screenWidth, screenHeight)
  return Math.min(REORDER_HANDLE_HIT_PX, shortest * REORDER_HANDLE_MAX_ENTITY_FRACTION)
}

// Auto-layout gap handle (ADR 0015 Milestone 2). The hit/paint strip spans the
// gap between adjacent managed children; at small zoom (or gap 0) it expands to
// this minimum thickness, centered on the seam, so it stays grabbable.
export const GAP_HANDLE_MIN_HIT_PX = 10

export const MULTI_SELECTION_OUTLINE_PADDING_PX = 8

export const EDGE_SIDES: readonly EdgeSide[] = ['top', 'right', 'bottom', 'left']

export function scaleEdgeAnchorHitSize(basePx: number, zoom: number): number {
  const scale = Math.max(EDGE_ANCHOR_HIT_MIN_SCALE, Math.min(zoom, 1))
  return basePx * scale
}
