import type { AgentSnapshotPage, AgentSnapshotNode } from '../../shared/types'

interface AgentSnapshotCacheEntry {
  snapshot: AgentSnapshotPage
  nodesByRef: Map<string, AgentSnapshotNode>
}

const agentSnapshotCache = new Map<string, AgentSnapshotCacheEntry>()

export function cacheAgentSnapshot(snapshot: AgentSnapshotPage): void {
  agentSnapshotCache.set(snapshot.pageId, {
    snapshot,
    nodesByRef: new Map(snapshot.nodes.map((node) => [node.ref, node])),
  })
}

export function getAgentSnapshot(pageId: string): AgentSnapshotPage | null {
  return agentSnapshotCache.get(pageId)?.snapshot ?? null
}

export function resolveAgentSnapshotNode(pageId: string, ref: string): AgentSnapshotNode | null {
  return agentSnapshotCache.get(pageId)?.nodesByRef.get(ref) ?? null
}

// Lets another cache key its own invalidation off this one (the presence
// layer's agent-browser ref map — see presence-manager.ts) without this
// module knowing anything about that caller.
const invalidationListeners = new Set<(pageId: string) => void>()

export function onAgentSnapshotInvalidated(listener: (pageId: string) => void): () => void {
  invalidationListeners.add(listener)
  return () => { invalidationListeners.delete(listener) }
}

export function invalidateAgentSnapshot(pageId: string): void {
  agentSnapshotCache.delete(pageId)
  for (const listener of invalidationListeners) listener(pageId)
}
