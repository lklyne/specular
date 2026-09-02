/**
 * Group-label geometry: the one definition of the label's screen-space box,
 * shared by the canvas painter (GroupLabelCanvasSurface) and the hit-tester
 * so the visible text and the grabbable target can't drift apart.
 *
 * Pure: no DOM. Text width is measured by the renderer (canvas measureText)
 * and passed in; callers without a measurer (main-side test routes) fall back
 * to a per-character estimate.
 */

import type { Rect } from './hit-regions'

export const GROUP_LABEL_FONT = '500 11px system-ui, sans-serif'
/** Matches the DOM label's inherited 1.5 line-height at 11px. */
export const GROUP_LABEL_LINE_HEIGHT = 16.5
/** The label's pb-1 gap between its line box and the group's top edge. */
export const GROUP_LABEL_BOTTOM_GAP = 4

/** Average system-ui glyph advance at 11px/500, the hit-test fallback. */
const ESTIMATED_CHAR_WIDTH = 6.1

export function estimateGroupLabelWidth(label: string): number {
  return label.length * ESTIMATED_CHAR_WIDTH
}

/** The label's hit box: line box + bottom gap, anchored above the group's
 * top-left corner (the DOM label's translateY(-100%) box). */
export function groupLabelRect(
  group: { screenX: number; screenY: number },
  labelWidth: number,
): Rect {
  const height = GROUP_LABEL_LINE_HEIGHT + GROUP_LABEL_BOTTOM_GAP
  return {
    x: group.screenX,
    y: group.screenY - height,
    width: labelWidth,
    height,
  }
}
