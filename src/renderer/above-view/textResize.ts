/**
 * Text resize policy — what a handle drag means for a text entity.
 *
 * A text entity stores one geometry axis: width. Its height is a content-sized
 * bound (CONTEXT.md) whose sole writer is the measurement in StickyBodyLayer,
 * so every patch here drops `height`. A second writer would let the body and
 * the selection outline disagree for as long as it took the next measurement
 * to land.
 *
 * What the surviving axis does depends on the handle:
 *   - side handles (e/w) *reflow*: width changes, the text keeps its size and
 *     rewraps, so a wider body usually ends up shorter.
 *   - corner and n/s handles *scale*: the font tracks the width ratio from
 *     gesture start (FigJam-style), and the height follows the resized text —
 *     so a vertical drag fills the box the drag asked for. Aspect is locked
 *     for those handles so a vertical drag drives width too.
 */

import type { ResizeHandle } from '../../shared/resize-accumulator'
import { clampTextSize } from './TextSizeDropdown'

export type TextResizeStart = { width: number; textSize: number }

type BoundsPatch = { width: number; height: number; canvasX?: number; canvasY?: number }

export function textResizePatch(
  handle: ResizeHandle,
  start: TextResizeStart,
  patch: BoundsPatch,
): { width: number; canvasX?: number; canvasY?: number; textSize?: number } {
  const { height: _contentSized, ...bounds } = patch
  if (handle === 'e' || handle === 'w') return bounds
  return {
    ...bounds,
    textSize: clampTextSize((start.textSize * patch.width) / start.width),
  }
}
