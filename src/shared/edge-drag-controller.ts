/**
 * Edge-drag controller — pure state machine + geometry for anchor-to-anchor
 * edge gestures (the "drag from a page's anchor to another page" gesture
 * that creates or re-routes an edge in the workspace).
 *
 * Lives in src/shared so the canvas-pointer-router (renderer) and tests can
 * exercise the state transitions without the DOM. The legacy bgView path
 * (`EdgeLayer.tsx`) used to host this logic inside React closures + refs;
 * Phase 2/3 of the input-authority refactor lifts it out so the gesture has
 * one home.
 *
 * State shape:
 *   - `idle`                   — no drag in progress.
 *   - `create { from, cursor, snap? }`  — dragging from a free anchor.
 *   - `edit   { edgeId, fixed, cursor, snap?, movingEnd }`
 *                              — re-routing an existing edge whose endpoint
 *                                lived on the grabbed anchor.
 *
 * Transitions:
 *   - `beginDrag(target, point, edges, entityMap)` — `idle → create | edit`.
 *   - `updateCursor(state, point, entityMap, zoom)` — recompute snap target.
 *   - `commit(state)` — `create | edit → idle` with a typed outcome.
 *   - `cancel(state)` — `* → idle` with a typed outcome.
 *
 * No IPC, no DOM, no React. Callers (the router) translate outcomes into
 * `api.beginEdgeDrag` / `updateEdgeDragTarget` / `commitEdgeDrag` /
 * `commitEdgeEdit` / `discardEdgeEdit` / `cancelEdgeDrag` IPC calls.
 */

import { EDGE_ANCHOR_HIT_MIN_SCALE, EDGE_SIDES } from './canvas-hit-geometry'
import {
  autoSide,
  buildBezierPath,
  getAnchorPoint,
  resolveEdgeAnchors,
  sideTowardPoint,
  type AnchorPoint,
  type GeometryPoint,
} from './edge-geometry'
import type { CanvasSceneEntity, EdgeSide, WorkspaceEdge } from './types'

const SNAP_DISTANCE = 48
/** Below this the press is a click, and a click connects nothing. */
const FREE_END_MIN_TRAVEL = 4

// --- Public types ---

export type { AnchorPoint }

export interface SnapTarget {
  entityId: string
  side: EdgeSide
  dist: number
}

export type EdgeDragState =
  | { kind: 'idle' }
  | {
      kind: 'create'
      /** Null when the drag started on empty canvas — `fromPoint` holds it. */
      fromEntityId: string | null
      fromPoint?: GeometryPoint
      /** Null means "no side yet": a body grab, resolved toward the cursor per
       *  move and committed as `undefined` so the end stays object-bound. */
      fromSide: EdgeSide | null
      /** Grab point, so a press that never travels can commit nothing. */
      originX: number
      originY: number
      cursorX: number
      cursorY: number
      snap: SnapTarget | null
      /** Entity whose body the cursor is over, source excluded. Drives the
       *  body-release commit that the anchor snap can't express. */
      hoverEntityId: string | null
      /** Connect-tool drags may end in empty space, leaving a free end; the
       *  anchor door still treats that release as a no-op. */
      freeEndsAllowed: boolean
    }
  | {
      kind: 'edit'
      edgeId: string
      movingEnd: 'from' | 'to'
      // Null when the far end of the edge being re-routed is itself free —
      // a legal starting state for this gesture, not just a legal target.
      fixedEntityId: string | null
      fixedSide: EdgeSide
      fixedPoint?: { x: number; y: number }
      cursorX: number
      cursorY: number
      snap: SnapTarget | null
    }

export type CommitOutcome =
  | { kind: 'noop' }
  | {
      kind: 'create-edge'
      fromEntityId: string | null
      /** In the drag's own (window) space — the caller converts to canvas. */
      fromPoint?: GeometryPoint
      /** Undefined means object-bound: the side rederives per paint. */
      fromSide?: EdgeSide
      toEntityId: string | null
      toPoint?: GeometryPoint
      toSide?: EdgeSide
    }
  | {
      kind: 'edit-edge'
      edgeId: string
      movingEnd: 'from' | 'to'
      targetEntityId: string
      targetSide: EdgeSide
    }
  | { kind: 'discard-edge'; edgeId: string }

// --- Construction ---

export const EDGE_DRAG_IDLE: EdgeDragState = { kind: 'idle' }

export function beginEdgeDrag(
  fromEntityId: string,
  side: EdgeSide | null,
  cursorX: number,
  cursorY: number,
  edges: readonly WorkspaceEdge[],
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  options: { freeEndsAllowed?: boolean } = {},
): EdgeDragState {
  const existing = side ? findEdgeAtAnchor(edges, entityMap, fromEntityId, side) : null
  if (existing) {
    return {
      kind: 'edit',
      edgeId: existing.edgeId,
      movingEnd: existing.movingEnd,
      fixedEntityId: existing.fixedEntityId,
      fixedSide: existing.fixedSide,
      fixedPoint: existing.fixedPoint,
      cursorX,
      cursorY,
      snap: null,
    }
  }
  return {
    kind: 'create',
    fromEntityId,
    fromSide: side,
    originX: cursorX,
    originY: cursorY,
    cursorX,
    cursorY,
    snap: null,
    hoverEntityId: null,
    freeEndsAllowed: options.freeEndsAllowed ?? false,
  }
}

