/**
 * Edge-routing gesture coordinator — dragging an elbow edge's crossbar.
 *
 * Drives the `routing-edge` interaction mode, mirroring `gap-gesture.ts`:
 * the renderer's segment-handle drag calls start → move* → commit | cancel
 * through IPC, and this module is the one state machine.
 *
 * The split value itself is computed renderer-side (the crossbar's position is
 * screen geometry the renderer already has, from the same anchor points it
 * paints the route with) and arrives here already normalized to 0–1.
 *
 * Invariants (§6 I5):
 *   - `start` enters the interaction mode BEFORE any mutation.
 *   - `move` only updates the broadcast split — no doc writes per tick.
 *   - `commit` writes once (one Y.Doc transaction, one undo step).
 *   - `cancel` restores the pre-drag route with no mutation.
 */

import { tryEnter, commitActive, cancelActive } from './runtime/interaction-controller'
import { updateEdgeRoutingSplit } from './runtime/interaction-state'
import { markUndoBoundary } from './runtime/space-undo'
import { updateEdge } from './runtime/document-commands'
import { workspaceEdges } from './runtime/space-model'
import type { CancelReason } from '../shared/interaction-types'
import type { EdgeSplitAxis } from '../shared/types'

type ActiveGesture = {
  edgeId: string
  axis: EdgeSplitAxis
  startSplit: number | undefined
  split: number
}

let active: ActiveGesture | null = null

function clampSplit(split: number): number {
  return Math.min(1, Math.max(0, split))
}

/** Begin a crossbar drag. Returns false when the edge is gone or the mode is
 *  already owned by another gesture. */
export function startEdgeRoutingGesture(
  edgeId: string,
  split: number,
  axis: EdgeSplitAxis,
): boolean {
  const edge = workspaceEdges.find((e) => e.id === edgeId)
  if (!edge) return false
  const start = clampSplit(split)
  const token = tryEnter({ kind: 'routing-edge', edgeId, split: start, axis })
  if ('refused' in token) return false
  active = { edgeId, axis, startSplit: edge.elbowSplit, split: start }
  return true
}

/** Update the live crossbar preview. */
export function moveEdgeRoutingGesture(split: number): void {
  if (!active) return
  active.split = clampSplit(split)
  updateEdgeRoutingSplit(active.split)
}

/** Commit the crossbar at its live value. One write, one undo step. Returns
 *  whether anything changed. */
export function commitEdgeRoutingGesture(): boolean {
  const gesture = active
  active = null
  if (!gesture) {
    commitActive()
    return false
  }
  const changed =
    gesture.split !== gesture.startSplit &&
    updateEdge(gesture.edgeId, {
      elbowSplit: gesture.split,
      elbowSplitAxis: gesture.axis,
    })
  commitActive()
  markUndoBoundary()
  return changed
}

/** Abort the crossbar drag without touching the edge. */
export function cancelEdgeRoutingGesture(reason: CancelReason): void {
  active = null
  cancelActive(reason)
}
