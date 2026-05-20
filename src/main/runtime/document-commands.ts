/**
 * Document Commands
 *
 * This module provides the canonical entry points for all mutations that change
 * persisted workspace data. Every function here modifies the workspace document
 * (pages, groups, edges, annotations) and triggers autosave.
 *
 * This is the seam where undo/redo will hook in: when undo is added, these
 * functions will be wrapped to capture before/after state. UI-only mutations
 * (selection, view mode, tool state) live in ui-actions.ts and do NOT
 * participate in the undo stack.
 *
 * Rules:
 * - Every document command triggers scheduleWorkspaceAutosave()
 * - Every document command triggers requestLayout()
 * - Document commands may also change UI state (e.g., selecting a newly created page)
 * - UI commands (in ui-actions.ts) never change persisted document data
 */

import { GRID_SIZE, VIEWPORT_PRESETS } from '../../shared/constants'
import type { DeviceOrientation } from '../../shared/device-catalog'
import { deviceForPresetIndex } from '../../shared/device-catalog'
import type { AlignmentReferenceName } from '../../shared/canvas-guides'
import type { ResizeHandle } from '../../shared/resize-accumulator'
import type { AnnotationDrawingStroke, EdgeEnd, EdgeSide } from '../../shared/types'
import type { WorkspaceGroup } from '../../shared/types'
import {
  updateSelectionForRemovedEntity,
  selectedEntityIds as uiSelectedEntityIds,
  selectedGroupId as uiSelectedGroupId,
} from '../ui-state'
import { removeEdgesTouchingEntities } from '../workspace-edges'
import {
  createUserGroup as createUserGroupInEngine,
  ungroupUserGroup as ungroupUserGroupInEngine,
} from '../workspace-groups'
import {
  createDrawingEntity as createDrawingEntityInState,
  deleteDrawingEntity as deleteDrawingEntityInState,
  drawingEntities,
  type DrawingEntity,
  updateDrawingEntity as updateDrawingEntityInState,
} from './drawing-entity-state'
import {
  createFileEntity as createFileEntityInState,
  updateFileEntity as updateFileEntityInState,
  deleteFileEntity as deleteFileEntityInState,
  fileEntities,
  type FileEntity,
} from './file-entity-state'
import {
  updateGroupEntity as updateGroupEntityInState,
} from './group-entity-state'
import { markDirty } from './layout-dirty'
import { pages } from './page-runtime'
import {
  clearCustomPageSizeMetadata,
  deviceIdFromMetadata,
  deviceOrientationFromMetadata,
  pageUsesCustomSize,
  setCustomPageSizeMetadata,
  setDeviceIdMetadata,
  setDeviceOrientationMetadata,
  setShowDeviceFrameMetadata,
  setUseSvgDeviceShellMetadata,
  showDeviceFrameFromMetadata,
  useSvgDeviceShellFromMetadata,
} from './runtime-entities'
import { selectEntities, selectGroup } from './selection-controller'
import { cancelEditingEntityIfMatches } from './editing-entity-runtime'
import {
  canvasOrigin,
  pageContentSize,
  pan,
  requestLayout,
  snapToGrid,
  zoom,
} from './surface-layout'
import {
  createTextEntity as createTextEntityInState,
  updateTextEntity as updateTextEntityInState,
  deleteTextEntity as deleteTextEntityInState,
  textEntities,
  type TextEntity,
} from './text-entity-state'
import {
  createShapeEntity as createShapeEntityInState,
  updateShapeEntity as updateShapeEntityInState,
  deleteShapeEntity as deleteShapeEntityInState,
  shapeEntities,
  type ShapeEntity,
} from './shape-entity-state'
import { axisLockDominantAxis, axisLockProjector } from '../../shared/axis-lock-projector'
import { alignmentGuideDetector } from './alignment-guide-detector'
import { broadcastCanvasGuides, clearCanvasGuides } from './canvas-guides'
import { distributionGuideDetector } from './distribution-guide-detector'
import { descendantEntityIdsForGroup } from './group-descendants'
import { resizeGuideReferencesForHandle } from './resize-guide-adapter'
import { workspaceEdges, workspaceGroups } from './workspace-model'
import { beginBatch, endBatch } from './workspace-observers'
import { scheduleWorkspaceAutosave } from './workspace-session'
import { markUndoBoundary } from './workspace-undo'
import {
  boundAvailableCanvasViewportRect,
  pageSnapBounds,
} from './runtime-geometry'
import {
  snapCandidateFromRect,
  snapCandidateSnapshot,
  type SnapCandidate,
  type SnapCandidateSnapshotEntity,
  type SnapRect,
} from './snap-candidate-snapshot'

