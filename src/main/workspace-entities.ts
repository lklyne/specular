// fallow-ignore-file circular-dependencies
// Suppressed: see #141. workspace-placement imports workspace-entities creating a mutual dependency
import type {
  CanvasEntityKind,
  DeletePagesRequest,
  DeletePagesResponse,
  EdgeSide,
  WorkspaceBounds,
  WorkspacePage,
  WorkspaceGraph,
  WorkspaceSelection,
} from '../shared/types'
import type { SelectionMutationMode } from '../shared/selection-modifiers'
import { applyEntitySelectionMutation } from './runtime/selection-controller'
import {
  findPageById,
  pages,
  removePageById,
} from './runtime/page-runtime'
import {
  deselectAll,
  focusCanvasBounds,
  getSelectedEntityIds,
  getSelectedGroupId,
  selectedPageId,
  selectPageById,
  setSelectedEntities,
  setSelectedPages,
  setSelectedGroupId,
} from './runtime/ui-actions'
import { textEntities } from './runtime/text-entity-state'
import { fileEntities } from './runtime/file-entity-state'
import { drawingEntities } from './runtime/drawing-entity-state'
import { shapeEntities } from './runtime/shape-entity-state'
import { pan, zoom } from './runtime/runtime-context'
import { requestLayout } from './runtime/viewport-control'
import { CHROME_HEADER_HEIGHT } from '../shared/entity-chrome-slots'
import { workspaceEdges, workspaceGroups } from './runtime/workspace-model'
import { scheduleWorkspaceAutosave } from './runtime/workspace-autosave'
import {
  boundsOverlap,
  pageContentSize,
  pageSnapBounds,
  pageVisualBounds,
} from './runtime/runtime-geometry'
import { cloneMetadata } from './workspace-utils'
import { removeEdgesTouchingEntities } from './workspace-edges'
import { occupiedRegions } from './workspace-placement'
import { cancelEditingEntityIfMatches } from './runtime/editing-entity-runtime'

// --- Bounds helpers ---

export function pageBoundsById(pageId: string): WorkspaceBounds | null {
  const page = findPageById(pageId)
  return page ? pageSnapBounds(page) : null
}

/**
 * Bounds for selection/hover/group-outline purposes: the snap rect extended
 * upward by the chrome strip. Wraps everything the user can see.
 */
export function pageSelectableBounds(
  page: Exclude<ReturnType<typeof findPageById>, undefined>,
): WorkspaceBounds {
  return pageVisualBounds(page)
}

export function unionBounds(boundsList: WorkspaceBounds[]): WorkspaceBounds | null {
  if (!boundsList.length) return null
  let left = boundsList[0].x
  let top = boundsList[0].y
  let right = left + boundsList[0].width
  let bottom = top + boundsList[0].height
  for (let i = 1; i < boundsList.length; i++) {
    const b = boundsList[i]
    if (b.x < left) left = b.x
    if (b.y < top) top = b.y
    if (b.x + b.width > right) right = b.x + b.width
    if (b.y + b.height > bottom) bottom = b.y + b.height
  }
  return { x: left, y: top, width: right - left, height: bottom - top }
}

export function entityBoundsById(entityId: string): WorkspaceBounds | null {
  return entityBoundsByIdWithVisited(entityId, new Set<string>())
}

/**
 * Offset from the entity's outer top-left (as returned by `entityBoundsById`)
 * to its data origin (`canvasX`/`canvasY`). For pages, `canvasY` is the
 * snap-rect top and `pageSelectableBounds.y` is `canvasY - CHROME_HEADER_HEIGHT`,
 * so the Y inset is the chrome strip; the snap rect's left edge already
 * equals `canvasX` so the X inset is zero.
 */
export function entityDataInsetsById(entityId: string): { insetX: number; insetY: number } {
  const page = findPageById(entityId)
  if (page) {
    return {
      insetX: 0,
      insetY: CHROME_HEADER_HEIGHT,
    }
  }
  return { insetX: 0, insetY: 0 }
}

type AnyEntity =
  | { canvasX: number; canvasY: number; width: number; height: number; id: string }

/**
 * Single dispatch point for "find an entity by id and tell me what kind it is."
 * `entityKindById` and `entityBoundsByIdWithVisited` both delegate here so the
 * id-keyed lookup chain runs only once per call. Pages are returned as-is so
 * the bounds path can use `pageSelectableBounds` (selectable bounds differ
 * from raw `{canvasX, canvasY, width, height}` for pages).
 */
