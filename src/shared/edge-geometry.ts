/**
 * Pure edge geometry — anchor points, bezier control offsets, and auto-side
 * selection shared by the edge-drag controller (window-space) and EdgeLayer
 * (aboveView-local space). `originY` shifts the y-origin: 0 for window-space
 * callers, `canvasOrigin.y` for aboveView.
 */

import { EDGE_ANCHOR_DOT_OFFSET_PX } from './canvas-hit-geometry'
import type { CanvasSceneEntity, EdgeSide } from './types'

export const CONTROL_POINT_MIN = 40
export const CONTROL_POINT_MAX = 200

export interface AnchorPoint {
  x: number
  y: number
  side: EdgeSide
}

export function getAnchorPoint(
  entity: CanvasSceneEntity,
  side: EdgeSide,
  zoom: number,
  originY = 0,
): AnchorPoint {
  const { screenX, screenY, screenWidth, screenHeight } = entity
  const localY = screenY - originY
  const dotOffset = EDGE_ANCHOR_DOT_OFFSET_PX * zoom
  switch (side) {
    case 'top':
      return { x: screenX + screenWidth / 2, y: localY - dotOffset, side }
    case 'bottom':
      return { x: screenX + screenWidth / 2, y: localY + screenHeight + dotOffset, side }
    case 'left':
      return { x: screenX - dotOffset, y: localY + screenHeight / 2, side }
    case 'right':
      return { x: screenX + screenWidth + dotOffset, y: localY + screenHeight / 2, side }
  }
}

export function controlPointOffset(
  side: EdgeSide,
  distance: number,
  zoom: number,
): { dx: number; dy: number } {
  const offset = Math.min(
    Math.max(distance * 0.4, CONTROL_POINT_MIN * zoom),
    CONTROL_POINT_MAX * zoom,
  )
  switch (side) {
    case 'top':
      return { dx: 0, dy: -offset }
    case 'bottom':
      return { dx: 0, dy: offset }
    case 'left':
      return { dx: -offset, dy: 0 }
    case 'right':
      return { dx: offset, dy: 0 }
  }
}

export function buildBezierPath(from: AnchorPoint, to: AnchorPoint, zoom: number): string {
  const dist = Math.hypot(to.x - from.x, to.y - from.y)
  const cp1 = controlPointOffset(from.side, dist, zoom)
  const cp2 = controlPointOffset(to.side, dist, zoom)
  return `M ${from.x} ${from.y} C ${from.x + cp1.dx} ${from.y + cp1.dy}, ${to.x + cp2.dx} ${to.y + cp2.dy}, ${to.x} ${to.y}`
}

/** Pick the best sides to connect two entities when sides aren't specified. */
export function autoSides(
  from: CanvasSceneEntity,
  to: CanvasSceneEntity,
): { fromSide: EdgeSide; toSide: EdgeSide } {
  const fromCx = from.screenX + from.screenWidth / 2
  const fromCy = from.screenY + from.screenHeight / 2
  const toCx = to.screenX + to.screenWidth / 2
  const toCy = to.screenY + to.screenHeight / 2
  const dx = toCx - fromCx
  const dy = toCy - fromCy
  if (Math.abs(dx) > Math.abs(dy)) {
    return { fromSide: dx > 0 ? 'right' : 'left', toSide: dx > 0 ? 'left' : 'right' }
  }
  return { fromSide: dy > 0 ? 'bottom' : 'top', toSide: dy > 0 ? 'top' : 'bottom' }
}