// --- Page Commands ---

export {
  addPageFromSource,
  createPageAtPosition,
  createPages,
  duplicatePageFromSource,
  tidySelectedPages,
} from '../workspace-pages'
export { deletePages } from '../workspace-entities'
export { pastePagesFromClipboard } from '../workspace-clipboard'

// --- Entity Movement ---

/**
 * Find any movable canvas entity by ID (page or text entity).
 * Returns an object with mutable canvasX/canvasY.
 */
export function findMovableEntity(id: string): { canvasX: number; canvasY: number } | null {
  const page = pages.find((p) => p.id === id)
  if (page) return page
  const te = textEntities.find((n) => n.id === id)
  if (te) return te
  const fe = fileEntities.find((e) => e.id === id)
  if (fe) return fe
  const de = drawingEntities.find((e) => e.id === id)
  if (de) return de
  const se = shapeEntities.find((e) => e.id === id)
  if (se) return se
  const group = workspaceGroups.find((candidate) => candidate.id === id)
  if (group) return group
  return null
}

/**
 * Accumulates sub-pixel drag deltas and applies grid-snapped positions.
 * Works with any entity type (pages, text entities).
 * When undo/redo is added, the drag-start snapshot and drag-end snapshot
 * form a single undoable operation.
 */
type DragAccumulator = {
  originX: number
  originY: number
  rawX: number
  rawY: number
  appliedX: number
  appliedY: number
}

type DragDeltaOptions = {
  shiftKey?: boolean
}

const dragAccumulatorById = new Map<string, DragAccumulator>()
let activeDragCandidates: SnapCandidate[] = []
let activeDraggedGuideIds: string[] = []
let activeResizeGuideSession: {
  entityId: string
  references: AlignmentReferenceName[]
  candidates: SnapCandidate[]
} | null = null

function currentCanvasViewportRect(): SnapRect {
  const viewport = boundAvailableCanvasViewportRect()
  const origin = canvasOrigin()
  return {
    x: (viewport.x - origin.x - pan.x) / zoom,
    y: (viewport.y - origin.y - pan.y) / zoom,
    width: viewport.width / zoom,
    height: viewport.height / zoom,
  }
}

function currentSnapSnapshotEntities(): SnapCandidateSnapshotEntity[] {
  return [
    ...pages.map((page) => {
      const bounds = pageSnapBounds(page)
      return {
        id: page.id,
        kind: 'page' as const,
        canvasX: bounds.x,
        canvasY: bounds.y,
        width: bounds.width,
        height: bounds.height,
        parentGroupId: page.parentGroupId,
      }
    }),
    ...textEntities.map((entity) => ({
      id: entity.id,
      kind: 'text' as const,
      canvasX: entity.canvasX,
      canvasY: entity.canvasY,
      width: entity.width,
      height: entity.height,
      parentGroupId: entity.parentGroupId,
    })),
    ...fileEntities.map((entity) => ({
      id: entity.id,
      kind: 'file' as const,
      canvasX: entity.canvasX,
      canvasY: entity.canvasY,
      width: entity.width,
      height: entity.height,
      parentGroupId: entity.parentGroupId,
    })),
    ...drawingEntities.map((entity) => ({
      id: entity.id,
      kind: 'drawing' as const,
      canvasX: entity.canvasX,
      canvasY: entity.canvasY,
      width: entity.width,
      height: entity.height,
      parentGroupId: entity.parentGroupId,
    })),
    ...shapeEntities.map((entity) => ({
      id: entity.id,
      kind: 'shape' as const,
      canvasX: entity.canvasX,
      canvasY: entity.canvasY,
      width: entity.width,
      height: entity.height,
      parentGroupId: entity.parentGroupId,
    })),
    ...workspaceGroups.map((group) => ({
      id: group.id,
      kind: 'group' as const,
      canvasX: group.canvasX,
      canvasY: group.canvasY,
      width: group.width,
      height: group.height,
      parentGroupId: group.parentGroupId,
    })),
  ]
}

