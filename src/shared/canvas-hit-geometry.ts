import type { EdgeSide } from './types'

export { CHROME_HEADER_HEIGHT } from './entity-chrome-slots'

export const EDGE_ANCHOR_DOT_OFFSET_PX = 8
export const EDGE_ANCHOR_HIT_ALONG_PX = 56
export const EDGE_ANCHOR_HIT_ACROSS_PX = 24
export const EDGE_ANCHOR_HIT_GAP_PX = 4
export const EDGE_ANCHOR_HIT_CORNER_PX = 2
export const EDGE_ANCHOR_HIT_MIN_SCALE = 0.35

export const RESIZE_HANDLE_VISUAL_PX = 8
export const RESIZE_HANDLE_HIT_PX = 12

// Auto-layout reorder dot (ADR 0015). Visual radius is small at rest and grows
// on hover (rendered in aboveView); the hit square stays comfortably grabbable.
export const REORDER_DOT_VISUAL_RADIUS_PX = 4
export const REORDER_DOT_HOVER_RADIUS_PX = 7
export const REORDER_HANDLE_HIT_PX = 28

export const MULTI_SELECTION_OUTLINE_PADDING_PX = 8

export const EDGE_SIDES: readonly EdgeSide[] = ['top', 'right', 'bottom', 'left']

export function scaleEdgeAnchorHitSize(basePx: number, zoom: number): number {
  const scale = Math.max(EDGE_ANCHOR_HIT_MIN_SCALE, Math.min(zoom, 1))
  return basePx * scale
}
