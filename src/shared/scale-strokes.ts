/**
 * Pure helpers for resizing drawing stroke geometry.
 *
 * Drawing stroke points are stored in absolute canvas coordinates. Resizing a
 * drawing must therefore transform each point from the old entity bounds into
 * the new entity bounds rather than multiplying raw coordinates from (0, 0).
 */

import type { AnnotationDrawingStroke } from './types'

export interface DrawingResizeBounds {
  canvasX: number
  canvasY: number
  width: number
  height: number
}

export function scaleStrokesToBounds(
  strokes: AnnotationDrawingStroke[],
  from: DrawingResizeBounds,
  to: DrawingResizeBounds,
): AnnotationDrawingStroke[] {
  const scaleX = from.width > 0 ? to.width / from.width : 1
  const scaleY = from.height > 0 ? to.height / from.height : 1
  return strokes.map((stroke) => ({
    ...stroke,
    points: stroke.points.map(({ x, y }) => ({
      x: to.canvasX + (x - from.canvasX) * scaleX,
      y: to.canvasY + (y - from.canvasY) * scaleY,
    })),
  }))
}