function currentSnapCandidateForEntity(id: string): SnapCandidate | null {
  const page = pages.find((candidate) => candidate.id === id)
  if (page) {
    return snapCandidateFromRect(
      { id: page.id, kind: 'page' },
      pageSnapBounds(page),
    )
  }

  const entity = currentSnapSnapshotEntities().find((candidate) => candidate.id === id)
  if (!entity) return null
  return snapCandidateFromRect(entity, {
    x: entity.canvasX,
    y: entity.canvasY,
    width: entity.width,
    height: entity.height,
  })
}

function guideEntityIdsForDrag(entityIds: string[]): string[] {
  const dragged = new Set(entityIds)
  return currentSnapSnapshotEntities()
    .filter((entity) => dragged.has(entity.id))
    .filter((entity) => !entity.parentGroupId || !dragged.has(entity.parentGroupId))
    .map((entity) => entity.id)
}

export function initializeDrag(entityIds: string[]): void {
  dragAccumulatorById.clear()
  activeDraggedGuideIds = guideEntityIdsForDrag(entityIds)
  activeDragCandidates = snapCandidateSnapshot(
    { entities: currentSnapSnapshotEntities() },
    currentCanvasViewportRect(),
    entityIds,
  )
  beginBatch()
  for (const id of entityIds) {
    const entity = findMovableEntity(id)
    if (!entity) continue
    dragAccumulatorById.set(id, {
      originX: entity.canvasX,
      originY: entity.canvasY,
      rawX: entity.canvasX,
      rawY: entity.canvasY,
      appliedX: entity.canvasX,
      appliedY: entity.canvasY,
    })
  }
}

function dragPositionFromAccumulator(
  acc: DragAccumulator,
  options: DragDeltaOptions,
  snap: boolean,
): { x: number; y: number } {
  const rawDelta = {
    x: acc.rawX - acc.originX,
    y: acc.rawY - acc.originY,
  }
  const projectedDelta = axisLockProjector(rawDelta, rawDelta, Boolean(options.shiftKey))
  const dominantAxis = axisLockDominantAxis(rawDelta, Boolean(options.shiftKey))
  const projectedX = acc.originX + projectedDelta.x
  const projectedY = acc.originY + projectedDelta.y

  return {
    x: !snap || dominantAxis === 'vertical' ? projectedX : snapToGrid(projectedX),
    y: !snap || dominantAxis === 'horizontal' ? projectedY : snapToGrid(projectedY),
  }
}

export function applyDragDelta(
  entityIds: string[],
  dx: number,
  dy: number,
  options: DragDeltaOptions = {},
): void {
  for (const id of entityIds) {
    const entity = findMovableEntity(id)
    if (!entity) continue
    let acc = dragAccumulatorById.get(id)
    if (!acc) {
      acc = {
        originX: entity.canvasX,
        originY: entity.canvasY,
        rawX: entity.canvasX,
        rawY: entity.canvasY,
        appliedX: entity.canvasX,
        appliedY: entity.canvasY,
      }
      dragAccumulatorById.set(id, acc)
    } else {
      const driftX = Math.abs(acc.appliedX - entity.canvasX)
      const driftY = Math.abs(acc.appliedY - entity.canvasY)
      if (driftX > GRID_SIZE / 2 || driftY > GRID_SIZE / 2) {
        acc.originX = entity.canvasX
        acc.originY = entity.canvasY
        acc.rawX = entity.canvasX
        acc.rawY = entity.canvasY
        acc.appliedX = entity.canvasX
        acc.appliedY = entity.canvasY
      }
    }
    acc.rawX += dx / zoom
    acc.rawY += dy / zoom
    const prevX = entity.canvasX
    const prevY = entity.canvasY
    const isDrawing = drawingEntities.some((d) => d.id === id)
    const next = dragPositionFromAccumulator(acc, options, !isDrawing)
    entity.canvasX = next.x
    entity.canvasY = next.y
    acc.appliedX = next.x
    acc.appliedY = next.y
    shiftDrawingStrokes(id, entity.canvasX - prevX, entity.canvasY - prevY)
  }
  if (entityIds.length) {
    const draggedRects = activeDraggedGuideIds
      .map(currentSnapCandidateForEntity)
      .filter((candidate): candidate is SnapCandidate => candidate !== null)
    broadcastCanvasGuides({
      alignmentGuides: alignmentGuideDetector(draggedRects, activeDragCandidates),
      distributionGuides: draggedRects.flatMap((dragged) => [
        ...distributionGuideDetector(dragged, activeDragCandidates, 'horizontal'),
        ...distributionGuideDetector(dragged, activeDragCandidates, 'vertical'),
      ]),
    })
    markDirty('canvas', 'sidebar')
    scheduleWorkspaceAutosave()
  }
}

