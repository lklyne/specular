import type { CreateEdgesRequest, CreateEdgesResponse, WorkspaceEdge } from '../shared/types'
import { workspaceEdges } from './runtime/workspace-model'
import { markDirty } from './runtime/layout-dirty'
import { scheduleWorkspaceAutosave } from './runtime/workspace-session'
import { appendStackOrderIdsAtTop } from './runtime/entity-order-state'
import { makeId, cloneMetadata } from './workspace-utils'

export function createEdges(input: CreateEdgesRequest): CreateEdgesResponse {
  const edgeIds: string[] = []
  for (const edge of input.edges) {
    const nextEdge: WorkspaceEdge = {
      id: edge.id ?? makeId('edge'),
      fromEntityId: edge.fromEntityId,
      toEntityId: edge.toEntityId,
      fromSide: edge.fromSide,
      toSide: edge.toSide,
      fromEnd: edge.fromEnd,
      toEnd: edge.toEnd,
      kind: edge.kind,
      metadata: cloneMetadata(edge.metadata),
    }
    workspaceEdges.push(nextEdge)
    edgeIds.push(nextEdge.id)
  }
  if (edgeIds.length) {
    appendStackOrderIdsAtTop(edgeIds)
    markDirty('canvas')
    scheduleWorkspaceAutosave()
  }
  return { edgeIds }
}

export function deleteEdges(input: { edgeIds: string[] }): { deletedEdgeIds: string[] } {
  const deletedEdgeIds: string[] = []
  for (const edgeId of input.edgeIds) {
    const idx = workspaceEdges.findIndex((edge) => edge.id === edgeId)
    if (idx === -1) continue
    deletedEdgeIds.push(workspaceEdges[idx].id)
    workspaceEdges.splice(idx, 1)
  }
  if (deletedEdgeIds.length) {
    markDirty('canvas')
    scheduleWorkspaceAutosave()
  }
  return { deletedEdgeIds }
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
