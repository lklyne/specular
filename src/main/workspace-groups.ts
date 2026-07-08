import type {
  DeleteGroupsRequest,
  DeleteGroupsResponse,
  WorkspaceBounds,
  WorkspaceGroup,
} from '../shared/types'
import {
  CLUSTER_HORIZONTAL_GUTTER,
  USER_GROUP_PADDING,
} from '../shared/constants'
import {
  createPage,
  findPageById,
  pages,
} from './runtime/page-runtime'
import {
  deselectAll,
  focusCanvasBounds,
  getSelectedGroupId,
  selectPageById,
  setSelectedGroupId,
} from './runtime/ui-actions'
import { textEntities, createTextEntity as createTextEntityInState } from './runtime/text-entity-state'
import { fileEntities, createFileEntity as createFileEntityInState } from './runtime/file-entity-state'
import { shapeEntities, createShapeEntity as createShapeEntityInState } from './runtime/shape-entity-state'
import { drawingEntities, createDrawingEntity as createDrawingEntityInState } from './runtime/drawing-entity-state'
import { snapToGrid } from '../shared/gesture-utils'
import { workspaceEdges, workspaceGroups } from './runtime/workspace-model'
import { scheduleWorkspaceAutosave } from './runtime/workspace-autosave'
import { markDirty } from './runtime/layout-dirty'
import { mutateWorkspace } from './runtime/mutate-workspace'
import { makeId, cloneMetadata, pageCurrentUrl } from './workspace-utils'
import {
  deletePages,
  entityBoundsById,
  groupBounds,
  groupBoundsForEntityIds,
  groupById,
  groupChildIds,
  groupDescendantIds,
  selectionBounds,
  unionBounds,
  pageBoundsById,
} from './workspace-entities'
import { findDuplicatePlacement } from './workspace-placement'
import { cancelEditingEntityIfMatches } from './runtime/editing-entity-runtime'

// --- Entity parent-group helpers ---

function getEntityParentGroupId(entityId: string): string | undefined {
  return (
    findPageById(entityId)?.parentGroupId ??
    textEntities.find((entity) => entity.id === entityId)?.parentGroupId ??
    fileEntities.find((entity) => entity.id === entityId)?.parentGroupId ??
    shapeEntities.find((entity) => entity.id === entityId)?.parentGroupId ??
    drawingEntities.find((entity) => entity.id === entityId)?.parentGroupId ??
    groupById(entityId)?.parentGroupId
  )
}

function setEntityParentGroupId(entityId: string, parentGroupId: string | undefined): void {
  const page = findPageById(entityId)
  if (page) { page.parentGroupId = parentGroupId; return }
  const textEntity = textEntities.find((entity) => entity.id === entityId)
  if (textEntity) { textEntity.parentGroupId = parentGroupId; return }
  const fileEntity = fileEntities.find((entity) => entity.id === entityId)
  if (fileEntity) { fileEntity.parentGroupId = parentGroupId; return }
  const shapeEntity = shapeEntities.find((entity) => entity.id === entityId)
  if (shapeEntity) { shapeEntity.parentGroupId = parentGroupId; return }
  const drawingEntity = drawingEntities.find((entity) => entity.id === entityId)
  if (drawingEntity) { drawingEntity.parentGroupId = parentGroupId; return }
  const childGroup = groupById(entityId)
  if (childGroup) childGroup.parentGroupId = parentGroupId
}

// --- Exported group operations ---

export function createUserGroup(entityIds: string[], label?: string): WorkspaceGroup {
  return mutateWorkspace(() => createUserGroupInternal(entityIds, label))
}

