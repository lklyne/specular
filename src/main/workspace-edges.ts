import type { CreateEdgesRequest, CreateEdgesResponse, WorkspaceEdge } from '../shared/types'
import { workspaceEdges } from './runtime/space-model'
import { markDirty } from './runtime/layout-dirty'
import { mutateWorkspace } from './runtime/mutate-workspace'
import { appendStackOrderIdsAtTop } from './runtime/entity-order-state'
import { makeId, cloneMetadata } from './workspace-utils'

export function edgeExists(id: string): boolean {
  return workspaceEdges.some((edge) => edge.id === id)
}

export function createEdges(input: CreateEdgesRequest): CreateEdgesResponse {
  return mutateWorkspace(
    () => createEdgesInternal(input),
    { changed: (result) => result.edgeIds.length > 0 },
  )
}

function createEdgesInternal(input: CreateEdgesRequest): CreateEdgesResponse {
  const edgeIds: string[] = []
  const newIds: string[] = []
  for (const edge of input.edges) {
    // A caller-supplied id that already exists is an update, not a second
    // record — this is the only door edges are created through, so guarding
    // here keeps duplicate ids impossible everywhere (apply route included).
    const existing = edge.id ? workspaceEdges.find((e) => e.id === edge.id) : undefined
    if (existing) {
      existing.fromEntityId = edge.fromEntityId
      existing.toEntityId = edge.toEntityId
      existing.fromPoint = edge.fromPoint
      existing.toPoint = edge.toPoint
      existing.fromSide = edge.fromSide
      existing.toSide = edge.toSide
      existing.fromEnd = edge.fromEnd
      existing.toEnd = edge.toEnd
      existing.color = edge.color
      existing.label = edge.label
      existing.strokeWidth = edge.strokeWidth
      existing.lineStyle = edge.lineStyle
      existing.routing = edge.routing
      existing.elbowSplit = edge.elbowSplit
      existing.elbowSplitAxis = edge.elbowSplitAxis
      existing.kind = edge.kind
      existing.metadata = cloneMetadata(edge.metadata)
      edgeIds.push(existing.id)
      continue
    }
    const nextEdge: WorkspaceEdge = {
      id: edge.id ?? makeId('edge'),
      fromEntityId: edge.fromEntityId,
      toEntityId: edge.toEntityId,
      fromPoint: edge.fromPoint,
      toPoint: edge.toPoint,
      fromSide: edge.fromSide,
      toSide: edge.toSide,
      fromEnd: edge.fromEnd,
      toEnd: edge.toEnd,
      color: edge.color,
      label: edge.label,
      strokeWidth: edge.strokeWidth,
      lineStyle: edge.lineStyle,
      routing: edge.routing,
      elbowSplit: edge.elbowSplit,
      elbowSplitAxis: edge.elbowSplitAxis,
      kind: edge.kind,
      metadata: cloneMetadata(edge.metadata),
    }
    workspaceEdges.push(nextEdge)
    edgeIds.push(nextEdge.id)
    newIds.push(nextEdge.id)
  }
  if (newIds.length) {
    appendStackOrderIdsAtTop(newIds)
  }
  return { edgeIds }
}

export function deleteEdges(input: { edgeIds: string[] }): { deletedEdgeIds: string[] } {
  return mutateWorkspace(() => {
    const deletedEdgeIds: string[] = []
    for (const edgeId of input.edgeIds) {
      const idx = workspaceEdges.findIndex((edge) => edge.id === edgeId)
      if (idx === -1) continue
      deletedEdgeIds.push(workspaceEdges[idx].id)
      workspaceEdges.splice(idx, 1)
    }
    return { deletedEdgeIds }
  }, { changed: (result) => result.deletedEdgeIds.length > 0 })
}

/**
 * An edge whose only bound end is deleted has nowhere left to attach — that
 * edge is removed. An edge with the other end still bound (or already free)
 * instead detaches the deleted end to a free point, rather than disappearing.
 * `lastKnownPoint` supplies the point to detach to — callers look it up
 * (e.g. via `entityBoundsById`) before deleting the entity, since by the time
 * this cascade runs the entity is already gone from state.
 */
export function removeEdgesTouchingEntities(
  entityIds: Set<string>,
  lastKnownPoint?: (entityId: string) => { x: number; y: number } | null,
): string[] {
  const deletedEdgeIds: string[] = []
  let detached = false
  for (let idx = workspaceEdges.length - 1; idx >= 0; idx--) {
    const edge = workspaceEdges[idx]
    const fromGone = edge.fromEntityId !== null && entityIds.has(edge.fromEntityId)
    const toGone = edge.toEntityId !== null && entityIds.has(edge.toEntityId)
    if (!fromGone && !toGone) continue

    const fromUsable = !fromGone && (edge.fromEntityId !== null || !!edge.fromPoint)
    const toUsable = !toGone && (edge.toEntityId !== null || !!edge.toPoint)
    if (!fromUsable && !toUsable) {
      deletedEdgeIds.push(edge.id)
      workspaceEdges.splice(idx, 1)
      continue
    }

    if (fromGone) {
      edge.fromPoint = lastKnownPoint?.(edge.fromEntityId as string) ?? edge.fromPoint
      edge.fromEntityId = null
      edge.fromSide = undefined
    }
    if (toGone) {
      edge.toPoint = lastKnownPoint?.(edge.toEntityId as string) ?? edge.toPoint
      edge.toEntityId = null
      edge.toSide = undefined
    }
    detached = true
  }
  if (deletedEdgeIds.length || detached) markDirty('canvas')
  return deletedEdgeIds
}
