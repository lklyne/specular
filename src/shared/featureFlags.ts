import type { CanvasEntityKind } from './types'

export const DRAWING_FEATURE_ENABLED = true
export const PERFECT_FREEHAND_ENABLED = true

/**
 * Focus presentation page menu layout.
 *
 * `false` — legacy edge-to-edge strip pinned to the viewport top.
 * `true`  — inset the strip by 8px and keep rounded floating corners.
 */
export const FOCUS_PRESENTATION_MENU_INSET = false
export const FOCUS_PRESENTATION_MENU_EDGE_INSET_PX = 8

/**
 * Region marquee selection mode.
 *
 * `true`  — only elements *fully contained* in the marquee are highlighted
 *           and grabbed (default; matches Figma / most design tools).
 * `false` — any element whose bbox *intersects* the marquee qualifies
 *           (legacy behavior).
 *
 * Internal A/B switch — not user-facing. Read by the page-paints region
 * preview overlay (preload/comment-hover-overlay.ts) and the commit-time
 * element extractor (preload/page-content.ts → `query-elements-in-rect`).
 */
export const REGION_SELECT_FULL_CONTAINMENT = true

export function isCanvasEntityKindEnabled(kind: CanvasEntityKind): boolean {
  if (kind === 'drawing') return DRAWING_FEATURE_ENABLED
  return true
}