function createUserGroupInternal(entityIds: string[], label?: string): WorkspaceGroup {
  const selectionIds = new Set([...new Set(entityIds)].filter(Boolean))

  let changed = true
  while (changed) {
    changed = false
    for (const group of workspaceGroups) {
      const childIds = groupChildIds(group.id)
      if (!childIds.length) continue
      if (!childIds.every((childId) => selectionIds.has(childId))) continue
      let removedChild = false
      for (const childId of childIds) {
        removedChild = selectionIds.delete(childId) || removedChild
      }
      if (removedChild) {
        selectionIds.add(group.id)
        changed = true
      }
    }
  }

  const normalizedIds = [...selectionIds]
  const roots = normalizedIds.filter((candidateId) => {
    let currentParentId = getEntityParentGroupId(candidateId)
    while (currentParentId) {
      if (normalizedIds.includes(currentParentId)) return false
      currentParentId = groupById(currentParentId)?.parentGroupId
    }
    return true
  })
  const contentBounds = groupBoundsForEntityIds(roots) ?? {
    x: 0,
    y: 0,
    width: USER_GROUP_PADDING * 2,
    height: USER_GROUP_PADDING * 2,
  }
  const parentGroupId = getEntityParentGroupId(roots[0])
  const group: WorkspaceGroup = {
    id: makeId('group'),
    kind: 'group',
    label: label ?? 'Group',
    canvasX: contentBounds.x - USER_GROUP_PADDING,
    canvasY: contentBounds.y - USER_GROUP_PADDING,
    width: Math.max(USER_GROUP_PADDING * 2, contentBounds.width + USER_GROUP_PADDING * 2),
    height: Math.max(USER_GROUP_PADDING * 2, contentBounds.height + USER_GROUP_PADDING * 2),
    parentGroupId,
    layoutMode: 'freeform',
    managedLayout: false,
  }
  workspaceGroups.push(group)
  for (const entityId of roots) {
    setEntityParentGroupId(entityId, group.id)
  }
  markDirty('canvas', 'sidebar')
  return group
}

export function ungroupUserGroup(groupId: string): string[] {
  const idx = workspaceGroups.findIndex((g) => g.id === groupId)
  if (idx === -1) return []
  return mutateWorkspace(() => {
    const group = workspaceGroups[idx]
    const freedIds = groupChildIds(groupId)
    for (const entityId of freedIds) {
      setEntityParentGroupId(entityId, group.parentGroupId)
    }
    workspaceGroups.splice(idx, 1)
    markDirty('canvas', 'sidebar')
    return freedIds
  })
}

export function removeEntityFromUserGroups(entityId: string): void {
  setEntityParentGroupId(entityId, undefined)
}

export function duplicateGroup(input: {
  groupId: string
  focus?: boolean
  placement?: { canvasX: number; canvasY: number }
}): { groupId: string; entityIds: string[] } {
  const sourceGroup = groupById(input.groupId)
  if (!sourceGroup) {
    throw new Error(`Unknown group: ${input.groupId}`)
  }
  return mutateWorkspace(() => duplicateGroupInternal(input, sourceGroup))
}