/**
 * Drawing strokes are stored in absolute canvas coordinates, not relative to
 * the entity origin (the renderer applies pan/zoom directly to each point).
 * When the entity's `canvasX/canvasY` moves, the strokes have to move with
 * it or the bbox will drift away from the visible ink.
 */
function shiftDrawingStrokes(entityId: string, deltaX: number, deltaY: number): void {
  if (deltaX === 0 && deltaY === 0) return
  const drawing = drawingEntities.find((d) => d.id === entityId)
  if (!drawing) return
  drawing.strokes = drawing.strokes.map((stroke) => ({
    ...stroke,
    points: stroke.points.map((p) => ({ x: p.x + deltaX, y: p.y + deltaY })),
  }))
}

/**
 * Compute alignment + distribution guides for a phantom drag position without
 * mutating any entity. Used during option-drag copy, where the underlying
 * entities stay in place while the user previews the copy target.
 */
export function previewDragGuides(
  dx: number,
  dy: number,
  options: DragDeltaOptions = {},
): void {
  if (activeDraggedGuideIds.length === 0) return

  const snapshotEntities = currentSnapSnapshotEntities()
  const draggedRects: SnapCandidate[] = []
  const originRects: SnapCandidate[] = []
  for (const id of activeDraggedGuideIds) {
    const acc = dragAccumulatorById.get(id)
    if (!acc) continue
    const snapshot = snapshotEntities.find((entity) => entity.id === id)
    if (!snapshot) continue

    const phantomAcc: DragAccumulator = {
      originX: acc.originX,
      originY: acc.originY,
      rawX: acc.originX + dx / zoom,
      rawY: acc.originY + dy / zoom,
      appliedX: acc.originX,
      appliedY: acc.originY,
    }
    const next = dragPositionFromAccumulator(phantomAcc, options, snapshot.kind !== 'drawing')
    const offsetX = next.x - acc.originX
    const offsetY = next.y - acc.originY

    draggedRects.push(snapCandidateFromRect(
      { id, kind: snapshot.kind },
      {
        x: snapshot.canvasX + offsetX,
        y: snapshot.canvasY + offsetY,
        width: snapshot.width,
        height: snapshot.height,
      },
    ))
    originRects.push(snapCandidateFromRect(
      { id: `${id}:origin`, kind: snapshot.kind },
      {
        x: snapshot.canvasX,
        y: snapshot.canvasY,
        width: snapshot.width,
        height: snapshot.height,
      },
    ))
  }

  if (draggedRects.length === 0) {
    clearCanvasGuides()
    return
  }

  const candidates = [...activeDragCandidates, ...originRects]
  broadcastCanvasGuides({
    alignmentGuides: alignmentGuideDetector(draggedRects, candidates),
    distributionGuides: draggedRects.flatMap((dragged) => [
      ...distributionGuideDetector(dragged, candidates, 'horizontal'),
      ...distributionGuideDetector(dragged, candidates, 'vertical'),
    ]),
  })
}

export function finalizeDrag(): void {
  dragAccumulatorById.clear()
  activeDragCandidates = []
  activeDraggedGuideIds = []
  clearCanvasGuides()
  endBatch()
  markUndoBoundary()
}

function resizeGuideExcludedIds(entityId: string): string[] {
  const excluded = new Set<string>([entityId])
  for (const selectedId of uiSelectedEntityIds()) excluded.add(selectedId)

  const selectedGroup = uiSelectedGroupId()
  if (selectedGroup) excluded.add(selectedGroup)

  const groupIds = [entityId, ...excluded].filter((id) => (
    workspaceGroups.some((group) => group.id === id)
  ))
  for (const groupId of groupIds) {
    descendantEntityIdsForGroup(groupId).forEach((id) => excluded.add(id))
  }

  return [...excluded]
}