function findEntityById(
  entityId: string,
): { kind: 'page'; page: ReturnType<typeof findPageById> }
  | { kind: 'text' | 'file' | 'drawing' | 'shape' | 'group'; entity: AnyEntity }
  | null {
  const page = findPageById(entityId)
  if (page) return { kind: 'page', page }
  const te = textEntities.find((t) => t.id === entityId)
  if (te) return { kind: 'text', entity: te }
  const fe = fileEntities.find((f) => f.id === entityId)
  if (fe) return { kind: 'file', entity: fe }
  const de = drawingEntities.find((d) => d.id === entityId)
  if (de) return { kind: 'drawing', entity: de }
  const se = shapeEntities.find((s) => s.id === entityId)
  if (se) return { kind: 'shape', entity: se }
  const group = workspaceGroups.find((g) => g.id === entityId)
  if (group) return { kind: 'group', entity: group }
  return null
}

export function entityKindById(entityId: string): CanvasEntityKind | null {
  return findEntityById(entityId)?.kind ?? null
}

function entityBoundsByIdWithVisited(
  entityId: string,
  visited: Set<string>,
): WorkspaceBounds | null {
  if (visited.has(entityId)) return null
  const found = findEntityById(entityId)
  if (!found) return null
  if (found.kind === 'page' && found.page) {
    return pageSelectableBounds(found.page)
  }
  if (found.kind !== 'page') {
    const e = found.entity
    return { x: e.canvasX, y: e.canvasY, width: e.width, height: e.height }
  }
  return null
}

export function groupBoundsForEntityIds(entityIds: string[]): WorkspaceBounds | null {
  return groupBoundsForEntityIdsWithVisited(entityIds, new Set<string>())
}

function groupBoundsForEntityIdsWithVisited(
  entityIds: string[],
  visited: Set<string>,
): WorkspaceBounds | null {
  const bounds = entityIds
    .map((entityId) => entityBoundsByIdWithVisited(entityId, visited))
    .filter((b): b is WorkspaceBounds => b !== null)
  return unionBounds(bounds)
}

// --- Group child helpers (needed by many modules) ---

export function groupById(groupId: string): import('../shared/types').WorkspaceGroup | undefined {
  return workspaceGroups.find((group) => group.id === groupId)
}

export function groupChildIds(groupId: string): string[] {
  return [
    ...pages.filter((page) => page.parentGroupId === groupId).map((page) => page.id),
    ...textEntities.filter((entity) => entity.parentGroupId === groupId).map((entity) => entity.id),
    ...fileEntities.filter((entity) => entity.parentGroupId === groupId).map((entity) => entity.id),
    ...drawingEntities.filter((entity) => entity.parentGroupId === groupId).map((entity) => entity.id),
    ...shapeEntities.filter((entity) => entity.parentGroupId === groupId).map((entity) => entity.id),
    ...workspaceGroups.filter((group) => group.parentGroupId === groupId).map((group) => group.id),
  ]
}

export function groupDescendantIds(groupId: string): string[] {
  const ids: string[] = []
  const visit = (parentId: string) => {
    for (const childId of groupChildIds(parentId)) {
      ids.push(childId)
      if (groupById(childId)) visit(childId)
    }
  }
  visit(groupId)
  return ids
}

export function groupBounds(group: import('../shared/types').WorkspaceGroup): WorkspaceBounds | null {
  const bounds = groupChildIds(group.id)
    .map(entityBoundsById)
    .filter((item): item is WorkspaceBounds => item !== null)
  return unionBounds(bounds)
}

// --- Selection helpers ---

export function selectionBounds(): WorkspaceBounds | null {
  const selectedIds = getSelectedEntityIds()
  if (selectedIds.length) {
    return unionBounds(
      selectedIds
        .map(pageBoundsById)
        .filter((item): item is WorkspaceBounds => item !== null),
    )
  }

  const activeGroupId = getSelectedGroupId()
  if (!activeGroupId) return null
  const group = groupById(activeGroupId)
  return group ? groupBounds(group) : null
}

export function currentSelection(): WorkspaceSelection {
  const selectedGroupId = getSelectedGroupId()
  if (selectedGroupId) {
    return {
      selectedGroupId,
    }
  }

  const selectedEntityIds = getSelectedEntityIds()
  const selectedPage = selectedPageId() ?? selectedEntityIds[0]
  return {
    selectedEntityId: selectedPage,
    selectedEntityIds: selectedEntityIds.length ? selectedEntityIds : undefined,
  }
}

// --- Empty group cleanup ---

