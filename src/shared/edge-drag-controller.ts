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
  resolveEdgeSides,
  buildBezierPath,
  getAnchorPoint,
  type AnchorPoint,
} from './edge-geometry'
import type { CanvasSceneEntity, EdgeSide, WorkspaceEdge } from './types'

const SNAP_DISTANCE = 48

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
      fromEntityId: string
      fromSide: EdgeSide
      cursorX: number
      cursorY: number
      snap: SnapTarget | null
    }
  | {
      kind: 'edit'
      edgeId: string
      movingEnd: 'from' | 'to'
      fixedEntityId: string
      fixedSide: EdgeSide
      cursorX: number
      cursorY: number
      snap: SnapTarget | null
    }

export type CommitOutcome =
  | { kind: 'noop' }
  | {
      kind: 'create-edge'
      fromEntityId: string
      fromSide: EdgeSide
      toEntityId: string
      toSide: EdgeSide
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
  side: EdgeSide,
  cursorX: number,
  cursorY: number,
  edges: readonly WorkspaceEdge[],
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
): EdgeDragState {
  const existing = findEdgeAtAnchor(edges, entityMap, fromEntityId, side)
  if (existing) {
    return {
      kind: 'edit',
      edgeId: existing.edgeId,
      movingEnd: existing.movingEnd,
      fixedEntityId: existing.fixedEntityId,
      fixedSide: existing.fixedSide,
      cursorX,
      cursorY,
      snap: null,
    }
  }
  return {
    kind: 'create',
    fromEntityId,
    fromSide: side,
    cursorX,
    cursorY,
    snap: null,
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
    fromEntityId,
    cursorX,
    cursorY,
    scaleSnapDistance(SNAP_DISTANCE, zoom),
    zoom,
  )
  return { ...state, cursorX, cursorY, snap }
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
  // create
  if (state.snap) {
    return {
      kind: 'create-edge',
      fromEntityId: state.fromEntityId,
      fromSide: state.fromSide,
      toEntityId: state.snap.entityId,
      toSide: state.snap.side,
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
): { entityId: string; side: EdgeSide } | null {
  switch (state.kind) {
    case 'idle':
      return null
    case 'create':
      return { entityId: state.fromEntityId, side: state.fromSide }
    case 'edit':
      return { entityId: state.fixedEntityId, side: state.fixedSide }
  }
}

// --- Visual helpers (rendered by EdgeDragLayer in aboveView) ---

export function buildEdgeDragPath(
  state: EdgeDragState,
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  zoom: number,
): { d: string; from: AnchorPoint; to: AnchorPoint } | null {
  if (state.kind === 'idle') return null
  const fromEntity = entityMap.get(
    state.kind === 'create' ? state.fromEntityId : state.fixedEntityId,
  )
  if (!fromEntity) return null
  const fromSide = state.kind === 'create' ? state.fromSide : state.fixedSide
  const from = getAnchorPoint(fromEntity, fromSide, zoom)

  const to: AnchorPoint = state.snap
    ? getAnchorPoint(entityMap.get(state.snap.entityId)!, state.snap.side, zoom)
    : { x: state.cursorX, y: state.cursorY, side: oppositeSide(fromSide) }

  return { d: buildBezierPath(from, to, zoom), from, to }
}

// --- Internal pure helpers ---

function findClosestAnchorTarget(
  entityMap: ReadonlyMap<string, CanvasSceneEntity>,
  fromEntityId: string,
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
  fixedEntityId: string
  fixedSide: EdgeSide
} | null {
  for (const edge of edges) {
    const fromEntity = entityMap.get(edge.fromEntityId)
    const toEntity = entityMap.get(edge.toEntityId)
    if (!fromEntity || !toEntity) continue
    const { fromSide, toSide } = resolveEdgeSides(fromEntity, toEntity, edge)
    if (edge.toEntityId === entityId && toSide === side) {
      return {
        edgeId: edge.id,
        movingEnd: 'to',
        fixedEntityId: edge.fromEntityId,
        fixedSide: fromSide,
      }
    }
    if (edge.fromEntityId === entityId && fromSide === side) {
      return {
        edgeId: edge.id,
        movingEnd: 'from',
        fixedEntityId: edge.toEntityId,
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