export function initializeResizeGuides(entityId: string, handle: ResizeHandle): void {
  activeResizeGuideSession = {
    entityId,
    references: resizeGuideReferencesForHandle(handle),
    candidates: snapCandidateSnapshot(
      { entities: currentSnapSnapshotEntities() },
      currentCanvasViewportRect(),
      resizeGuideExcludedIds(entityId),
    ),
  }
}

export function updateResizeGuides(entityId: string): void {
  if (!activeResizeGuideSession || activeResizeGuideSession.entityId !== entityId) return

  const dragged = currentSnapCandidateForEntity(entityId)
  if (!dragged) {
    clearCanvasGuides()
    return
  }

  broadcastCanvasGuides({
    alignmentGuides: alignmentGuideDetector(
      [{ ...dragged, references: activeResizeGuideSession.references }],
      activeResizeGuideSession.candidates,
    ),
    distributionGuides: [
      ...distributionGuideDetector(dragged, activeResizeGuideSession.candidates, 'horizontal'),
      ...distributionGuideDetector(dragged, activeResizeGuideSession.candidates, 'vertical'),
    ],
  })
}

export function finalizeResizeGuides(): void {
  activeResizeGuideSession = null
  clearCanvasGuides()
}

/**
 * Move entities by a screen-pixel delta. For use outside of drag flows
 * (e.g., keyboard arrow movement).
 */
export function moveEntities(entityIds: string[], dx: number, dy: number): void {
  for (const id of entityIds) {
    const entity = findMovableEntity(id)
    if (!entity) continue
    const prevX = entity.canvasX
    const prevY = entity.canvasY
    entity.canvasX = snapToGrid(entity.canvasX + dx / zoom)
    entity.canvasY = snapToGrid(entity.canvasY + dy / zoom)
    shiftDrawingStrokes(id, entity.canvasX - prevX, entity.canvasY - prevY)
  }
  if (entityIds.length) {
    markDirty('canvas', 'sidebar')
    scheduleWorkspaceAutosave()
    requestLayout()
  }
}

// --- Group Commands ---

export { deleteGroups } from '../workspace-groups'

export function groupSelectedEntities(): WorkspaceGroup | null {
  const ids = uiSelectedEntityIds()
  if (ids.length < 2) return null
  const group = createUserGroupInEngine(ids)
  selectGroup(group.id)
  requestLayout()
  return group
}

export function ungroupSelectedGroup(): string[] | null {
  const groupId = uiSelectedGroupId()
  if (!groupId) return null
  const freedIds = ungroupUserGroupInEngine(groupId)
  if (!freedIds.length) return null
  selectEntities(freedIds)
  requestLayout()
  return freedIds
}

// --- Edge Commands ---

export {
  createEdges,
  deleteEdges,
  removeEdgesTouchingEntities,
} from '../workspace-edges'

export function updateEdge(
  id: string,
  patch: {
    fromEntityId?: string
    toEntityId?: string
    fromEnd?: EdgeEnd
    toEnd?: EdgeEnd
    fromSide?: EdgeSide
    toSide?: EdgeSide
    color?: string
    label?: string
  },
): boolean {
  const edge = workspaceEdges.find((e) => e.id === id)
  if (!edge) return false
  if (patch.fromEntityId !== undefined) edge.fromEntityId = patch.fromEntityId
  if (patch.toEntityId !== undefined) edge.toEntityId = patch.toEntityId
  if (patch.fromEnd !== undefined) edge.fromEnd = patch.fromEnd
  if (patch.toEnd !== undefined) edge.toEnd = patch.toEnd
  if (patch.fromSide !== undefined) edge.fromSide = patch.fromSide
  if (patch.toSide !== undefined) edge.toSide = patch.toSide
  if (patch.color !== undefined) edge.color = patch.color || undefined
  if (patch.label !== undefined) edge.label = patch.label || undefined
  markDirty('canvas')
  scheduleWorkspaceAutosave()
  requestLayout()
  return true
}

export function deleteEdge(id: string): boolean {
  const idx = workspaceEdges.findIndex((e) => e.id === id)
  if (idx === -1) return false
  workspaceEdges.splice(idx, 1)
  updateSelectionForRemovedEntity(id)
  markDirty('canvas')
  scheduleWorkspaceAutosave()
  requestLayout()
  return true
}

// --- Layout Task Commands ---

export {
  applyTaskLayout,
  layoutComponentStates,
} from '../workspace-layout-tasks'

// --- Text Entity Commands ---

