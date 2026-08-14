/**
 * Pure edge geometry — anchor points, bezier control offsets, and auto-side
 * selection shared by the edge-drag controller (window-space) and EdgeLayer
 * (aboveView-local space). `originY` shifts the y-origin: 0 for window-space
 * callers, `canvasOrigin.y` for aboveView.
 */

import { EDGE_ANCHOR_DOT_OFFSET_PX } from './canvas-hit-geometry'
import type { CanvasSceneEntity, EdgeSide, EdgeSplitAxis, WorkspaceEdge } from './types'

const CONTROL_POINT_MIN = 40
const CONTROL_POINT_MAX = 200
/** How far an elbow steps clear of an endpoint before turning. */
const ELBOW_STUB = 24
const ELBOW_CORNER_RADIUS = 8

export interface AnchorPoint {
  x: number
  y: number
  side: EdgeSide
}

export interface GeometryPoint {
  x: number
  y: number
}

export interface ElbowSplit {
  value: number
  axis: EdgeSplitAxis
}

export function getAnchorPoint(
  entity: CanvasSceneEntity,
  side: EdgeSide,
  zoom: number,
  originY = 0,
): AnchorPoint {
  const { screenX, screenY, screenWidth, screenHeight } = entity
  const localY = screenY - originY
  // Screen-space overlay: keep the gap constant on screen so the dot doesn't
  // collapse onto the item edge when zoomed out (matches the hit-rect gap).
  const dotOffset = EDGE_ANCHOR_DOT_OFFSET_PX
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

function controlPointOffset(
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

export function buildStraightPath(from: AnchorPoint, to: AnchorPoint): string {
  return `M ${from.x} ${from.y} L ${to.x} ${to.y}`
}

export function sceneEntityCenter(entity: CanvasSceneEntity, originY = 0): GeometryPoint {
  return {
    x: entity.screenX + entity.screenWidth / 2,
    y: entity.screenY - originY + entity.screenHeight / 2,
  }
}

/**
 * Which side of `self` faces `toward`. Resolved per endpoint against a bare
 * point so the opposing end can be an entity center, a free endpoint, or a live
 * cursor — all three want the same answer.
 */
export function autoSide(self: CanvasSceneEntity, toward: GeometryPoint, originY = 0): EdgeSide {
  return sideTowardPoint(sceneEntityCenter(self, originY), toward)
}

/**
 * Which side a bare point (a free endpoint, or a live cursor) faces toward
 * another point. Same `|dx| > |dy|` test `autoSide` uses against an entity
 * center — a free end has no rect to test against, just this.
 */
export function sideTowardPoint(from: GeometryPoint, toward: GeometryPoint): EdgeSide {
  const dx = toward.x - from.x
  const dy = toward.y - from.y
  if (Math.abs(dx) > Math.abs(dy)) return dx > 0 ? 'right' : 'left'
  return dy > 0 ? 'bottom' : 'top'
}

/** Anchor point for a free endpoint — the point itself, no entity dot offset. */
export function freeAnchorPoint(
  point: GeometryPoint,
  side: EdgeSide,
  originY = 0,
): AnchorPoint {
  return { x: point.x, y: point.y - originY, side }
}

/**
 * Resolve both endpoints' sides. Each end is independent: a pinned side is kept
 * verbatim while the other end keeps rederiving against the opposing center.
 */
export function resolveEdgeSides(
  from: CanvasSceneEntity,
  to: CanvasSceneEntity,
  pinned: { fromSide?: EdgeSide; toSide?: EdgeSide } = {},
  originY = 0,
): { fromSide: EdgeSide; toSide: EdgeSide } {
  return {
    fromSide: pinned.fromSide ?? autoSide(from, sceneEntityCenter(to, originY), originY),
    toSide: pinned.toSide ?? autoSide(to, sceneEntityCenter(from, originY), originY),
  }
}

// --- Elbow routing ---

function sideAxis(side: EdgeSide): EdgeSplitAxis {
  return side === 'left' || side === 'right' ? 'x' : 'y'
}

/** Unit step pointing away from the entity the anchor sits on. */
function sideOutward(side: EdgeSide): number {
  return side === 'right' || side === 'bottom' ? 1 : -1
}

function samePoint(a: GeometryPoint, b: GeometryPoint): boolean {
  return Math.abs(a.x - b.x) < 0.01 && Math.abs(a.y - b.y) < 0.01
}

/** Drop duplicate and collinear waypoints so segment counts mean something. */
function simplify(points: GeometryPoint[]): GeometryPoint[] {
  const out: GeometryPoint[] = []
  for (const point of points) {
    if (out.length && samePoint(out[out.length - 1], point)) continue
    out.push({ x: point.x, y: point.y })
  }
  for (let i = out.length - 2; i > 0; i--) {
    const prev = out[i - 1]
    const next = out[i + 1]
    const collinear =
      (Math.abs(prev.x - out[i].x) < 0.01 && Math.abs(next.x - out[i].x) < 0.01) ||
      (Math.abs(prev.y - out[i].y) < 0.01 && Math.abs(next.y - out[i].y) < 0.01)
    if (collinear) out.splice(i, 1)
  }
  return out
}

/**
 * The axis a route's crossbar can be dragged along, or null when the route has
 * nothing adjustable (a 5-segment S ships non-adjustable).
 */
export function elbowAdjustableAxis(from: AnchorPoint, to: AnchorPoint): EdgeSplitAxis | null {
  const fromAxis = sideAxis(from.side)
  const toAxis = sideAxis(to.side)
  if (fromAxis !== toAxis) return fromAxis
  const outFrom = sideOutward(from.side)
  const outTo = sideOutward(to.side)
  const gap = fromAxis === 'x' ? to.x - from.x : to.y - from.y
  const facing = outTo === -outFrom && Math.sign(gap) === outFrom
  return facing ? fromAxis : null
}

/**
 * Waypoints of an elbow route, endpoints included. A stored split only applies
 * on the axis it was dragged on — a route whose adjustable axis differs ignores
 * it rather than reinterpreting the same number as a different placement.
 */
export function buildElbowPoints(
  from: AnchorPoint,
  to: AnchorPoint,
  zoom = 1,
  split?: ElbowSplit,
): GeometryPoint[] {
  const stub = ELBOW_STUB * zoom
  const fromAxis = sideAxis(from.side)
  const toAxis = sideAxis(to.side)
  const outFrom = sideOutward(from.side)
  const outTo = sideOutward(to.side)
  const adjustable = elbowAdjustableAxis(from, to)
  const t = split && split.axis === adjustable ? split.value : null

  if (fromAxis === toAxis && adjustable) {
    // Facing each other on one axis: a single crossbar between them.
    const ratio = t ?? 0.5
    if (fromAxis === 'x') {
      const x = from.x + (to.x - from.x) * ratio
      return simplify([from, { x, y: from.y }, { x, y: to.y }, to])
    }
    const y = from.y + (to.y - from.y) * ratio
    return simplify([from, { x: from.x, y }, { x: to.x, y }, to])
  }

  if (fromAxis === toAxis) {
    // Facing away or overlapping: step clear of both ends and cross in between.
    if (fromAxis === 'x') {
      const x1 = from.x + outFrom * stub
      const x2 = to.x + outTo * stub
      const midY = (from.y + to.y) / 2
      return simplify([
        from,
        { x: x1, y: from.y },
        { x: x1, y: midY },
        { x: x2, y: midY },
        { x: x2, y: to.y },
        to,
      ])
    }
    const y1 = from.y + outFrom * stub
    const y2 = to.y + outTo * stub
    const midX = (from.x + to.x) / 2
    return simplify([
      from,
      { x: from.x, y: y1 },
      { x: midX, y: y1 },
      { x: midX, y: y2 },
      { x: to.x, y: y2 },
      to,
    ])
  }

  // Perpendicular sides. Without a stored split the corner is fully determined,
  // so the route is an L whenever that corner lies ahead of both ends.
  if (fromAxis === 'x') {
    const corner = { x: to.x, y: from.y }
    const cornerAhead =
      Math.sign(corner.x - from.x) !== -outFrom && Math.sign(corner.y - to.y) !== -outTo
    if (t === null && cornerAhead) return simplify([from, corner, to])
    const x = t === null ? from.x + outFrom * stub : from.x + (to.x - from.x) * t
    const y = to.y + outTo * stub
    return simplify([from, { x, y: from.y }, { x, y }, { x: to.x, y }, to])
  }

  const corner = { x: from.x, y: to.y }
  const cornerAhead =
    Math.sign(corner.y - from.y) !== -outFrom && Math.sign(corner.x - to.x) !== -outTo
  if (t === null && cornerAhead) return simplify([from, corner, to])
  const y = t === null ? from.y + outFrom * stub : from.y + (to.y - from.y) * t
  const x = to.x + outTo * stub
  return simplify([from, { x: from.x, y }, { x, y }, { x, y: to.y }, to])
}

function segmentLength(a: GeometryPoint, b: GeometryPoint): number {
  return Math.hypot(b.x - a.x, b.y - a.y)
}

function towards(from: GeometryPoint, to: GeometryPoint, distance: number): GeometryPoint {
  const length = segmentLength(from, to)
  if (length === 0) return { ...from }
  return {
    x: from.x + ((to.x - from.x) / length) * distance,
    y: from.y + ((to.y - from.y) / length) * distance,
  }
}

/** SVG path through elbow waypoints, corners rounded and clamped. */
export function elbowPathFromPoints(points: GeometryPoint[], zoom = 1): string {
  if (points.length === 0) return ''
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`
  const maxRadius = ELBOW_CORNER_RADIUS * zoom
  let d = `M ${points[0].x} ${points[0].y}`
  for (let i = 1; i < points.length - 1; i++) {
    const prev = points[i - 1]
    const corner = points[i]
    const next = points[i + 1]
    // Half the shorter adjacent segment, so neighbouring corners never overlap.
    const radius = Math.min(
      maxRadius,
      segmentLength(prev, corner) / 2,
      segmentLength(corner, next) / 2,
    )
    const start = towards(corner, prev, radius)
    const end = towards(corner, next, radius)
    d += ` L ${start.x} ${start.y}`
    if (radius > 0) d += ` Q ${corner.x} ${corner.y} ${end.x} ${end.y}`
  }
  const last = points[points.length - 1]
  d += ` L ${last.x} ${last.y}`
  return d
}

export function buildElbowPath(
  from: AnchorPoint,
  to: AnchorPoint,
  zoom = 1,
  split?: ElbowSplit,
): string {
  return elbowPathFromPoints(buildElbowPoints(from, to, zoom, split), zoom)
}

/**
 * Resolve both endpoints of an edge to anchor points, whichever mix of bound
 * and free ends it has. Shared by `EdgeLayer` (rendering) and the edge-drag
 * controller (re-route gesture) so a free end resolves identically in both —
 * null when a bound end's entity isn't in `entityMap` or a free end has no
 * stored point.
 */
export function resolveEdgeAnchors(
  edge: Pick<WorkspaceEdge, 'fromEntityId' | 'toEntityId' | 'fromPoint' | 'toPoint' | 'fromSide' | 'toSide'>,
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  zoom = 1,
  originY = 0,
): { from: AnchorPoint; to: AnchorPoint } | null {
  const fromEntity = edge.fromEntityId ? entityMap.get(edge.fromEntityId) : undefined
  const toEntity = edge.toEntityId ? entityMap.get(edge.toEntityId) : undefined
  if (edge.fromEntityId && !fromEntity) return null
  if (edge.toEntityId && !toEntity) return null
  if (!fromEntity && !edge.fromPoint) return null
  if (!toEntity && !edge.toPoint) return null

  if (fromEntity && toEntity) {
    const { fromSide, toSide } = resolveEdgeSides(fromEntity, toEntity, edge, originY)
    return {
      from: getAnchorPoint(fromEntity, fromSide, zoom, originY),
      to: getAnchorPoint(toEntity, toSide, zoom, originY),
    }
  }
  if (fromEntity && edge.toPoint) {
    const toSide = edge.toSide ?? sideTowardPoint(edge.toPoint, sceneEntityCenter(fromEntity, originY))
    const fromSide = edge.fromSide ?? autoSide(fromEntity, edge.toPoint, originY)
    return {
      from: getAnchorPoint(fromEntity, fromSide, zoom, originY),
      to: freeAnchorPoint(edge.toPoint, toSide, originY),
    }
  }
  if (toEntity && edge.fromPoint) {
    const fromSide = edge.fromSide ?? sideTowardPoint(edge.fromPoint, sceneEntityCenter(toEntity, originY))
    const toSide = edge.toSide ?? autoSide(toEntity, edge.fromPoint, originY)
    return {
      from: freeAnchorPoint(edge.fromPoint, fromSide, originY),
      to: getAnchorPoint(toEntity, toSide, zoom, originY),
    }
  }
  if (edge.fromPoint && edge.toPoint) {
    const fromSide = edge.fromSide ?? sideTowardPoint(edge.fromPoint, edge.toPoint)
    const toSide = edge.toSide ?? sideTowardPoint(edge.toPoint, edge.fromPoint)
    return {
      from: freeAnchorPoint(edge.fromPoint, fromSide, originY),
      to: freeAnchorPoint(edge.toPoint, toSide, originY),
    }
  }
  return null
}

/** Dispatch an edge onto its routing style. Absent routing stays bezier. */
export function buildEdgePath(
  edge: Pick<WorkspaceEdge, 'routing' | 'elbowSplit' | 'elbowSplitAxis'>,
  from: AnchorPoint,
  to: AnchorPoint,
  zoom = 1,
): string {
  switch (edge.routing) {
    case 'straight':
      return buildStraightPath(from, to)
    case 'elbow': {
      const split =
        edge.elbowSplit !== undefined && edge.elbowSplitAxis !== undefined
          ? { value: edge.elbowSplit, axis: edge.elbowSplitAxis }
          : undefined
      return buildElbowPath(from, to, zoom, split)
    }
    default:
      return buildBezierPath(from, to, zoom)
  }
}
