/**
 * Sidebar tree builder — constructs hierarchical sidebar data for the left panel.
 */

import type {
  LeftSidebarData,
  LeftSidebarSections,
  SidebarCanvasItem,
  SidebarDrawingItem,
  SidebarFileItem,
  SidebarPageItem,
  SidebarShapeItem,
  SidebarTextItem,
  WorkspaceBounds,
  WorkspaceGroup,
} from '../../shared/types'
import {
  findPageById,
  interactionState,
  pages,
} from './runtime-context'
import { activeWorkspaceTabId, workspaceGroups } from './workspace-model'
import { leftSidebarView } from './view-refs'
import {
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedEntityIds as uiSelectedEntityIds,
  selectedGroupId as uiSelectedGroupId,
  workspaceViewMode as uiWorkspaceViewMode,
} from '../ui-state'
import { textEntities } from './text-entity-state'
import { fileEntities } from './file-entity-state'
import { drawingEntitiesForUi } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { pageDisplayLabel } from './runtime-serialization'
import { workspaceTabSummaries } from './workspace-tabs'
import { LEFT_SIDEBAR_WIDTH } from './runtime-constants'
import { DOC_ARRAY_ENTITY_ORDER, getActiveDoc } from './workspace-doc'

type SidebarLeafItem =
  | SidebarPageItem
  | SidebarTextItem
  | SidebarFileItem
  | SidebarDrawingItem
  | SidebarShapeItem
type SidebarNodeBuild = {
  group: WorkspaceGroup
  bounds: WorkspaceBounds
  parentId: string | null
  childGroupIds: string[]
}
type SortableSidebarItem = SidebarCanvasItem & {
  sortKey: number
}

function entityOrderRank(): Map<string, number> {
  return new Map(
    getActiveDoc().getArray<string>(DOC_ARRAY_ENTITY_ORDER).toArray()
      .map((id, index) => [id, index]),
  )
}

function sortSidebarItems(items: SortableSidebarItem[]): SidebarCanvasItem[] {
  return items
    .sort((a, b) => b.sortKey - a.sortKey)
    .map(({ sortKey: _sortKey, ...item }) => item)
}

function buildSidebarLeafItem(
  entityId: string,
  ranks: Map<string, number>,
): (SidebarLeafItem & { sortKey: SortableSidebarItem['sortKey'] }) | null {
  const page = findPageById(entityId)
  if (page) {
    return {
      kind: 'page',
      id: entityId,
      label: pageDisplayLabel(page),
      faviconUrl: page.faviconUrl ?? null,
      width: page.peekWidth,
      height: page.peekHeight,
      sortKey: ranks.get(entityId) ?? Number.MAX_SAFE_INTEGER,
    }
  }

  const te = textEntities.find((entity) => entity.id === entityId)
  if (te) {
    return {
      kind: 'text',
      id: entityId,
      label: te.label || te.text || 'Text',
      color: te.color,
      sortKey: ranks.get(entityId) ?? Number.MAX_SAFE_INTEGER,
    }
  }

  const fe = fileEntities.find((entity) => entity.id === entityId)
  if (fe) {
    const fileName = fe.file.split('/').pop() ?? fe.file
    const displayName = fileName
      .replace(/\.wireframe\.json$/i, '')
      .replace(/\.md$/i, '')
    return {
      kind: 'file',
      id: entityId,
      label: displayName,
      file: fe.file,
      sortKey: ranks.get(entityId) ?? Number.MAX_SAFE_INTEGER,
    }
  }

  const de = drawingEntitiesForUi().find((entity) => entity.id === entityId)
  if (de) {
    const defaultLabel = `Drawing (${de.strokes.length} stroke${de.strokes.length === 1 ? '' : 's'})`
    return {
      kind: 'drawing',
      id: entityId,
      label: de.label || defaultLabel,
      strokeCount: de.strokes.length,
      sortKey: ranks.get(entityId) ?? Number.MAX_SAFE_INTEGER,
    }
  }

  const se = shapeEntities.find((entity) => entity.id === entityId)
  if (se) {
    const trimmed = se.text.trim()
    const defaultLabel =
      se.shapeKind === 'ellipse' ? 'Ellipse' : se.shapeKind === 'diamond' ? 'Diamond' : 'Rectangle'
    return {
      kind: 'shape',
      id: entityId,
      label: se.label || trimmed || defaultLabel,
      shapeKind: se.shapeKind,
      sortKey: ranks.get(entityId) ?? Number.MAX_SAFE_INTEGER,
    }
  }

  return null
}

function countSidebarLeafDescendants(groupId: string): number {
  const directLeafCount =
    pages.filter((page) => page.parentGroupId === groupId).length +
    textEntities.filter((entity) => entity.parentGroupId === groupId).length +
    fileEntities.filter((entity) => entity.parentGroupId === groupId).length +
    drawingEntitiesForUi().filter((entity) => entity.parentGroupId === groupId).length +
    shapeEntities.filter((entity) => entity.parentGroupId === groupId).length

  const nestedLeafCount = workspaceGroups
    .filter((group) => group.parentGroupId === groupId)
    .reduce((total, group) => total + countSidebarLeafDescendants(group.id), 0)

  return directLeafCount + nestedLeafCount
}

export function buildSidebarItems(): SidebarCanvasItem[] {
  const sections = buildSidebarSections()
  return [...sections.notes, ...sections.pages]
}