export function createTextEntity(input: {
  canvasX: number
  canvasY: number
  text?: string
  color?: string
  textStyle?: import('../../shared/types').TextEntityStyle
  widthMode?: import('../../shared/types').TextWidthMode
  textSize?: number
  width?: number
  height?: number
  id?: string
}): TextEntity {
  const entity = createTextEntityInState(input)
  scheduleWorkspaceAutosave()
  requestLayout()
  return entity
}

export function updateTextEntity(id: string, patch: Partial<Omit<TextEntity, 'id'>>): TextEntity | null {
  const snapped = { ...patch }
  if (snapped.width !== undefined) snapped.width = snapToGrid(snapped.width)
  if (snapped.height !== undefined) snapped.height = snapToGrid(snapped.height)
  if (snapped.canvasX !== undefined) snapped.canvasX = snapToGrid(snapped.canvasX)
  if (snapped.canvasY !== undefined) snapped.canvasY = snapToGrid(snapped.canvasY)
  const entity = updateTextEntityInState(id, snapped)
  if (entity) {
    updateResizeGuides(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return entity
}

export function deleteTextEntity(id: string): boolean {
  cancelEditingEntityIfMatches(id)
  const deleted = deleteTextEntityInState(id)
  if (deleted) {
    removeEdgesTouchingEntities(new Set([id]))
    updateSelectionForRemovedEntity(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return deleted
}

export function getTextEntities(): TextEntity[] {
  return [...textEntities]
}

// --- File Entity Commands ---

export function createFileEntity(input: {
  canvasX: number
  canvasY: number
  file: string
  subpath?: string
  width?: number
  height?: number
  id?: string
  metadata?: Record<string, unknown>
}): FileEntity {
  const entity = createFileEntityInState(input)
  scheduleWorkspaceAutosave()
  requestLayout()
  return entity
}

export function updateFileEntity(id: string, patch: Partial<Omit<FileEntity, 'id'>>): FileEntity | null {
  const snapped = { ...patch }
  if (snapped.canvasX !== undefined) snapped.canvasX = snapToGrid(snapped.canvasX)
  if (snapped.canvasY !== undefined) snapped.canvasY = snapToGrid(snapped.canvasY)
  const entity = updateFileEntityInState(id, snapped)
  if (entity) {
    updateResizeGuides(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return entity
}

export function deleteFileEntity(id: string): boolean {
  cancelEditingEntityIfMatches(id)
  const deleted = deleteFileEntityInState(id)
  if (deleted) {
    removeEdgesTouchingEntities(new Set([id]))
    updateSelectionForRemovedEntity(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return deleted
}

export function getFileEntities(): FileEntity[] {
  return [...fileEntities]
}

export function updateDrawingEntity(
  id: string,
  patch: Partial<Omit<DrawingEntity, 'id'>>,
): DrawingEntity | null {
  const snapped = { ...patch }
  if (snapped.width !== undefined) snapped.width = snapToGrid(snapped.width)
  if (snapped.height !== undefined) snapped.height = snapToGrid(snapped.height)
  if (snapped.canvasX !== undefined) snapped.canvasX = snapToGrid(snapped.canvasX)
  if (snapped.canvasY !== undefined) snapped.canvasY = snapToGrid(snapped.canvasY)
  const entity = updateDrawingEntityInState(id, snapped)
  if (entity) {
    updateResizeGuides(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return entity
}

export function deleteDrawingEntity(id: string): boolean {
  cancelEditingEntityIfMatches(id)
  const deleted = deleteDrawingEntityInState(id)
  if (deleted) {
    removeEdgesTouchingEntities(new Set([id]))
    updateSelectionForRemovedEntity(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return deleted
}

export function createDrawingEntity(input: {
  canvasX: number
  canvasY: number
  width: number
  height: number
  strokes: AnnotationDrawingStroke[]
  id?: string
}): DrawingEntity {
  const entity = createDrawingEntityInState(input)
  scheduleWorkspaceAutosave()
  requestLayout()
  return entity
}

export function getDrawingEntities(): DrawingEntity[] {
  return [...drawingEntities]
}

// --- Shape Entity Commands ---

export function createShapeEntity(input: {
  canvasX: number
  canvasY: number
  shapeKind?: ShapeEntity['shapeKind']
  width?: number
  height?: number
  text?: string
  color?: string
  strokeWidth?: number
  textSize?: number
  id?: string
}): ShapeEntity {
  const entity = createShapeEntityInState(input)
  scheduleWorkspaceAutosave()
  requestLayout()
  return entity
}

export function updateShapeEntity(
  id: string,
  patch: Partial<Omit<ShapeEntity, 'id'>>,
): ShapeEntity | null {
  const snapped = { ...patch }
  if (snapped.width !== undefined) snapped.width = snapToGrid(snapped.width)
  if (snapped.height !== undefined) snapped.height = snapToGrid(snapped.height)
  if (snapped.canvasX !== undefined) snapped.canvasX = snapToGrid(snapped.canvasX)
  if (snapped.canvasY !== undefined) snapped.canvasY = snapToGrid(snapped.canvasY)
  const entity = updateShapeEntityInState(id, snapped)
  if (entity) {
    updateResizeGuides(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return entity
}

export function deleteShapeEntity(id: string): boolean {
  cancelEditingEntityIfMatches(id)
  const deleted = deleteShapeEntityInState(id)
  if (deleted) {
    removeEdgesTouchingEntities(new Set([id]))
    updateSelectionForRemovedEntity(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return deleted
}

export function getShapeEntities(): ShapeEntity[] {
  return [...shapeEntities]
}

export function updateGroupEntity(
  id: string,
  patch: Partial<Omit<WorkspaceGroup, 'id' | 'kind'>>,
): WorkspaceGroup | null {
  const snapped = { ...patch }
  if (snapped.width !== undefined) snapped.width = snapToGrid(snapped.width)
  if (snapped.height !== undefined) snapped.height = snapToGrid(snapped.height)
  if (snapped.canvasX !== undefined) snapped.canvasX = snapToGrid(snapped.canvasX)
  if (snapped.canvasY !== undefined) snapped.canvasY = snapToGrid(snapped.canvasY)
  const entity = updateGroupEntityInState(id, snapped)
  if (entity) {
    updateResizeGuides(id)
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return entity
}

// --- Multi-Selection Resize (no grid snap) ---

export interface MultiResizeEntry {
  id: string
  kind: 'page' | 'text' | 'file' | 'drawing' | 'shape'
  width: number
  height: number
  canvasX: number
  canvasY: number
  /** Scaled strokes for drawing entities — undefined for all other kinds. */
  strokes?: AnnotationDrawingStroke[]
}

export function resizeMultiSelection(entries: MultiResizeEntry[]): void {
  let changed = false
  for (const entry of entries) {
    if (entry.kind === 'page') {
      const page = pages.find((p) => p.id === entry.id)
      if (!page) continue
      const currentSize = pageContentSize(page)
      const nextSize = { width: entry.width, height: entry.height }
      const sizeChanged =
        nextSize.width !== currentSize.width || nextSize.height !== currentSize.height
      if (pageUsesCustomSize(page.metadata) || sizeChanged) {
        let meta = setCustomPageSizeMetadata(page.metadata, nextSize)
        if (sizeChanged && deviceIdFromMetadata(meta)) {
          meta = setDeviceIdMetadata(meta, null)
        }
        page.metadata = meta
      }
      page.canvasX = entry.canvasX
      page.canvasY = entry.canvasY
      changed = true
    } else if (entry.kind === 'text') {
      const entity = updateTextEntityInState(entry.id, {
        width: entry.width,
        height: entry.height,
        canvasX: entry.canvasX,
        canvasY: entry.canvasY,
      })
      if (entity) changed = true
    } else if (entry.kind === 'file') {
      const entity = updateFileEntityInState(entry.id, {
        width: entry.width,
        height: entry.height,
        canvasX: entry.canvasX,
        canvasY: entry.canvasY,
      })
      if (entity) changed = true
    } else if (entry.kind === 'drawing') {
      const drawingPatch: Parameters<typeof updateDrawingEntityInState>[1] = {
        width: entry.width,
        height: entry.height,
        canvasX: entry.canvasX,
        canvasY: entry.canvasY,
      }
      if (entry.strokes !== undefined) drawingPatch.strokes = entry.strokes
      const entity = updateDrawingEntityInState(entry.id, drawingPatch)
      if (entity) changed = true
    } else if (entry.kind === 'shape') {
      const entity = updateShapeEntityInState(entry.id, {
        width: entry.width,
        height: entry.height,
        canvasX: entry.canvasX,
        canvasY: entry.canvasY,
      })
      if (entity) changed = true
    }
  }
  if (changed) {
    scheduleWorkspaceAutosave()
    requestLayout()
  }
}

// --- Device Page Commands ---

export function setPagePreset(pageId: string, presetIndex: number): void {
  if (presetIndex < 0 || presetIndex >= VIEWPORT_PRESETS.length) return
  const page = pages.find((p) => p.id === pageId)
  if (!page) return
  page.presetIndex = presetIndex
  let meta = clearCustomPageSizeMetadata(page.metadata) ?? {}
  // Auto-assign device based on the new preset
  const matchedDevice = deviceForPresetIndex(presetIndex)
  meta = setDeviceIdMetadata(meta, matchedDevice?.id ?? null)
  page.metadata = meta
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

export function setPageCustom(pageId: string): void {
  const page = pages.find((p) => p.id === pageId)
  if (!page) return
  const size = pageContentSize(page)
  let meta = setCustomPageSizeMetadata(page.metadata, size)
  meta = setDeviceIdMetadata(meta, null)
  page.metadata = meta
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

export function setDeviceOrientation(pageId: string, orientation: DeviceOrientation): void {
  const page = pages.find((p) => p.id === pageId)
  if (!page) return
  let meta = page.metadata ?? {}
  meta = setDeviceOrientationMetadata(meta, orientation)
  page.metadata = meta
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

export function toggleDeviceShell(pageId: string): void {
  const page = pages.find((p) => p.id === pageId)
  if (!page) return
  let meta = page.metadata ?? {}
  const current = showDeviceFrameFromMetadata(meta)
  meta = setShowDeviceFrameMetadata(meta, !current)
  page.metadata = meta
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

export function toggleSvgDeviceShell(pageId: string): void {
  const page = pages.find((p) => p.id === pageId)
  if (!page) return
  let meta = page.metadata ?? {}
  const current = useSvgDeviceShellFromMetadata(meta)
  meta = setUseSvgDeviceShellMetadata(meta, !current)
  page.metadata = meta
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

// --- File Device Commands ---

export function setFilePreset(fileId: string, presetIndex: number): void {
  if (presetIndex < 0 || presetIndex >= VIEWPORT_PRESETS.length) return
  const entity = fileEntities.find((e) => e.id === fileId)
  if (!entity) return
  const preset = VIEWPORT_PRESETS[presetIndex]
  entity.presetIndex = presetIndex
  entity.width = preset.width
  entity.height = preset.height
  let meta = clearCustomPageSizeMetadata(entity.metadata) ?? {}
  const matchedDevice = deviceForPresetIndex(presetIndex)
  meta = setDeviceIdMetadata(meta, matchedDevice?.id ?? null)
  entity.metadata = meta
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

export function setFileCustom(fileId: string): void {
  const entity = fileEntities.find((e) => e.id === fileId)
  if (!entity) return
  let meta = setCustomPageSizeMetadata(entity.metadata, { width: entity.width, height: entity.height })
  meta = setDeviceIdMetadata(meta, null)
  entity.metadata = meta
  entity.presetIndex = undefined
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

export function setFileDeviceOrientation(fileId: string, orientation: DeviceOrientation): void {
  const entity = fileEntities.find((e) => e.id === fileId)
  if (!entity) return
  // Swap width/height when changing orientation (only for preset sizes)
  const meta = entity.metadata ?? {}
  const currentOrientation = deviceOrientationFromMetadata(meta)
  if (currentOrientation !== orientation && entity.presetIndex !== undefined) {
    const temp = entity.width
    entity.width = entity.height
    entity.height = temp
  }
  entity.metadata = setDeviceOrientationMetadata(meta, orientation)
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

export function toggleFileDeviceShell(fileId: string): void {
  const entity = fileEntities.find((e) => e.id === fileId)
  if (!entity) return
  let meta = entity.metadata ?? {}
  const current = showDeviceFrameFromMetadata(meta)
  meta = setShowDeviceFrameMetadata(meta, !current)
  entity.metadata = meta
  scheduleWorkspaceAutosave()
  markDirty('canvas')
  requestLayout()
  markUndoBoundary()
}

// --- Annotation Commands ---

export {
  createAnnotation,
  updateAnnotationStatus,
  addAnnotationReply,
  moveAnnotation,
  deleteAnnotation,
} from '../workspace-annotations'
