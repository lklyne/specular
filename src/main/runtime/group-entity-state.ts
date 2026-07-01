import { randomUUID } from 'crypto'
import type {
  CanvasSceneGroupEntity,
  PersistedGroupEntity,
  WorkspaceGroup,
  WorkspaceGroupLayoutMode,
} from '../../shared/types'
import { workspaceGroups } from './workspace-model'
import { markDirty } from './layout-dirty'

export const DEFAULT_GROUP_WIDTH = 240
export const DEFAULT_GROUP_HEIGHT = 180
export const MIN_GROUP_WIDTH = 120
export const MIN_GROUP_HEIGHT = 80

export function createGroupEntity(input: {
  id?: string
  label?: string
  color?: string
  canvasX: number
  canvasY: number
  width?: number
  height?: number
  parentGroupId?: string
  layoutMode?: WorkspaceGroupLayoutMode
  managedLayout?: boolean
  sourceTaskId?: string
  metadata?: Record<string, unknown>
}): WorkspaceGroup {
  const group: WorkspaceGroup = {
    id: input.id ?? `group_${randomUUID()}`,
    kind: 'group',
    label: input.label ?? 'Group',
    color: input.color,
    canvasX: input.canvasX,
    canvasY: input.canvasY,
    width: input.width ?? DEFAULT_GROUP_WIDTH,
    height: input.height ?? DEFAULT_GROUP_HEIGHT,
    parentGroupId: input.parentGroupId,
    layoutMode: input.layoutMode ?? 'freeform',
    managedLayout: input.managedLayout ?? false,
    sourceTaskId: input.sourceTaskId,
    metadata: input.metadata ? { ...input.metadata } : undefined,
  }
  workspaceGroups.push(group)
  markDirty('canvas', 'sidebar')
  return group
}

export function updateGroupEntity(
  id: string,
  patch: Partial<Omit<WorkspaceGroup, 'id' | 'kind'>>,
): WorkspaceGroup | null {
  const group = workspaceGroups.find((candidate) => candidate.id === id)
  if (!group) return null
  if (patch.label !== undefined) group.label = patch.label
  if (patch.color !== undefined) group.color = patch.color
  if (patch.canvasX !== undefined) group.canvasX = patch.canvasX
  if (patch.canvasY !== undefined) group.canvasY = patch.canvasY
  if (patch.width !== undefined) group.width = patch.width
  if (patch.height !== undefined) group.height = patch.height
  if (patch.parentGroupId !== undefined) group.parentGroupId = patch.parentGroupId
  if (patch.layoutMode !== undefined) group.layoutMode = patch.layoutMode
  if (patch.managedLayout !== undefined) group.managedLayout = patch.managedLayout
  if (patch.sourceTaskId !== undefined) group.sourceTaskId = patch.sourceTaskId
  if (patch.metadata !== undefined) {
    group.metadata = patch.metadata ? { ...patch.metadata } : undefined
  }
  markDirty('canvas', 'sidebar')
  return group
}

export function deleteGroupEntity(id: string): boolean {
  const index = workspaceGroups.findIndex((candidate) => candidate.id === id)
  if (index === -1) return false
  workspaceGroups.splice(index, 1)
  markDirty('canvas', 'sidebar')
  return true
}

export function clearGroupEntities(): void {
  workspaceGroups.length = 0
}

export function buildGroupSceneEntity(
  group: WorkspaceGroup,
  entityIds: string[],
): CanvasSceneGroupEntity {
  return {
    kind: 'group',
    id: group.id,
    label: group.label,
    color: group.color,
    canvasX: group.canvasX,
    canvasY: group.canvasY,
    width: group.width,
    height: group.height,
    parentGroupId: group.parentGroupId,
    layoutMode: group.layoutMode,
    managedLayout: group.managedLayout,
    entityIds,
  }
}

export function persistGroupEntity(group: WorkspaceGroup): PersistedGroupEntity {
  return {
    id: group.id,
    kind: 'group',
    label: group.label,
    color: group.color,
    canvasX: group.canvasX,
    canvasY: group.canvasY,
    width: group.width,
    height: group.height,
    parentGroupId: group.parentGroupId,
    layoutMode: group.layoutMode,
    managedLayout: group.managedLayout,
    sourceTaskId: group.sourceTaskId,
    metadata: group.metadata ? { ...group.metadata } : undefined,
  }
}
