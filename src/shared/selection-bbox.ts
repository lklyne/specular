/**
 * Shared bounding-box-over-selected-entities math. Multiple call sites (hit
 * testing, multi-resize, drag chrome) each union the same selected-entity
 * rects in either screen or canvas space; this is the one implementation.
 */

import type { CanvasSceneEntity } from './types'

export interface SelectionBbox {
  x: number
  y: number
  width: number
  height: number
}

/**
 * Union the bbox of the entities in `entities` whose id is in `selectedIds`,
 * in the given coordinate space. A group contributes its own stored rect —
 * it is the group's visual unit (padding included), and the box must wrap
 * the border, not just the descendants inside it. Returns null when fewer
 * than two matching entities are found (multi-bbox is a 2+-entity concept;
 * callers fall through to a single-entity path on null).
 */
export function selectionBbox(
  entities: readonly CanvasSceneEntity[],
  selectedIds: readonly string[],
  space: 'screen' | 'canvas',
): SelectionBbox | null {
  const ids = new Set(selectedIds)
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  let count = 0
  for (const entity of entities) {
    if (!ids.has(entity.id)) continue
    const x = space === 'screen' ? entity.screenX : entity.canvasX
    const y = space === 'screen' ? entity.screenY : entity.canvasY
    const width = space === 'screen' ? entity.screenWidth : entity.width
    const height = space === 'screen' ? entity.screenHeight : entity.height
    minX = Math.min(minX, x)
    minY = Math.min(minY, y)
    maxX = Math.max(maxX, x + width)
    maxY = Math.max(maxY, y + height)
    count++
  }
  if (count < 2) return null
  return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
}
