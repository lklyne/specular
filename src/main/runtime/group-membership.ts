import { allEntities } from '../entities/contract'
import { reflowManagedGroup } from '../managed-layout'
import { normalizeGroupStackContiguity } from './entity-order-state'
import { markDirty } from './layout-dirty'
import { mutateWorkspace } from './mutate-workspace'
import { workspaceGroups } from './workspace-model'

function entityById(id: string) {
  return allEntities().find(({ entity }) => entity.id === id)?.entity ?? null
}

function isGroupDescendant(candidateId: string, ancestorId: string): boolean {
  let parentId = entityById(candidateId)?.parentGroupId
  while (parentId) {
    if (parentId === ancestorId) return true
    parentId = entityById(parentId)?.parentGroupId
  }
  return false
}

function rootIds(ids: readonly string[]): string[] {
  const requested = new Set(ids)
  return [...requested].filter((id) => {
    let parentId = entityById(id)?.parentGroupId
    while (parentId) {
      if (requested.has(parentId)) return false
      parentId = entityById(parentId)?.parentGroupId
    }
    return true
  })
}

function canReparentEntity(id: string, parentGroupId: string | null): boolean {
  if (!parentGroupId) return true
  if (id === parentGroupId) return false
  const isGroup = workspaceGroups.some((group) => group.id === id)
  return !isGroup || !isGroupDescendant(parentGroupId, id)
}

function applyParentChange(
  id: string,
  parentGroupId: string | null,
  affectedGroups: Set<string>,
): boolean {
  const entity = entityById(id)
  if (!entity || !canReparentEntity(id, parentGroupId)) return false

  const previousParentId = entity.parentGroupId
  const nextParentId = parentGroupId ?? undefined
  if (previousParentId === nextParentId) return false
  if (previousParentId) affectedGroups.add(previousParentId)
  if (nextParentId) affectedGroups.add(nextParentId)
  entity.parentGroupId = nextParentId
  return true
}

/**
 * Raw membership writer for an already-open gesture transaction. The caller
 * owns persistence/undo finalization.
 */
export function reparentEntitiesInGesture(
  ids: readonly string[],
  parentGroupId: string | null,
): string[] {
  if (parentGroupId && !workspaceGroups.some((group) => group.id === parentGroupId)) return []

  const changed: string[] = []
  const affectedGroups = new Set<string>()
  for (const id of rootIds(ids)) {
    if (applyParentChange(id, parentGroupId, affectedGroups)) changed.push(id)
  }

  for (const groupId of affectedGroups) reflowManagedGroup(groupId)
  if (changed.length) {
    normalizeGroupStackContiguity()
    markDirty('canvas', 'sidebar')
  }
  return changed
}

/** Reparent canvas items as one persisted, undoable document mutation. */
export function reparentEntities(
  ids: readonly string[],
  parentGroupId: string | null,
): string[] {
  return mutateWorkspace(
    () => reparentEntitiesInGesture(ids, parentGroupId),
    { changed: (changedIds) => changedIds.length > 0 },
  )
}
