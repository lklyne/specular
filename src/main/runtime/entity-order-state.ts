import type { SidebarSectionKey } from '../../shared/types'
import {
  appendAtTop,
  bringToFront,
  enforceGroupContiguity,
  moveBlockBefore,
  moveBackward,
  moveForward,
  replaceSubsequence,
  sendToBack,
  type MovePosition,
} from '../../shared/entity-order-math'
import { allEntities } from '../entities/contract'
import { getActiveDoc, DOC_ARRAY_ENTITY_ORDER } from './space-doc'
import { markDirty } from './layout-dirty'
import { mutateWorkspace } from './mutate-workspace'
import { selectedEntityIds as uiSelectedEntityIds, selectedGroupId as uiSelectedGroupId } from '../ui-state'
import { scheduleSpaceAutosave } from './space-autosave'
import { workspaceEdges, workspaceGroups } from './space-model'

type EntityKindForOrder = 'page' | 'text' | 'file' | 'drawing' | 'shape' | 'group' | 'edge'
export type StackOrderAction = 'bring-forward' | 'send-backward' | 'bring-to-front' | 'send-to-back'

function defaultEntityOrder(): string[] {
  return [
    ...allEntities().map(({ entity }) => entity.id),
    ...workspaceEdges.map((edge) => edge.id),
  ]
}

export function currentEntityIds(): Set<string> {
  return new Set(defaultEntityOrder())
}

export function currentEntityOrder(): string[] {
  const currentIds = currentEntityIds()
  const seen = new Set<string>()
  const order: string[] = []
  for (const id of getActiveDoc().getArray<string>(DOC_ARRAY_ENTITY_ORDER).toArray()) {
    if (!currentIds.has(id) || seen.has(id)) continue
    seen.add(id)
    order.push(id)
  }
  for (const id of defaultEntityOrder()) {
    if (seen.has(id)) continue
    seen.add(id)
    order.push(id)
  }
  return order
}

export function currentEntityOrderRank(): Map<string, number> {
  return new Map(currentEntityOrder().map((id, index) => [id, index]))
}

export function writeEntityOrder(nextOrder: readonly string[]): void {
  const currentIds = currentEntityIds()
  const seen = new Set<string>()
  const sanitized: string[] = []
  for (const id of nextOrder) {
    if (!currentIds.has(id) || seen.has(id)) continue
    seen.add(id)
    sanitized.push(id)
  }
  for (const id of defaultEntityOrder()) {
    if (seen.has(id)) continue
    seen.add(id)
    sanitized.push(id)
  }

  const order = getActiveDoc().getArray<string>(DOC_ARRAY_ENTITY_ORDER)
  if (JSON.stringify(order.toArray()) === JSON.stringify(sanitized)) return
  getActiveDoc().transact(() => {
    order.delete(0, order.length)
    if (sanitized.length) order.push(sanitized)
  }, 'user')
}

function entityKindById(id: string): EntityKindForOrder | null {
  const found = allEntities().find(({ entity }) => entity.id === id)
  if (found) return found.kind
  if (workspaceEdges.some((edge) => edge.id === id)) return 'edge'
  return null
}

function parentGroupIdById(id: string): string | null {
  return allEntities().find(({ entity }) => entity.id === id)?.entity.parentGroupId ?? null
}

function directChildIds(groupId: string): string[] {
  return allEntities()
    .filter(({ entity }) => entity.parentGroupId === groupId)
    .map(({ entity }) => entity.id)
}

function descendantIds(groupId: string): string[] {
  const ids: string[] = []
  for (const childId of directChildIds(groupId)) {
    ids.push(childId)
    if (entityKindById(childId) === 'group') ids.push(...descendantIds(childId))
  }
  return ids
}

function groupHasSection(groupId: string, section: SidebarSectionKey): boolean {
  return descendantIds(groupId).some((id) => isInSidebarSection(id, section))
}

export function isInSidebarSection(id: string, section: SidebarSectionKey): boolean {
  const kind = entityKindById(id)
  if (!kind) return false
  if (kind === 'edge') return false
  if (kind === 'group') return groupHasSection(id, section)
  if (section === 'pages') return kind === 'page'
  return kind === 'text' || kind === 'file' || kind === 'drawing' || kind === 'shape'
}

function selectedBlockForDrag(draggedId: string, section: SidebarSectionKey, parentId: string | null): string[] {
  const selectedIds = uiSelectedEntityIds()
  const groupId = uiSelectedGroupId()
  const selectedBlock = selectedIds.includes(draggedId)
    ? selectedIds
    : groupId === draggedId
      ? [groupId]
      : [draggedId]
  return selectedBlock.filter(
    (id) => isInSidebarSection(id, section) && parentGroupIdById(id) === parentId,
  )
}

function groupsForContiguity() {
  return workspaceGroups.map((group) => ({
    id: group.id,
    parentGroupId: group.parentGroupId ?? null,
    childIds: directChildIds(group.id),
  }))
}

function selectedBlockForStackOrder(targetId?: string): string[] {
  const selectedIds = uiSelectedEntityIds()
  const groupId = uiSelectedGroupId()
  if (targetId) {
    if (groupId === targetId) return [targetId]
    if (selectedIds.includes(targetId)) return selectedIds
    return [targetId]
  }
  if (groupId) return [groupId]
  return selectedIds
}

