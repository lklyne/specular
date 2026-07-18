/**
 * Sidebar tree builder — constructs hierarchical sidebar data for the left panel.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import { shapeDef } from '../../shared/shapes'
import type {
  Annotation,
  LeftSidebarData,
  LeftSidebarSections,
  SidebarAnchoredEntityItem,
  SidebarCanvasItem,
  SidebarDrawingItem,
  SidebarFileItem,
  SidebarPageChildItem,
  SidebarPageItem,
  SidebarShapeItem,
  SidebarTextItem,
  WorkspaceBounds,
  WorkspaceGroup,
} from '../../shared/types'
import { matchesPageUrl } from '../../shared/page-anchor'
import { findAnchorableEntity } from './page-anchor-state'
import { isUnresolved, truncate } from '../../shared/annotation-utils'
import {
  findPageById,
  interactionState,
  pages,
} from './runtime-context'
import {
  activeWorkspaceTabId,
  workspaceAnnotations,
  workspaceGroups,
} from './workspace-model'
import { leftSidebarView } from './view-refs'
import {
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedEntityIds as uiSelectedEntityIds,
  selectedGroupId as uiSelectedGroupId,
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

/** Sort by stack rank (top of stack first) and strip the transient key. */
function sortSidebarItems<T>(items: (T & { sortKey: number })[]): T[] {
  return items
    .sort((a, b) => b.sortKey - a.sortKey)
    .map(({ sortKey: _sortKey, ...item }) => item as T)
}

/**
 * The sidebar's leaf entities in stack order: pages then the non-group entity
 * kinds. Drawings use the UI-filtered view (`drawingEntitiesForUi()`), not the
 * raw persisted store — so this stays a local enumeration rather than the
 * registry's `allEntities()`, which exposes the raw store and includes groups.
 */
function sidebarLeafEntities(): { id: string; parentGroupId?: string }[] {
  return [
    ...pages,
    ...textEntities,
    ...fileEntities,
    ...drawingEntitiesForUi(),
    ...shapeEntities,
  ]
}

function buildSidebarLeafItem(
  entityId: string,
  ranks: Map<string, number>,
): (SidebarLeafItem & { sortKey: SortableSidebarItem['sortKey'] }) | null {
  const leaf = describeSidebarLeaf(entityId)
  if (!leaf) return null
  return { ...leaf, sortKey: ranks.get(entityId) ?? Number.MAX_SAFE_INTEGER }
}

/**
 * Whether a leaf entity is hooked to an existing page (shared/page-anchor.ts)
 * — it nests under that page in the sidebar instead of the root list.
 * Grouped entities stay under their group.
 */
function anchoredPageIdFor(entityId: string): string | null {
  const entity = findAnchorableEntity(entityId)
  if (!entity || entity.parentGroupId) return null
  const pageId = entity.pageAnchor?.pageId
  if (!pageId) return null
  return pages.some((page) => page.id === pageId) ? pageId : null
}

/**
 * Content belonging to a page, projected as sidebar child rows (the page
 * acts as a folder for content anchored to it): anchored canvas entities in
 * stack order, then unresolved page-anchored annotations newest first. The
 * binding read is `pageAnchor` for both — annotations without one are
 * canvas-bound and stay out of the tree. `onCurrentPage` dims rows whose
 * page has navigated away from the URL they were placed on.
 */