/** Connect tool on empty canvas: the edge's start is a bare point. */
export function beginFreeEdgeDrag(
  fromPoint: GeometryPoint,
  cursorX: number,
  cursorY: number,
): EdgeDragState {
  return {
    kind: 'create',
    fromEntityId: null,
    fromPoint,
    fromSide: null,
    originX: cursorX,
    originY: cursorY,
    cursorX,
    cursorY,
    snap: null,
    hoverEntityId: null,
    freeEndsAllowed: true,
  }
}

export function updateEdgeDragCursor(
  state: EdgeDragState,
  cursorX: number,
  cursorY: number,
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  zoom: number,
): EdgeDragState {
  if (state.kind === 'idle') return state
  const fromEntityId =
    state.kind === 'create' ? state.fromEntityId : state.fixedEntityId
  const snap = findClosestAnchorTarget(
    entityMap,
    fromEntityId ?? undefined,
    cursorX,
    cursorY,
    scaleSnapDistance(SNAP_DISTANCE, zoom),
    zoom,
  )
  if (state.kind === 'edit') return { ...state, cursorX, cursorY, snap }
  return {
    ...state,
    cursorX,
    cursorY,
    snap,
    // The source is deliberately NOT excluded: a release on it has to be
    // recognised so it can commit `noop` rather than a self-edge.
    hoverEntityId: entityAtPoint(entityMap, cursorX, cursorY),
  }
}

/** Topmost entity whose body contains the point. */
function entityAtPoint(
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  x: number,
  y: number,
): string | null {
  let found: string | null = null
  for (const [entityId, entity] of entityMap) {
    if (
      x >= entity.screenX &&
      x <= entity.screenX + entity.screenWidth &&
      y >= entity.screenY &&
      y <= entity.screenY + entity.screenHeight
    ) {
      found = entityId
    }
  }
  return found
}

export function commitEdgeDrag(state: EdgeDragState): CommitOutcome {
  if (state.kind === 'idle') return { kind: 'noop' }
  if (state.kind === 'edit') {
    if (state.snap) {
      return {
        kind: 'edit-edge',
        edgeId: state.edgeId,
        movingEnd: state.movingEnd,
        targetEntityId: state.snap.entityId,
        targetSide: state.snap.side,
      }
    }
    return { kind: 'discard-edge', edgeId: state.edgeId }
  }
  // create. A release on the source entity is a self-edge — rejected, because
  // a self-loop needs its own route and shares nothing with this builder.
  if (state.hoverEntityId !== null && state.hoverEntityId === state.fromEntityId) {
    return { kind: 'noop' }
  }
  const from = {
    fromEntityId: state.fromEntityId,
    fromPoint: state.fromEntityId ? undefined : state.fromPoint,
    // A null side is object-bound, not "top": it rederives per paint.
    fromSide: state.fromSide ?? undefined,
  }
  if (state.snap) {
    return {
      kind: 'create-edge',
      ...from,
      toEntityId: state.snap.entityId,
      toSide: state.snap.side,
    }
  }
  // Released over a body with no anchor snap: bind to the object, side auto.
  if (state.hoverEntityId) {
    return { kind: 'create-edge', ...from, toEntityId: state.hoverEntityId }
  }
  // A click that never travels creates nothing — the tool just stays active.
  const travelled = Math.hypot(state.cursorX - state.originX, state.cursorY - state.originY)
  if (state.freeEndsAllowed && travelled >= FREE_END_MIN_TRAVEL && (state.fromEntityId || state.fromPoint)) {
    return {
      kind: 'create-edge',
      ...from,
      toEntityId: null,
      toPoint: { x: state.cursorX, y: state.cursorY },
    }
  }
  return { kind: 'noop' }
}

export function cancelEdgeDrag(state: EdgeDragState): CommitOutcome {
  if (state.kind === 'edit') return { kind: 'discard-edge', edgeId: state.edgeId }
  return { kind: 'noop' }
}

/**
 * The anchor main is told the gesture hangs off in `beginEdgeDrag` IPC: the
 * grabbed anchor for a create drag, the far (fixed) endpoint for an edit
 * drag — the rubber-band is pinned to the end that is NOT moving.
 */
export function edgeDragOrigin(
  state: EdgeDragState,
  entityMap?: ReadonlyMap<string, CanvasSceneEntity>,
): { entityId: string; side: EdgeSide } | { point: { x: number; y: number }; side: EdgeSide } | null {
  switch (state.kind) {
    case 'idle':
      return null
    case 'create': {
      const side = createOriginSide(state, entityMap)
      if (state.fromEntityId) return { entityId: state.fromEntityId, side }
      if (state.fromPoint) return { point: state.fromPoint, side }
      return null
    }
    case 'edit':
      if (state.fixedEntityId) return { entityId: state.fixedEntityId, side: state.fixedSide }
      if (state.fixedPoint) return { point: state.fixedPoint, side: state.fixedSide }
      return null
  }
}