function duplicateGroupInternal(
  input: { groupId: string; focus?: boolean; placement?: { canvasX: number; canvasY: number } },
  sourceGroup: WorkspaceGroup,
): { groupId: string; entityIds: string[] } {

  const sourceBounds = groupBounds(sourceGroup)
  const placement = input.placement
    ? { canvasX: snapToGrid(input.placement.canvasX), canvasY: snapToGrid(input.placement.canvasY) }
    : sourceBounds
      ? findDuplicatePlacement(sourceBounds)
      : { canvasX: sourceGroup.canvasX + CLUSTER_HORIZONTAL_GUTTER, canvasY: sourceGroup.canvasY }
  const offsetX = placement.canvasX - sourceGroup.canvasX
  const offsetY = placement.canvasY - sourceGroup.canvasY

  const groupIdMap = new Map<string, string>()
  const entityIdMap = new Map<string, string>()
  const duplicatedEntityIds: string[] = []

  const cloneGroupTree = (group: WorkspaceGroup, parentGroupId?: string): WorkspaceGroup => {
    const childGroups = workspaceGroups.filter((candidate) => candidate.parentGroupId === group.id)
    const clonedGroup: WorkspaceGroup = {
      ...group,
      id: makeId('group'),
      canvasX: snapToGrid(group.canvasX + offsetX),
      canvasY: snapToGrid(group.canvasY + offsetY),
      parentGroupId,
      metadata: cloneMetadata(group.metadata),
    }
    workspaceGroups.push(clonedGroup)
    groupIdMap.set(group.id, clonedGroup.id)

    const childPages = pages.filter((page) => page.parentGroupId === group.id)
    for (const page of childPages) {
      const duplicatedPage = createPage({
        name: page.name,
        url: pageCurrentUrl(page.id) ?? page.url ?? 'about:blank',
        presetIndex: page.presetIndex,
        canvasX: snapToGrid(page.canvasX + offsetX),
        canvasY: snapToGrid(page.canvasY + offsetY),
        syncId: null,
        suppressInitialNavigationBroadcast: true,
        source: page.source,
        parentGroupId: clonedGroup.id,
        metadata: cloneMetadata(page.metadata),
        colorScheme: page.colorScheme,
      })
      entityIdMap.set(page.id, duplicatedPage.id)
      duplicatedEntityIds.push(duplicatedPage.id)
    }

    const childTextEntities = textEntities.filter((entity) => entity.parentGroupId === group.id)
    for (const entity of childTextEntities) {
      const duplicatedEntity = createTextEntityInState({
        canvasX: snapToGrid(entity.canvasX + offsetX),
        canvasY: snapToGrid(entity.canvasY + offsetY),
        text: entity.text,
        color: entity.color,
        textStyle: entity.textStyle,
        width: entity.width,
        height: entity.height,
        parentGroupId: clonedGroup.id,
      })
      entityIdMap.set(entity.id, duplicatedEntity.id)
      duplicatedEntityIds.push(duplicatedEntity.id)
    }

    const childFileEntities = fileEntities.filter((entity) => entity.parentGroupId === group.id)
    for (const entity of childFileEntities) {
      const duplicatedEntity = createFileEntityInState({
        canvasX: snapToGrid(entity.canvasX + offsetX),
        canvasY: snapToGrid(entity.canvasY + offsetY),
        file: entity.file,
        subpath: entity.subpath,
        width: entity.width,
        height: entity.height,
        parentGroupId: clonedGroup.id,
        presetIndex: entity.presetIndex,
        metadata: entity.metadata ? { ...entity.metadata } : undefined,
        objectFit: entity.objectFit,
      })
      entityIdMap.set(entity.id, duplicatedEntity.id)
      duplicatedEntityIds.push(duplicatedEntity.id)
    }

    const childShapeEntities = shapeEntities.filter((entity) => entity.parentGroupId === group.id)
    for (const entity of childShapeEntities) {
      const duplicatedEntity = createShapeEntityInState({
        canvasX: snapToGrid(entity.canvasX + offsetX),
        canvasY: snapToGrid(entity.canvasY + offsetY),
        shapeKind: entity.shapeKind,
        text: entity.text,
        color: entity.color,
        strokeWidth: entity.strokeWidth,
        theme: entity.theme,
        width: entity.width,
        height: entity.height,
        parentGroupId: clonedGroup.id,
        label: entity.label,
      })
      entityIdMap.set(entity.id, duplicatedEntity.id)
      duplicatedEntityIds.push(duplicatedEntity.id)
    }

    const childDrawingEntities = drawingEntities.filter((entity) => entity.parentGroupId === group.id)
    for (const entity of childDrawingEntities) {
      const duplicatedEntity = createDrawingEntityInState({
        canvasX: snapToGrid(entity.canvasX + offsetX),
        canvasY: snapToGrid(entity.canvasY + offsetY),
        width: entity.width,
        height: entity.height,
        strokes: entity.strokes.map((stroke) => ({
          ...stroke,
          id: `${stroke.id}_dup_${Math.random().toString(36).slice(2, 8)}`,
          points: stroke.points.map((point) => ({ x: point.x + offsetX, y: point.y + offsetY })),
        })),
        parentGroupId: clonedGroup.id,
        label: entity.label,
      })
      entityIdMap.set(entity.id, duplicatedEntity.id)
      duplicatedEntityIds.push(duplicatedEntity.id)
    }

    for (const childGroup of childGroups) {
      cloneGroupTree(childGroup, clonedGroup.id)
    }

    return clonedGroup
  }

  const duplicatedRootGroup = cloneGroupTree(sourceGroup, sourceGroup.parentGroupId)
  const descendantIds = new Set([sourceGroup.id, ...groupDescendantIds(sourceGroup.id)])
  const sourceEdges = [...workspaceEdges]

  for (const edge of sourceEdges) {
    if (!descendantIds.has(edge.fromEntityId) || !descendantIds.has(edge.toEntityId)) continue
    const fromEntityId = entityIdMap.get(edge.fromEntityId) ?? groupIdMap.get(edge.fromEntityId)
    const toEntityId = entityIdMap.get(edge.toEntityId) ?? groupIdMap.get(edge.toEntityId)
    if (!fromEntityId || !toEntityId) continue
    workspaceEdges.push({
      ...edge,
      id: makeId('edge'),
      fromEntityId,
      toEntityId,
      metadata: cloneMetadata(edge.metadata),
    })
  }

  if (input.focus ?? true) {
    deselectAll()
    setSelectedGroupId(duplicatedRootGroup.id)
  }

  return {
    groupId: duplicatedRootGroup.id,
    entityIds: duplicatedEntityIds,
  }
}