function sidebarPageChildren(
  pageId: string,
  currentPageUrl: string | undefined,
  ranks: Map<string, number>,
): SidebarPageChildItem[] {
  const anchored: (SidebarAnchoredEntityItem & { sortKey: number })[] = []
  for (const entity of sidebarLeafEntities()) {
    if (anchoredPageIdFor(entity.id) !== pageId) continue
    const leaf = describeSidebarLeaf(entity.id)
    if (!leaf || leaf.kind === 'page') continue
    const anchor = findAnchorableEntity(entity.id)?.pageAnchor
    anchored.push({
      ...leaf,
      onCurrentPage: matchesPageUrl(anchor?.pageUrl, currentPageUrl),
      sortKey: ranks.get(entity.id) ?? Number.MAX_SAFE_INTEGER,
    })
  }

  const annotations = workspaceAnnotations
    .filter(
      (annotation) =>
        isUnresolved(annotation.status) && annotation.pageAnchor?.pageId === pageId,
    )
    .sort((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt))
    .map(
      (annotation): SidebarPageChildItem => ({
        kind: 'annotation',
        id: annotation.id,
        label: sidebarAnnotationLabel(annotation),
        messageCount: 1 + annotation.replies.length,
        onCurrentPage: matchesPageUrl(annotation.pageAnchor?.pageUrl, currentPageUrl),
      }),
    )

  return [...sortSidebarItems(anchored), ...annotations]
}

function sidebarAnnotationLabel(annotation: Annotation): string {
  const name = annotation.elementName?.trim()
  if (name) return name
  const text = annotation.text.trim()
  if (text) return truncate(text, 60)
  return 'Comment'
}

// The kind-specific projection, minus sort position (the caller stamps that).
function describeSidebarLeaf(entityId: string): SidebarLeafItem | null {
  const page = findPageById(entityId)
  if (page) {
    const children = sidebarPageChildren(entityId, page.url, entityOrderRank())
    return {
      kind: 'page',
      id: entityId,
      label: pageDisplayLabel(page),
      faviconUrl: page.faviconUrl ?? null,
      width: page.peekWidth,
      height: page.peekHeight,
      ...(children.length ? { children } : {}),
    }
  }

  const te = textEntities.find((entity) => entity.id === entityId)
  if (te) {
    return { kind: 'text', id: entityId, label: te.label || te.text || 'Text', color: te.color }
  }

  const fe = fileEntities.find((entity) => entity.id === entityId)
  if (fe) {
    const fileName = fe.file.split('/').pop() ?? fe.file
    const displayName = fileName.replace(/\.md$/i, '')
    return { kind: 'file', id: entityId, label: displayName, file: fe.file }
  }

  const de = drawingEntitiesForUi().find((entity) => entity.id === entityId)
  if (de) {
    const defaultLabel = `Drawing (${de.strokes.length} stroke${de.strokes.length === 1 ? '' : 's'})`
    return {
      kind: 'drawing',
      id: entityId,
      label: de.label || defaultLabel,
      strokeCount: de.strokes.length,
    }
  }

  const se = shapeEntities.find((entity) => entity.id === entityId)
  if (se) {
    const defaultLabel = shapeDef(se.shapeKind).label
    return {
      kind: 'shape',
      id: entityId,
      label: se.label || se.text.trim() || defaultLabel,
      shapeKind: se.shapeKind,
    }
  }

  return null
}

function countSidebarLeafDescendants(groupId: string): number {
  const directLeafCount = sidebarLeafEntities()
    .filter((entity) => entity.parentGroupId === groupId).length

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
    const directLeafItems = sidebarLeafEntities()
      .filter((entity) => entity.parentGroupId === node.group.id)
      .map((entity) => buildSidebarLeafItem(entity.id, ranks))
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

  const groupedEntityIds = new Set<string>(
    sidebarLeafEntities()
      .filter((entity) => entity.parentGroupId)
      .map((entity) => entity.id),
  )
  const rootLeafItems = sidebarLeafEntities()
    .filter((entity) => !groupedEntityIds.has(entity.id))
    // Page-anchored entities nest under their page row, not the root list.
    .filter((entity) => anchoredPageIdFor(entity.id) === null)
    .map((entity) => buildSidebarLeafItem(entity.id, ranks))
    .filter((item): item is NonNullable<typeof item> => Boolean(item))

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
  wc.send(ipcChannels.leftSidebarData, buildLeftSidebarData())
}