export function removeEmptyGroups(): string[] {
  const deletedGroupIds: string[] = []
  for (let idx = workspaceGroups.length - 1; idx >= 0; idx--) {
    const g = workspaceGroups[idx]
    const isEmpty = groupChildIds(g.id).length === 0
    if (isEmpty) {
      deletedGroupIds.push(workspaceGroups[idx].id)
      workspaceGroups.splice(idx, 1)
    }
  }
  if (deletedGroupIds.includes(getSelectedGroupId() ?? '')) {
    setSelectedGroupId(null)
  }
  return deletedGroupIds
}

// --- Delete pages ---

export function deletePages(input: DeletePagesRequest): DeletePagesResponse {
  const deletedPageIds: string[] = []
  const missingPageIds: string[] = []

  for (const pageId of input.pageIds) {
    cancelEditingEntityIfMatches(pageId)
    const removed = removePageById(pageId)
    if (!removed) {
      missingPageIds.push(pageId)
      continue
    }
    deletedPageIds.push(pageId)
  }

  const deletedEdgeIds = removeEdgesTouchingEntities(new Set(deletedPageIds))
  const deletedGroupIds = removeEmptyGroups()

  if (!selectedPageId() && !getSelectedGroupId()) {
    deselectAll()
  }

  if (input.focusAfter) {
    const bounds = selectionBounds()
    if (bounds) focusCanvasBounds(bounds)
  } else {
    requestLayout()
  }

  if (deletedPageIds.length || deletedEdgeIds.length || deletedGroupIds.length) {
    scheduleWorkspaceAutosave()
  }

  return {
    deletedPageIds,
    deletedEdgeIds,
    deletedGroupIds,
    missingPageIds,
    warnings: missingPageIds.length
      ? [`Missing page IDs: ${missingPageIds.join(', ')}`]
      : [],
  }
}

// --- Selection in rect ---

export function selectEntitiesInRect(
  bounds: WorkspaceBounds,
  options: {
    includeDrawings?: boolean
    mode?: SelectionMutationMode
  } = {},
): { entityIds: string[] } {
  const includeDrawings = options.includeDrawings ?? true
  const mode = options.mode ?? 'replace'
  const pageIds = pages
    .filter((page) => boundsOverlap(pageSelectableBounds(page), bounds))
    .map((page) => page.id)
  const textIds = textEntities
    .filter((note) =>
      boundsOverlap(
        {
          x: note.canvasX,
          y: note.canvasY,
          width: note.width,
          height: note.height,
        },
        bounds,
      ),
    )
    .map((note) => note.id)
  const fileIds = fileEntities
    .filter((fe) =>
      boundsOverlap(
        {
          x: fe.canvasX,
          y: fe.canvasY,
          width: fe.width,
          height: fe.height,
        },
        bounds,
      ),
    )
    .map((fe) => fe.id)
  const drawingIds = !includeDrawings ? [] : drawingEntities
    .filter((de) =>
      boundsOverlap(
        {
          x: de.canvasX,
          y: de.canvasY,
          width: de.width,
          height: de.height,
        },
        bounds,
      ),
    )
    .map((de) => de.id)
  const shapeIds = shapeEntities
    .filter((se) =>
      boundsOverlap(
        {
          x: se.canvasX,
          y: se.canvasY,
          width: se.width,
          height: se.height,
        },
        bounds,
      ),
    )
    .map((se) => se.id)
  const edgeIds = workspaceEdges
    .filter((edge) => {
      const fromBounds = entityBoundsById(edge.fromEntityId)
      const toBounds = entityBoundsById(edge.toEntityId)
      if (!fromBounds || !toBounds) return false
      const from = sideAnchorPoint(fromBounds, edge.fromSide)
      const to = sideAnchorPoint(toBounds, edge.toSide)
      return segmentIntersectsRect(from, to, bounds)
    })
    .map((edge) => edge.id)

  const entityIds = [...pageIds, ...textIds, ...fileIds, ...drawingIds, ...shapeIds, ...edgeIds]

  if (mode !== 'replace') {
    // Additive / toggle / remove modes: preserve existing selection outside the rect
    // and merge rect hits according to the mode. An empty rect with an additive
    // mode is a no-op, which matches OS-level shift-click conventions.
    applyEntitySelectionMutation(entityIds, mode)
    return { entityIds }
  }

  if (!entityIds.length) {
    deselectAll()
    return { entityIds: [] }
  }

  if (
    !textIds.length &&
    !fileIds.length &&
    !drawingIds.length &&
    !shapeIds.length &&
    !edgeIds.length
  ) {
    if (pageIds.length === 1) {
      selectPageById(pageIds[0])
    } else {
      setSelectedPages(pageIds)
    }
    return { entityIds: pageIds }
  }

  setSelectedEntities(entityIds)
  return { entityIds }
}