// --- Visual helpers (rendered by EdgeDragLayer in aboveView) ---

export function buildEdgeDragPath(
  state: EdgeDragState,
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  zoom: number,
): { d: string; from: AnchorPoint; to: AnchorPoint } | null {
  if (state.kind === 'idle') return null
  let from: AnchorPoint
  let fromSide: EdgeSide
  if (state.kind === 'create') {
    fromSide = createOriginSide(state, entityMap)
    if (state.fromEntityId) {
      const fromEntity = entityMap.get(state.fromEntityId)
      if (!fromEntity) return null
      from = getAnchorPoint(fromEntity, fromSide, zoom)
    } else if (state.fromPoint) {
      from = { x: state.fromPoint.x, y: state.fromPoint.y, side: fromSide }
    } else {
      return null
    }
  } else {
    fromSide = state.fixedSide
    if (state.fixedEntityId) {
      const fromEntity = entityMap.get(state.fixedEntityId)
      if (!fromEntity) return null
      from = getAnchorPoint(fromEntity, fromSide, zoom)
    } else if (state.fixedPoint) {
      from = { x: state.fixedPoint.x, y: state.fixedPoint.y, side: fromSide }
    } else {
      return null
    }
  }

  const to: AnchorPoint = state.snap
    ? getAnchorPoint(entityMap.get(state.snap.entityId)!, state.snap.side, zoom)
    : { x: state.cursorX, y: state.cursorY, side: oppositeSide(fromSide) }

  return { d: buildBezierPath(from, to, zoom), from, to }
}

/**
 * The side a create drag's rubber-band leaves from. A pinned side is kept; a
 * null side faces the cursor, so the band swings around the source as the
 * pointer crosses its corners.
 */
function createOriginSide(
  state: Extract<EdgeDragState, { kind: 'create' }>,
  entityMap?: ReadonlyMap<string, CanvasSceneEntity>,
): EdgeSide {
  if (state.fromSide) return state.fromSide
  const cursor = { x: state.cursorX, y: state.cursorY }
  const fromEntity = state.fromEntityId ? entityMap?.get(state.fromEntityId) : undefined
  if (fromEntity) return autoSide(fromEntity, cursor)
  if (state.fromPoint) return sideTowardPoint(state.fromPoint, cursor)
  return 'right'
}

// --- Internal pure helpers ---

function findClosestAnchorTarget(
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  fromEntityId: string | undefined,
  clientX: number,
  clientY: number,
  snapDistance: number,
  zoom: number,
): SnapTarget | null {
  let best: SnapTarget | null = null
  for (const [entityId, entity] of entityMap) {
    if (entityId === fromEntityId) continue
    for (const side of EDGE_SIDES) {
      const pt = getAnchorPoint(entity, side, zoom)
      const dist = Math.hypot(pt.x - clientX, pt.y - clientY)
      if (dist < snapDistance && (!best || dist < best.dist)) {
        best = { entityId, side, dist }
      }
    }
  }
  return best
}

function findEdgeAtAnchor(
  edges: readonly WorkspaceEdge[],
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  entityId: string,
  side: EdgeSide,
): {
  edgeId: string
  movingEnd: 'from' | 'to'
  fixedEntityId: string | null
  fixedSide: EdgeSide
  fixedPoint?: { x: number; y: number }
} | null {
  for (const edge of edges) {
    const anchors = resolveEdgeAnchors(edge, entityMap)
    if (!anchors) continue
    const { from: fromAnchor, to: toAnchor } = anchors
    const fromSide = fromAnchor.side
    const toSide = toAnchor.side
    if (edge.toEntityId === entityId && toSide === side) {
      return {
        edgeId: edge.id,
        movingEnd: 'to',
        fixedEntityId: edge.fromEntityId,
        fixedSide: fromSide,
        fixedPoint: edge.fromEntityId ? undefined : { x: fromAnchor.x, y: fromAnchor.y },
      }
    }
    if (edge.fromEntityId === entityId && fromSide === side) {
      return {
        edgeId: edge.id,
        movingEnd: 'from',
        fixedEntityId: edge.toEntityId,
        fixedPoint: edge.toEntityId ? undefined : { x: toAnchor.x, y: toAnchor.y },
        fixedSide: toSide,
      }
    }
  }
  return null
}

function oppositeSide(side: EdgeSide): EdgeSide {
  switch (side) {
    case 'top': return 'bottom'
    case 'bottom': return 'top'
    case 'left': return 'right'
    case 'right': return 'left'
  }
}

function scaleSnapDistance(base: number, zoom: number): number {
  const scale = Math.max(EDGE_ANCHOR_HIT_MIN_SCALE, Math.min(zoom, 1))
  return base * scale
}
