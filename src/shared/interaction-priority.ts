/**
 * Hit-test layer priority. Top wins.
 *
 * See docs/adr/0001-click-to-enter-page-focus.md for the load-bearing
 * constraints encoded here:
 *   - Resize handles above anchors: once selected, the next gesture is shaping.
 *   - Body kind-dispatches: same priority slot, behavior chosen by kind.
 */

export type HitLayer =
  | 'resize-handles'
  | 'anchors'
  | 'reorder-handle'
  | 'gap-handle'
  | 'body'
  | 'background'

export const HIT_LAYER_ORDER: readonly HitLayer[] = [
  'resize-handles',
  'anchors',
  // Auto-layout reorder dots sit above the child body so dragging the dot
  // reorders, while dragging the body still moves the whole group (ADR 0015 D4).
  // Below anchors/chrome/handles, which own the entity's edges.
  'reorder-handle',
  // Auto-layout gap strips (between adjacent managed children) sit below the
  // reorder dot: the dot is a small center target, the gap strip is the space
  // between — where they'd overlap (tiny children), the dot wins.
  'gap-handle',
  'body',
  'background',
] as const