function sideAnchorPoint(
  bounds: WorkspaceBounds,
  side: EdgeSide | undefined,
): { x: number; y: number } {
  switch (side) {
    case 'top':
      return { x: bounds.x + bounds.width / 2, y: bounds.y }
    case 'bottom':
      return { x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height }
    case 'left':
      return { x: bounds.x, y: bounds.y + bounds.height / 2 }
    case 'right':
      return { x: bounds.x + bounds.width, y: bounds.y + bounds.height / 2 }
    default:
      return { x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 }
  }
}

// Liang-Barsky: returns true if the segment from p1 to p2 intersects rect
// (including the case where the segment lies entirely inside the rect).
function segmentIntersectsRect(
  p1: { x: number; y: number },
  p2: { x: number; y: number },
  rect: WorkspaceBounds,
): boolean {
  const dx = p2.x - p1.x
  const dy = p2.y - p1.y
  const xMin = rect.x
  const xMax = rect.x + rect.width
  const yMin = rect.y
  const yMax = rect.y + rect.height
  const p = [-dx, dx, -dy, dy]
  const q = [p1.x - xMin, xMax - p1.x, p1.y - yMin, yMax - p1.y]
  let t0 = 0
  let t1 = 1
  for (let i = 0; i < 4; i++) {
    if (p[i] === 0) {
      if (q[i] < 0) return false
    } else {
      const t = q[i] / p[i]
      if (p[i] < 0) {
        if (t > t1) return false
        if (t > t0) t0 = t
      } else {
        if (t < t0) return false
        if (t < t1) t1 = t
      }
    }
  }
  return true
}

// --- Workspace graph ---

export function toWorkspacePage(pageId: string): WorkspacePage | null {
  const page = findPageById(pageId)
  if (!page) return null
  const size = pageContentSize(page)
  return {
    id: page.id,
    kind: 'page',
    name: page.name?.trim() || undefined,
    url: page.pageView.webContents.getURL() || 'about:blank',
    presetIndex: page.presetIndex,
    canvasX: page.canvasX,
    canvasY: page.canvasY,
    width: size.width,
    height: size.height,
    linkedBrowsing: page.linked,
    source: page.source,
    parentGroupId: page.parentGroupId,
    groupId: page.parentGroupId,
    metadata: cloneMetadata(page.metadata),
  }
}

export function allWorkspacePages(): WorkspacePage[] {
  return pages
    .map((page) => toWorkspacePage(page.id))
    .filter((page): page is WorkspacePage => page !== null)
}

export function getWorkspaceGraph(): WorkspaceGraph {
  return {
    entities: [
      ...allWorkspacePages(),
      ...textEntities.map((entity) => ({
        id: entity.id,
        kind: 'text' as const,
        preview: entity.text.slice(0, 80) + (entity.text.length > 80 ? '…' : ''),
        color: entity.color,
        canvasX: entity.canvasX,
        canvasY: entity.canvasY,
        width: entity.width,
        height: entity.height,
        parentGroupId: entity.parentGroupId,
      })),
      ...fileEntities.map((entity) => ({
        id: entity.id,
        kind: 'file' as const,
        file: entity.file,
        subpath: entity.subpath,
        canvasX: entity.canvasX,
        canvasY: entity.canvasY,
        width: entity.width,
        height: entity.height,
        parentGroupId: entity.parentGroupId,
      })),
      ...workspaceGroups.map((group) => ({
        ...group,
        kind: 'group' as const,
        metadata: cloneMetadata(group.metadata),
      })),
      ...drawingEntities.map((entity) => ({
        id: entity.id,
        kind: 'drawing' as const,
        canvasX: entity.canvasX,
        canvasY: entity.canvasY,
        width: entity.width,
        height: entity.height,
        parentGroupId: entity.parentGroupId,
        label: entity.label,
      })),
      ...shapeEntities.map((entity) => ({
        id: entity.id,
        kind: 'shape' as const,
        shapeKind: entity.shapeKind,
        text: entity.text,
        color: entity.color,
        strokeWidth: entity.strokeWidth,
        textSize: entity.textSize,
        theme: entity.theme,
        canvasX: entity.canvasX,
        canvasY: entity.canvasY,
        width: entity.width,
        height: entity.height,
        parentGroupId: entity.parentGroupId,
        label: entity.label,
      })),
    ],
    edges: workspaceEdges.map((edge) => ({
      ...edge,
      metadata: cloneMetadata(edge.metadata),
    })),
    selection: currentSelection(),
    camera: {
      zoom,
      panX: pan.x,
      panY: pan.y,
    },
    occupiedRegions: occupiedRegions(),
  }
}

export function getSelectionState(): WorkspaceSelection {
  return currentSelection()
}