export function buildSidebarSections(): LeftSidebarSections {
  const userGroups = workspaceGroups
  const ranks = entityOrderRank()

  const nodeById = new Map<string, SidebarNodeBuild>(
    userGroups.map((group) => [
      group.id,
      {
        group,
        bounds: { x: group.canvasX, y: group.canvasY, width: group.width, height: group.height },
        parentId: group.parentGroupId ?? null,
        childGroupIds: [],
      },
    ]),
  )

  const groupNodes = Array.from(nodeById.values())
  for (const node of groupNodes) {
    if (node.parentId) {
      const parent = nodeById.get(node.parentId)
      if (parent) {
        parent.childGroupIds.push(node.group.id)
      }
    }
  }

  function buildGroupItem(groupId: string): SortableSidebarItem | null {
    const node = nodeById.get(groupId)
    if (!node) return null

    const childGroups = node.childGroupIds
      .map(buildGroupItem)
      .filter((item): item is SortableSidebarItem => Boolean(item))
    const directLeafItems = [
      ...pages.filter((page) => page.parentGroupId === node.group.id).map((page) => page.id),
      ...textEntities.filter((entity) => entity.parentGroupId === node.group.id).map((entity) => entity.id),
      ...fileEntities.filter((entity) => entity.parentGroupId === node.group.id).map((entity) => entity.id),
      ...drawingEntitiesForUi().filter((entity) => entity.parentGroupId === node.group.id).map((entity) => entity.id),
      ...shapeEntities.filter((entity) => entity.parentGroupId === node.group.id).map((entity) => entity.id),
    ]
      .map((id) => buildSidebarLeafItem(id, ranks))
      .filter((item): item is NonNullable<typeof item> => Boolean(item))

    return {
      kind: 'group',
      id: node.group.id,
      label: node.group.label,
      entityCount: countSidebarLeafDescendants(node.group.id),
      children: sortSidebarItems([...childGroups, ...directLeafItems]),
      sortKey: ranks.get(node.group.id) ?? Number.MAX_SAFE_INTEGER,
    }
  }

  const groupedEntityIds = new Set<string>([
    ...pages.filter((page) => page.parentGroupId).map((page) => page.id),
    ...textEntities.filter((entity) => entity.parentGroupId).map((entity) => entity.id),
    ...fileEntities.filter((entity) => entity.parentGroupId).map((entity) => entity.id),
    ...drawingEntitiesForUi().filter((entity) => entity.parentGroupId).map((entity) => entity.id),
    ...shapeEntities.filter((entity) => entity.parentGroupId).map((entity) => entity.id),
  ])
  const rootLeafItems = [
    ...pages
      .filter((page) => !groupedEntityIds.has(page.id))
      .map((page) => buildSidebarLeafItem(page.id, ranks)),
    ...textEntities
      .filter((entity) => !groupedEntityIds.has(entity.id))
      .map((entity) => buildSidebarLeafItem(entity.id, ranks)),
    ...fileEntities
      .filter((entity) => !groupedEntityIds.has(entity.id))
      .map((entity) => buildSidebarLeafItem(entity.id, ranks)),
    ...drawingEntitiesForUi()
      .filter((entity) => !groupedEntityIds.has(entity.id))
      .map((entity) => buildSidebarLeafItem(entity.id, ranks)),
    ...shapeEntities
      .filter((entity) => !groupedEntityIds.has(entity.id))
      .map((entity) => buildSidebarLeafItem(entity.id, ranks)),
  ].filter((item): item is NonNullable<typeof item> => Boolean(item))

  const rootGroupItems = groupNodes
    .filter((node) => !node.parentId)
    .map((node) => buildGroupItem(node.group.id))
    .filter((item): item is SortableSidebarItem => Boolean(item))

  const items = sortSidebarItems([...rootLeafItems, ...rootGroupItems])
  return {
    notes: partitionSidebarItems(items, 'notes'),
    pages: partitionSidebarItems(items, 'pages'),
  }
}

function partitionSidebarItems(items: SidebarCanvasItem[], section: 'notes' | 'pages'): SidebarCanvasItem[] {
  const result: SidebarCanvasItem[] = []
  for (const item of items) {
    if (item.kind === 'group') {
      const children = partitionSidebarItems(item.children, section)
      if (children.length) result.push({ ...item, children, entityCount: countLeaves(children) })
      continue
    }
    if (section === 'pages' && item.kind === 'page') result.push(item)
    if (section === 'notes' && item.kind !== 'page') result.push(item)
  }
  return result
}

function countLeaves(items: SidebarCanvasItem[]): number {
  let count = 0
  for (const item of items) {
    if (item.kind === 'group') count += countLeaves(item.children)
    else count += 1
  }
  return count
}

export function buildLeftSidebarData(): LeftSidebarData {
  const sections = buildSidebarSections()
  return {
    width: uiLeftSidebarOpen() ? LEFT_SIDEBAR_WIDTH : 0,
    selectedEntityIds: uiSelectedEntityIds(),
    selectedGroupId: uiSelectedGroupId(),
    tabs: workspaceTabSummaries(),
    activeTabId: activeWorkspaceTabId,
    viewMode: uiWorkspaceViewMode(),
    hasPages: pages.length > 0,
    sections,
    items: [...sections.notes, ...sections.pages],
  }
}

export function getLeftSidebarData(): LeftSidebarData {
  return buildLeftSidebarData()
}

export function notifyLeftSidebarData(): void {
  if (!leftSidebarView) return
  const wc = leftSidebarView.webContents
  if (wc.isDestroyed()) return
  if (interactionState.kind === 'dragging-entities') return
  wc.send('left-sidebar-data', buildLeftSidebarData())
}
