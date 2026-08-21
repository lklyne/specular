import {
  enforceGroupContiguity,
  type EntityOrderGroup,
} from '../../shared/entity-order-math'
import type {
  PersistedCanvasEntity,
  WorkspaceGroup,
  WorkspacePageSnapshot,
  WorkspaceSnapshot,
} from '../../shared/types'

type GroupDraft = {
  id: string
  parentGroupId: string | null
  childIds: Set<string>
}

function parentGroupIdForEntity(entity: PersistedCanvasEntity): string | null {
  return 'parentGroupId' in entity
    ? entity.parentGroupId ?? null
    : 'groupId' in entity
      ? entity.groupId ?? null
      : null
}

function parentGroupIdForPage(page: WorkspacePageSnapshot): string | null {
  return page.parentGroupId ?? page.groupId ?? null
}

function ensureGroup(groups: Map<string, GroupDraft>, id: string, parentGroupId: string | null): GroupDraft {
  const existing = groups.get(id)
  if (existing) {
    existing.parentGroupId = existing.parentGroupId ?? parentGroupId
    return existing
  }
  const group = { id, parentGroupId, childIds: new Set<string>() }
  groups.set(id, group)
  return group
}

function addChild(groups: Map<string, GroupDraft>, groupId: string | null, childId: string): void {
  if (!groupId || groupId === childId) return
  ensureGroup(groups, groupId, null).childIds.add(childId)
}

function addWorkspaceGroup(groups: Map<string, GroupDraft>, group: WorkspaceGroup): void {
  const draft = ensureGroup(groups, group.id, group.parentGroupId ?? null)
  for (const id of group.pageIds ?? []) draft.childIds.add(id)
  for (const id of group.entityIds ?? []) draft.childIds.add(id)
}

function defaultSnapshotOrder(snapshot: WorkspaceSnapshot): string[] {
  return [
    ...snapshot.pages.flatMap((page) => (page.id ? [page.id] : [])),
    ...Object.keys(snapshot.entities ?? {}),
    ...(snapshot.edges ?? []).map((edge) => edge.id),
  ]
}

function sortGroupsDepthFirstFrontmostFirst(
  groups: Iterable<GroupDraft>,
  order: readonly string[],
): EntityOrderGroup[] {
  const rank = new Map(order.map((id, index) => [id, index]))
  const byParent = new Map<string | null, GroupDraft[]>()
  const byId = new Map<string, GroupDraft>()
  for (const group of groups) {
    byId.set(group.id, group)
    const siblings = byParent.get(group.parentGroupId) ?? []
    siblings.push(group)
    byParent.set(group.parentGroupId, siblings)
  }

  const compareFrontmost = (a: GroupDraft, b: GroupDraft) =>
    (rank.get(b.id) ?? -1) - (rank.get(a.id) ?? -1)
  for (const siblings of byParent.values()) siblings.sort(compareFrontmost)

  const visited = new Set<string>()
  const result: EntityOrderGroup[] = []
  const visit = (group: GroupDraft) => {
    if (visited.has(group.id)) return
    visited.add(group.id)
    result.push({
      id: group.id,
      parentGroupId: group.parentGroupId,
      childIds: [...group.childIds],
    })
    for (const child of byParent.get(group.id) ?? []) visit(child)
  }

  for (const root of byParent.get(null) ?? []) visit(root)
  for (const group of [...byId.values()].sort(compareFrontmost)) visit(group)
  return result
}

function groupsForSnapshot(snapshot: WorkspaceSnapshot, order: readonly string[]): EntityOrderGroup[] {
  const groups = new Map<string, GroupDraft>()

  for (const group of snapshot.groups ?? []) addWorkspaceGroup(groups, group)

  for (const entity of Object.values(snapshot.entities ?? {})) {
    if (entity.kind === 'group') {
      ensureGroup(groups, entity.id, entity.parentGroupId ?? null)
    }
  }

  for (const page of snapshot.pages) {
    if (page.id) addChild(groups, parentGroupIdForPage(page), page.id)
  }

  for (const entity of Object.values(snapshot.entities ?? {})) {
    addChild(groups, parentGroupIdForEntity(entity), entity.id)
  }

  return sortGroupsDepthFirstFrontmostFirst(groups.values(), order)
}

export function migrateSnapshotEntityOrderForRestore(snapshot: WorkspaceSnapshot): boolean {
  const order = snapshot.entityOrder ?? defaultSnapshotOrder(snapshot)
  const nextOrder = enforceGroupContiguity(order, groupsForSnapshot(snapshot, order))
  if (JSON.stringify(order) === JSON.stringify(nextOrder)) return false
  snapshot.entityOrder = nextOrder
  return true
}
