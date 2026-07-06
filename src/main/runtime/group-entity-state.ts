import { randomUUID } from 'crypto'
import type {
  CanvasSceneGroupEntity,
  PersistedGroupEntity,
  WorkspaceGroup,
  WorkspaceGroupLayoutMode,
} from '../../shared/types'
import { workspaceGroups } from './workspace-model'
import { markDirty } from './layout-dirty'
import { applyPatch } from './apply-patch'

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
  applyPatch(group, patch, [
    'label', 'color', 'canvasX', 'canvasY', 'width', 'height',
    'parentGroupId', 'layoutMode', 'layoutGap', 'managedLayout', 'sourceTaskId',
  ])
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
  zoom: number,
  pan: { x: number; y: number },
  canvasOrigin: { x: number; y: number },
  entityIds: string[],
): CanvasSceneGroupEntity {
  const screenX = canvasOrigin.x + group.canvasX * zoom + pan.x
  const screenY = canvasOrigin.y + group.canvasY * zoom + pan.y
  return {
    kind: 'group',
    id: group.id,
    label: group.label,
    color: group.color,
    canvasX: group.canvasX,
    canvasY: group.canvasY,
    width: group.width,
    height: group.height,
    screenX,
    screenY,
    screenWidth: group.width * zoom,
    screenHeight: group.height * zoom,
    parentGroupId: group.parentGroupId,
    layoutMode: group.layoutMode,
    managedLayout: group.managedLayout,
    layoutGap: group.layoutGap,
    entityIds,
  }
}

/**
 * Every key the doc's groups map can hold. Groups mirror to the doc as raw
 * `WorkspaceGroup` objects (identity projection in the forward sync), so the
 * list is the full runtime type keyed by `satisfies` — a new `WorkspaceGroup`
 * field cannot silently skip the declaration (ADR 0024 §5). Restore replaces
 * the array wholesale, so every listed field survives undo.
 */
const WORKSPACE_GROUP_PERSISTED_FIELD_SET = {
  id: true,
  kind: true,
  label: true,
  canvasX: true,
  canvasY: true,
  width: true,
  height: true,
  parentGroupId: true,
  color: true,
  layoutMode: true,
  layoutGap: true,
  managedLayout: true,
  pageIds: true,
  entityIds: true,
  sourceTaskId: true,
  metadata: true,
} as const satisfies Record<keyof WorkspaceGroup, true>

export const WORKSPACE_GROUP_PERSISTED_FIELDS: readonly string[] = Object.keys(
  WORKSPACE_GROUP_PERSISTED_FIELD_SET,
)

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
    layoutGap: group.layoutGap,
    managedLayout: group.managedLayout,
    sourceTaskId: group.sourceTaskId,
    metadata: group.metadata ? { ...group.metadata } : undefined,
  }
}