function applyStackOrderAction(
  order: readonly string[],
  ids: readonly string[],
  action: StackOrderAction,
): string[] {
  switch (action) {
    case 'bring-forward':
      return moveForward(order, ids)
    case 'send-backward':
      return moveBackward(order, ids)
    case 'bring-to-front':
      return bringToFront(order, ids)
    case 'send-to-back':
      return sendToBack(order, ids)
  }
}

export function reorderStackOrder(action: StackOrderAction, targetId?: string): boolean {
  const order = currentEntityOrder()
  const currentIds = currentEntityIds()
  const block = selectedBlockForStackOrder(targetId).filter((id) => currentIds.has(id))
  if (!block.length) return false

  const nextOrder = enforceGroupContiguity(
    applyStackOrderAction(order, block, action),
    groupsForContiguity(),
  )
  if (JSON.stringify(order) === JSON.stringify(nextOrder)) return false

  return mutateWorkspace(() => {
    writeEntityOrder(nextOrder)
    markDirty('canvas', 'sidebar')
    return true
  })
}

export function reorderStackOrderIds(action: StackOrderAction, ids: readonly string[]): boolean {
  const order = currentEntityOrder()
  const currentIds = currentEntityIds()
  const block = ids.filter((id) => currentIds.has(id))
  if (!block.length) return false

  const nextOrder = enforceGroupContiguity(
    applyStackOrderAction(order, block, action),
    groupsForContiguity(),
  )
  if (JSON.stringify(order) === JSON.stringify(nextOrder)) return false

  return mutateWorkspace(() => {
    writeEntityOrder(nextOrder)
    markDirty('canvas', 'sidebar')
    return true
  })
}

/**
 * Direct children of a group in their `entityOrder` run order — the layout
 * sequence of a managed (auto-layout) group (ADR 0015 D2). The order *within the
 * run* is the left-to-right packing order; `canvasX` is an output of reflow, not
 * the key. Returns children regardless of contiguity (the relative order is
 * still well-defined).
 */
export function managedChildOrder(groupId: string): string[] {
  const childIds = new Set(directChildIds(groupId))
  if (!childIds.size) return []
  return currentEntityOrder().filter((id) => childIds.has(id))
}

/**
 * Rewrite a group's run so its direct children follow `orderedChildIds`, then
 * re-enforce group contiguity. `orderedChildIds` must be a permutation of the
 * group's current direct children. Returns true if the order actually changed.
 *
 * This is the order half of a managed reorder; the caller pairs it with a reflow
 * inside one transaction (see `commitAsOneTransaction`) so order + positions
 * collapse to a single undo step.
 */
export function writeManagedChildOrder(
  groupId: string,
  orderedChildIds: readonly string[],
): boolean {
  const childIds = new Set(directChildIds(groupId))
  if (!childIds.size) return false
  const requested = orderedChildIds.filter((id) => childIds.has(id))
  if (requested.length !== childIds.size) return false

  const order = currentEntityOrder()
  const eligible = (id: string) => childIds.has(id)
  const nextOrder = enforceGroupContiguity(
    replaceSubsequence(order, eligible, requested),
    groupsForContiguity(),
  )
  if (JSON.stringify(order) === JSON.stringify(nextOrder)) return false

  writeEntityOrder(nextOrder)
  markDirty('canvas', 'sidebar')
  scheduleSpaceAutosave()
  return true
}

export function appendStackOrderIdsAtTop(ids: readonly string[]): boolean {
  if (!ids.length) return false
  let nextOrder = currentEntityOrder()
  for (const id of ids) {
    nextOrder = appendAtTop(nextOrder, id)
  }
  writeEntityOrder(nextOrder)
  return true
}

export function normalizeGroupStackContiguity(): boolean {
  const order = currentEntityOrder()
  const nextOrder = enforceGroupContiguity(order, groupsForContiguity())
  if (JSON.stringify(order) === JSON.stringify(nextOrder)) return false
  writeEntityOrder(nextOrder)
  return true
}

export function reorderSidebarStackOrder(input: {
  section: SidebarSectionKey
  draggedId: string
  anchorId: string | null
  position: MovePosition
  parentId: string | null
}): boolean {
  const { section, draggedId, anchorId, position, parentId } = input
  if (!isInSidebarSection(draggedId, section)) return false
  if (parentGroupIdById(draggedId) !== parentId) return false
  if (anchorId) {
    if (!isInSidebarSection(anchorId, section)) return false
    if (parentGroupIdById(anchorId) !== parentId) return false
  }

  const order = currentEntityOrder()
  const block = selectedBlockForDrag(draggedId, section, parentId)
  if (!block.length) return false
  const eligible = (id: string) => isInSidebarSection(id, section) && parentGroupIdById(id) === parentId
  const sectionOrder = order.filter(eligible)
  const movedSectionOrder = moveBlockBefore(sectionOrder, block, anchorId, position)
  const nextOrder = enforceGroupContiguity(
    replaceSubsequence(order, eligible, movedSectionOrder),
    groupsForContiguity(),
  )
  if (JSON.stringify(order) === JSON.stringify(nextOrder)) return false

  return mutateWorkspace(() => {
    writeEntityOrder(nextOrder)
    markDirty('canvas', 'sidebar')
    return true
  })
}
