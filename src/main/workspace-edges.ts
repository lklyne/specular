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
      existing.fromSide = edge.fromSide
      existing.toSide = edge.toSide
      existing.fromEnd = edge.fromEnd
      existing.toEnd = edge.toEnd
      existing.color = edge.color
      existing.label = edge.label
      existing.strokeWidth = edge.strokeWidth
      existing.lineStyle = edge.lineStyle
      existing.kind = edge.kind
      existing.metadata = cloneMetadata(edge.metadata)
      edgeIds.push(existing.id)
      continue
    }
    const nextEdge: WorkspaceEdge = {
      id: edge.id ?? makeId('edge'),
      fromEntityId: edge.fromEntityId,
      toEntityId: edge.toEntityId,
      fromSide: edge.fromSide,
      toSide: edge.toSide,
      fromEnd: edge.fromEnd,
      toEnd: edge.toEnd,
      color: edge.color,
      label: edge.label,
      strokeWidth: edge.strokeWidth,
      lineStyle: edge.lineStyle,
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

export function removeEdgesTouchingEntities(entityIds: Set<string>): string[] {
  const deletedEdgeIds: string[] = []
  for (let idx = workspaceEdges.length - 1; idx >= 0; idx--) {
    const edge = workspaceEdges[idx]
    if (entityIds.has(edge.fromEntityId) || entityIds.has(edge.toEntityId)) {
      deletedEdgeIds.push(edge.id)
      workspaceEdges.splice(idx, 1)
    }
  }
  if (deletedEdgeIds.length) markDirty('canvas')
  return deletedEdgeIds
}