export function deleteGroups(input: DeleteGroupsRequest): DeleteGroupsResponse {
  return mutateWorkspace(() => deleteGroupsInternal(input), {
    changed: (result) =>
      result.deletedGroupIds.length > 0 ||
      result.deletedPageIds.length > 0 ||
      result.deletedEdgeIds.length > 0,
  })
}

function deleteGroupsInternal(input: DeleteGroupsRequest): DeleteGroupsResponse {
  const deletedGroupIds: string[] = []
  const deletedPageIds: string[] = []
  const deletedEdgeIds: string[] = []
  const missingGroupIds: string[] = []

  for (const groupId of input.groupIds) {
    cancelEditingEntityIfMatches(groupId)
    const idx = workspaceGroups.findIndex((group) => group.id === groupId)
    if (idx === -1) {
      missingGroupIds.push(groupId)
      continue
    }
    const group = workspaceGroups[idx]
    if (input.deleteMemberPages ?? true) {
      const pageDeletion = deletePages({
        pageIds: pages.filter((page) => page.parentGroupId === group.id).map((page) => page.id),
      })
      deletedPageIds.push(...pageDeletion.deletedPageIds)
      deletedEdgeIds.push(...pageDeletion.deletedEdgeIds)
    } else {
      for (const pageId of pages.filter((page) => page.parentGroupId === group.id).map((page) => page.id)) {
        const page = findPageById(pageId)
        if (page) {
          page.parentGroupId = undefined
          page.groupId = undefined
        }
      }
    }
    const existingIdx = workspaceGroups.findIndex((candidate) => candidate.id === groupId)
    if (existingIdx !== -1) {
      workspaceGroups.splice(existingIdx, 1)
    }
    deletedGroupIds.push(groupId)
  }
  if (deletedGroupIds.includes(getSelectedGroupId() ?? '')) {
    setSelectedGroupId(null)
  }

  if (input.focusAfter) {
    const bounds = selectionBounds()
    if (bounds) focusCanvasBounds(bounds)
  }

  return {
    deletedGroupIds,
    deletedPageIds,
    deletedEdgeIds,
    missingGroupIds,
    warnings: missingGroupIds.length
      ? [`Missing group IDs: ${missingGroupIds.join(', ')}`]
      : [],
  }
}

export function focusTargets(input: {
  pageIds?: string[]
  groupIds?: string[]
  bounds?: WorkspaceBounds
}): { focused: boolean } {
  if (input.bounds) {
    focusCanvasBounds(input.bounds)
    scheduleWorkspaceAutosave()
    return { focused: true }
  }

  if (input.groupIds?.length) {
    const groups = input.groupIds
      .map(groupById)
      .filter((group): group is WorkspaceGroup => Boolean(group))
    const bounds = unionBounds(
      groups
        .map((group) => groupBounds(group))
        .filter((item): item is WorkspaceBounds => item !== null),
    )
    if (bounds) {
      setSelectedGroupId(groups[0].id)
      focusCanvasBounds(bounds)
      scheduleWorkspaceAutosave()
      return { focused: true }
    }
  }

  if (input.pageIds?.length) {
    const bounds = unionBounds(
      input.pageIds
        .map(pageBoundsById)
        .filter((item): item is WorkspaceBounds => item !== null),
    )
    if (bounds) {
      selectPageById(input.pageIds[0])
      focusCanvasBounds(bounds)
      scheduleWorkspaceAutosave()
      return { focused: true }
    }
  }

  return { focused: false }
}
