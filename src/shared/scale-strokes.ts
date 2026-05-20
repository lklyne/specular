/**
 * Pure helper — scale drawing stroke point coordinates by (scaleX, scaleY).
 * Brush width is passed through unchanged; only the curve geometry scales.
 */

import type { AnnotationDrawingStroke } from './types'

export function scaleStrokes(
  strokes: AnnotationDrawingStroke[],
  scaleX: number,
  scaleY: number,
): AnnotationDrawingStroke[] {
  return strokes.map((stroke) => ({
    ...stroke,
    points: stroke.points.map(({ x, y }) => ({ x: x * scaleX, y: y * scaleY })),
  }))
}
