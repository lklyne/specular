/**
 * Document Commands
 *
 * The canonical entry points for mutations that change persisted workspace
 * data. Every exported command runs through `mutateWorkspace` (ADR 0025),
 * which owns the dirty → autosave → layout → undo-boundary trailer; the raw
 * per-kind state mutators (`*-entity-state.ts`) stay internal to commands.
 * UI-only mutations (selection, focus camera, tool state) live in
 * ui-actions.ts and do NOT participate in the undo stack.
 *
 * Multi-tick gestures (drag, resize, reorder, distribute) bracket their
 * per-tick mutations in a gesture session (`beginGestureSession` …
 * `finalize`), so the whole interaction collapses to one doc sync and one
 * undo step.
 */

import { GRID_SIZE, VIEWPORT_PRESETS } from '../../shared/constants'
import type { DeviceOrientation } from '../../shared/device-catalog'
import { deviceForPresetIndex } from '../../shared/device-catalog'
import type { AlignmentReferenceName } from '../../shared/canvas-guides'
import type { ResizeHandle } from '../../shared/resize-accumulator'
import type { AnnotationDrawingStroke, BatchLayoutMode, EdgeEnd, EdgeSide, LayoutDirective, PageColorScheme, WorkspaceBounds } from '../../shared/types'
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
import { makeAutoLayoutGroup } from '../managed-layout'
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
import { bumpFileReloadVersion } from './local-file-watcher'
import {
  deleteGroupEntity as deleteGroupEntityInState,
  updateGroupEntity as updateGroupEntityInState,
} from './group-entity-state'
import { markDirty } from './layout-dirty'
import { mutateWorkspace } from './mutate-workspace'
import {
  reanchorEntityById,
  withPageAnchoredEntityIds,
} from './page-anchor-state'
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
import { pan, zoom } from './runtime-context'
import { recenterFocusPresentation, requestLayout } from './viewport-control'
import {
  snapGeometryPatch,
  snapToGrid,
  type GeometryPatchKey,
} from '../../shared/gesture-utils'
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
import {
  detectReorderableRow,
  reorderRowPositions,
  SELECTION_ROW_GAP_TOLERANCE,
  type Box,
  type ReorderableRow,
} from '../../shared/reorder-row'
import { packedGapPositions } from '../../shared/gap-handles'
import { arrangeInSpan } from '../../shared/span-arrange'
import { alignmentGuideDetector } from './alignment-guide-detector'
import { broadcastCanvasGuides, clearCanvasGuides } from './canvas-guides'
import { distributionGuideDetector } from './distribution-guide-detector'
import { descendantEntityIdsForGroup } from './group-descendants'
import { applyLayoutDirective } from '../workspace-placement'
import { entityBoundsById, entityKindById } from '../workspace-entities'
import { resizeGuideReferencesForHandle } from './resize-guide-adapter'
import { workspaceEdges, workspaceGroups } from './workspace-model'
import { beginGestureSession, type GestureSession } from './workspace-gesture-session'
import { scheduleWorkspaceAutosave } from './workspace-autosave'
import {
  boundAvailableCanvasViewportRect,
  boundCanvasOrigin as canvasOrigin,
  pageContentSize,
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
let dragSession: GestureSession | null = null
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
  if (nudgeGuideTimer) {
    clearTimeout(nudgeGuideTimer)
    nudgeGuideTimer = null
  }
  dragAccumulatorById.clear()
  activeDraggedGuideIds = guideEntityIdsForDrag(entityIds)
  activeDragCandidates = snapCandidateSnapshot(
    { entities: currentSnapSnapshotEntities() },
    currentCanvasViewportRect(),
    entityIds,
  )
  dragSession = beginGestureSession()
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

const NUDGE_GUIDE_MS = 500
let nudgeGuideTimer: ReturnType<typeof setTimeout> | null = null

/**
 * Keyboard nudge. Unlike a drag this moves by an exact canvas delta — no
 * accumulator, no zoom division, no grid snapping: a 5px nudge off-grid must
 * stay off-grid, and the grid-step nudge lands on-grid only if it started there.
 */
export function nudgeSelection(dx: number, dy: number): void {
  const selectedIds = uiSelectedEntityIds()
  if (!selectedIds.length) return
  // Entities anchored to a nudged page travel with it; the directly-nudged
  // entities re-resolve their anchor from where they land.
  const entityIds = withPageAnchoredEntityIds(selectedIds)

  const moved = mutateWorkspace(
    () => {
      const movedIds = entityIds.filter((id) => {
        const entity = findMovableEntity(id)
        return entity ? moveEntityTo(id, entity.canvasX + dx, entity.canvasY + dy) : false
      })
      for (const id of selectedIds) reanchorEntityById(id)
      return movedIds
    },
    { changed: (ids) => ids.length > 0 },
  )
  if (moved.length) flashNudgeGuides(moved)
}

/** Show whatever guides the nudged position happens to align with, then fade. */
function flashNudgeGuides(entityIds: string[]): void {
  const candidates = snapCandidateSnapshot(
    { entities: currentSnapSnapshotEntities() },
    currentCanvasViewportRect(),
    entityIds,
  )
  const draggedRects = guideEntityIdsForDrag(entityIds)
    .map(currentSnapCandidateForEntity)
    .filter((candidate): candidate is SnapCandidate => candidate !== null)

  broadcastCanvasGuides({
    alignmentGuides: alignmentGuideDetector(draggedRects, candidates),
    distributionGuides: draggedRects.flatMap((dragged) => [
      ...distributionGuideDetector(dragged, candidates, 'horizontal'),
      ...distributionGuideDetector(dragged, candidates, 'vertical'),
    ]),
  })

  if (nudgeGuideTimer) clearTimeout(nudgeGuideTimer)
  nudgeGuideTimer = setTimeout(() => {
    nudgeGuideTimer = null
    clearCanvasGuides()
  }, NUDGE_GUIDE_MS)
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
  // Placement decides anchoring: entities dropped on a page hook to it,
  // entities dragged off go free-form. Runs before the session finalizes so
  // the anchor change lands in the drag's single undo step.
  for (const id of dragAccumulatorById.keys()) reanchorEntityById(id)
  dragAccumulatorById.clear()
  activeDragCandidates = []
  activeDraggedGuideIds = []
  clearCanvasGuides()
  dragSession?.finalize()
  dragSession = null
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

// --- Group Commands ---

export { deleteGroups } from '../workspace-groups'

export function groupSelectedEntities(): WorkspaceGroup | null {
  const ids = uiSelectedEntityIds()
  if (ids.length < 2) return null
  const group = createUserGroupInEngine(ids)
  selectGroup(group.id)
  return group
}

/**
 * Convert the current selection into an auto-layout (managed row) group: a
 * selected group is converted in place; a multi-selection is wrapped into a new
 * managed group. Reachability entry point for drag-reorder on arbitrary content
 * (plan O1). Returns the managed group or null. (ADR 0015)
 */
export function makeAutoLayoutFromSelection(): WorkspaceGroup | null {
  const groupId = uiSelectedGroupId()
  if (groupId) return makeAutoLayoutGroup({ groupId })

  const ids = uiSelectedEntityIds()
  if (ids.length < 2) return null
  const group = makeAutoLayoutGroup({ entityIds: ids })
  if (group) selectGroup(group.id)
  return group
}

export function ungroupSelectedGroup(): string[] | null {
  const groupId = uiSelectedGroupId()
  if (!groupId) return null
  const freedIds = ungroupUserGroupInEngine(groupId)
  if (!freedIds.length) return null
  selectEntities(freedIds)
  return freedIds
}

// --- Edge Commands ---

export {
  createEdges,
  deleteEdges,
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
  return mutateWorkspace(() => {
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
    return true
  }, { changed: (updated) => updated })
}

export function deleteEdge(id: string): boolean {
  return mutateWorkspace(() => {
    const idx = workspaceEdges.findIndex((e) => e.id === id)
    if (idx === -1) return false
    workspaceEdges.splice(idx, 1)
    updateSelectionForRemovedEntity(id)
    return true
  }, { changed: (deleted) => deleted })
}

// --- Layout Task Commands ---

export {
  applyTaskLayout,
  layoutComponentStates,
} from '../workspace-layout-tasks'

// --- Per-Kind Entity Commands ---

/** The shared update command: snap geometry, patch in state, refresh guides. */
function updateEntityCommand<P extends Partial<Record<GeometryPatchKey, number>>, E>(
  id: string,
  patch: P,
  updateInState: (id: string, patch: P) => E | null,
  snapKeys?: readonly GeometryPatchKey[],
): E | null {
  return mutateWorkspace(() => {
    const entity = updateInState(id, snapGeometryPatch(patch, snapKeys))
    if (entity) updateResizeGuides(id)
    return entity
  }, { changed: (entity) => entity !== null })
}

/** The shared delete command: drop the entity plus its edges and selection. */
function deleteEntityCommand(id: string, deleteInState: (id: string) => boolean): boolean {
  return mutateWorkspace(() => {
    cancelEditingEntityIfMatches(id)
    const deleted = deleteInState(id)
    if (deleted) {
      removeEdgesTouchingEntities(new Set([id]))
      updateSelectionForRemovedEntity(id)
    }
    return deleted
  }, { changed: (deleted) => deleted })
}

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
  return mutateWorkspace(() => {
    const entity = createTextEntityInState(input)
    reanchorEntityById(entity.id)
    return entity
  })
}

export function updateTextEntity(id: string, patch: Partial<Omit<TextEntity, 'id'>>): TextEntity | null {
  return updateEntityCommand(id, patch, updateTextEntityInState)
}

export function deleteTextEntity(id: string): boolean {
  return deleteEntityCommand(id, deleteTextEntityInState)
}

export function getTextEntities(): TextEntity[] {
  return [...textEntities]
}

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
  return mutateWorkspace(() => createFileEntityInState(input))
}

/** File entities snap position only — their size is intrinsic to the file. */
export function updateFileEntity(id: string, patch: Partial<Omit<FileEntity, 'id'>>): FileEntity | null {
  return updateEntityCommand(id, patch, updateFileEntityInState, ['canvasX', 'canvasY'])
}

export function deleteFileEntity(id: string): boolean {
  return deleteEntityCommand(id, deleteFileEntityInState)
}

/** Manual "Refresh" action: re-mounts the entity's renderer even if the
 *  watcher's own change signal never arrived. Not undo-tracked — it doesn't
 *  change persisted document data, just forces a re-fetch from disk. */
export function refreshFileEntity(id: string): void {
  if (!fileEntities.some((e) => e.id === id)) return
  bumpFileReloadVersion(id)
  markDirty('canvas')
  requestLayout()
}

export function getFileEntities(): FileEntity[] {
  return [...fileEntities]
}

export function createDrawingEntity(input: {
  canvasX: number
  canvasY: number
  width: number
  height: number
  strokes: AnnotationDrawingStroke[]
  id?: string
}): DrawingEntity {
  return mutateWorkspace(() => {
    const entity = createDrawingEntityInState(input)
    reanchorEntityById(entity.id)
    return entity
  })
}

export function updateDrawingEntity(
  id: string,
  patch: Partial<Omit<DrawingEntity, 'id'>>,
): DrawingEntity | null {
  return updateEntityCommand(id, carryStrokesOnMove(id, patch), updateDrawingEntityInState)
}

/**
 * A drawing's strokes live in absolute canvas coords, so writing `canvasX`/
 * `canvasY` is a move — the ink has to travel with the origin or the box drifts
 * away from it. Injects shifted strokes into the patch so a bare `update --at`
 * (or any canvas-apply move) stays correct without a dedicated move verb.
 * Callers that pass explicit `strokes` (resize) opt out.
 */
function carryStrokesOnMove(
  id: string,
  patch: Partial<Omit<DrawingEntity, 'id'>>,
): Partial<Omit<DrawingEntity, 'id'>> {
  if (patch.strokes !== undefined) return patch
  if (patch.canvasX === undefined && patch.canvasY === undefined) return patch
  const cur = drawingEntities.find((d) => d.id === id)
  if (!cur) return patch
  const dx = (patch.canvasX ?? cur.canvasX) - cur.canvasX
  const dy = (patch.canvasY ?? cur.canvasY) - cur.canvasY
  if (dx === 0 && dy === 0) return patch
  return {
    ...patch,
    strokes: cur.strokes.map((stroke) => ({
      ...stroke,
      points: stroke.points.map((p) => ({ x: p.x + dx, y: p.y + dy })),
    })),
  }
}

export function deleteDrawingEntity(id: string): boolean {
  return deleteEntityCommand(id, deleteDrawingEntityInState)
}

export function getDrawingEntities(): DrawingEntity[] {
  return [...drawingEntities]
}

export function createShapeEntity(input: {
  canvasX: number
  canvasY: number
  shapeKind?: ShapeEntity['shapeKind']
  width?: number
  height?: number
  text?: string
  color?: string
  strokeWidth?: number
  borderStyle?: ShapeEntity['borderStyle']
  borderColor?: string
  textSize?: number
  id?: string
}): ShapeEntity {
  return mutateWorkspace(() => {
    const entity = createShapeEntityInState(input)
    reanchorEntityById(entity.id)
    return entity
  })
}

export function updateShapeEntity(
  id: string,
  patch: Partial<Omit<ShapeEntity, 'id'>>,
): ShapeEntity | null {
  return updateEntityCommand(id, patch, updateShapeEntityInState)
}

export function deleteShapeEntity(id: string): boolean {
  return deleteEntityCommand(id, deleteShapeEntityInState)
}

export function getShapeEntities(): ShapeEntity[] {
  return [...shapeEntities]
}

export function updateGroupEntity(
  id: string,
  patch: Partial<Omit<WorkspaceGroup, 'id' | 'kind'>>,
): WorkspaceGroup | null {
  carryChildrenOnMove(id, patch)
  return updateEntityCommand(id, patch, updateGroupEntityInState)
}

/**
 * A group's children are absolute-positioned, not relative to the box, so moving
 * the box has to move them too — the same thing dragging does via drag-id
 * expansion. Runs inside the caller's transaction (canvas-apply's
 * `commitAsOneTransaction`), so box + children collapse to one undo step, and
 * the diff-sync picks up the raw child writes.
 */
function carryChildrenOnMove(
  id: string,
  patch: Partial<Omit<WorkspaceGroup, 'id' | 'kind'>>,
): void {
  if (patch.canvasX === undefined && patch.canvasY === undefined) return
  const cur = workspaceGroups.find((g) => g.id === id)
  if (!cur) return
  const dx = (patch.canvasX ?? cur.canvasX) - cur.canvasX
  const dy = (patch.canvasY ?? cur.canvasY) - cur.canvasY
  if (dx === 0 && dy === 0) return
  for (const descId of descendantEntityIdsForGroup(id)) {
    const desc = findMovableEntity(descId)
    if (!desc) continue
    desc.canvasX += dx
    desc.canvasY += dy
    shiftDrawingStrokes(descId, dx, dy)
  }
}

/**
 * Remove a group container only — children keep their geometry and
 * un-parent on the next restore. The registry's `group` handler dispatches
 * headless deletes here; dissolving members is `deleteGroups`.
 */
export function deleteGroupEntity(id: string): boolean {
  return mutateWorkspace(
    () => deleteGroupEntityInState(id),
    { changed: (deleted) => deleted },
  )
}

// --- Multi-Selection Resize (no grid snap) ---

export interface MultiResizeEntry {
  id: string
  kind: 'page' | 'text' | 'file' | 'drawing' | 'shape'
  width: number
  height: number
  canvasX: number
  canvasY: number
  /** Bounds-transformed strokes for drawing entities — undefined for all other kinds. */
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

/** Write a single entity's canvas origin through its per-kind mutator. */
function writeReorderedPosition(
  id: string,
  kind: SnapCandidateSnapshotEntity['kind'],
  pos: { x: number; y: number },
): boolean {
  if (kind === 'page') {
    const page = pages.find((p) => p.id === id)
    if (!page) return false
    page.canvasX = pos.x
    page.canvasY = pos.y
    return true
  }
  const patch = { canvasX: pos.x, canvasY: pos.y }
  if (kind === 'text') return updateTextEntityInState(id, patch) !== null
  if (kind === 'file') return updateFileEntityInState(id, patch) !== null
  if (kind === 'drawing') return updateDrawingEntityInState(id, patch) !== null
  if (kind === 'shape') return updateShapeEntityInState(id, patch) !== null
  return false
}

/**
 * Build the frozen `ReorderableRow` for a selection from live geometry, or null
 * when the selection isn't an eligible equal-gap row. Shared by the selection
 * reorder door's gesture (freeze at start, drop-index per move) and its commit
 * (`reorderSelection`) so both read the row off the same boxes.
 */
export function buildSelectionRow(orderedIds: string[]): ReorderableRow | null {
  const geometryById = new Map(
    currentSnapSnapshotEntities().map((entity) => [entity.id, entity] as const),
  )
  const boxes: Box[] = []
  for (const id of orderedIds) {
    const entity = geometryById.get(id)
    if (!entity) continue
    boxes.push({
      id,
      x: entity.canvasX,
      y: entity.canvasY,
      width: entity.width,
      height: entity.height,
    })
  }
  return detectReorderableRow(boxes, { gapTolerance: SELECTION_ROW_GAP_TOLERANCE })
}

/**
 * Write a repacked position set through each entity's per-kind mutator inside
 * one gesture session — the shared commit tail of `reorderSelection` and
 * `applySelectionGap`. One undo step; nothing persists but the positions.
 */
function commitRepackedPositions(positions: Map<string, { x: number; y: number }>): boolean {
  if (positions.size === 0) return false
  const kindById = new Map(
    currentSnapSnapshotEntities().map((entity) => [entity.id, entity.kind] as const),
  )
  const session = beginGestureSession()
  let changed = false
  for (const [id, pos] of positions) {
    const kind = kindById.get(id)
    if (kind && writeReorderedPosition(id, kind, pos)) changed = true
  }
  if (changed) scheduleWorkspaceAutosave()
  session.finalize()

  if (changed) requestLayout()
  return changed
}

/**
 * Selection reorder commit (ADR 0015 D7) — the position-only sibling of
 * `reorderManagedChild`. Geometry is the source of truth: the row is read off
 * the current boxes, repacked with `movingId` at `dropIndex`, and only the
 * changed origins are written. **No** `entityOrder` write, **no**
 * `managedLayout`, **no** `commitAsOneTransaction` — nothing persists but the
 * new positions.
 *
 * No-op (returns false) when the selection isn't an eligible equal-gap row or
 * the move changes nothing.
 */
export function reorderSelection(
  orderedIds: string[],
  movingId: string,
  dropIndex: number,
): boolean {
  const row = buildSelectionRow(orderedIds)
  if (!row) return false
  return commitRepackedPositions(reorderRowPositions(row, movingId, dropIndex))
}

/**
 * Selection gap commit (ADR 0015 Milestone 2) — the positions-only sibling of
 * `setGroupLayoutGap`, mirroring how `reorderSelection` sits beside
 * `reorderManagedChild`. Repacks the entities along `axis` at `gap` (anchored
 * at the first item, each keeping its own cross-axis coordinate) and writes
 * only the changed origins through each entity's per-kind mutator inside one
 * gesture session — a single undo step, nothing persisted but the positions.
 *
 * No-op (returns false) when the selection is no longer an eligible equal-gap
 * row along `axis` (same commit-time re-validation as `reorderSelection`) or
 * nothing moves.
 */
export function applySelectionGap(
  orderedIds: string[],
  axis: 'x' | 'y',
  gap: number,
): boolean {
  const row = buildSelectionRow(orderedIds)
  if (!row || row.axis !== axis) return false
  const children = row.order.flatMap((id) => {
    const box = row.boxesById.get(id)
    return box
      ? [{ id, canvasX: box.x, canvasY: box.y, width: box.width, height: box.height }]
      : []
  })
  return commitRepackedPositions(packedGapPositions(children, axis, gap, { keepCross: true }))
}

/**
 * Arrange a set of entities into a row, column, or grid — the shared brain
 * behind both the popup toolbar (via IPC) and the `arrange` CLI verb (via
 * `/selection/arrange`). One gesture session = one undo step. No-op (false)
 * for fewer than 2 movable entities.
 *
 * Two modes, chosen by whether the caller passes an explicit `gap`:
 *
 * - No gap (toolbar always; CLI default) → *tidy in place*: keep the cluster's
 *   current footprint and just regularize the spacing inside it (row/column
 *   even the gaps along one axis and align the other; grid keeps the existing
 *   2-D structure, holes and all). The footprint the user built is visible and
 *   intentional — a fixed gap is neither.
 * - Explicit gap (CLI `--gap`) → *pack*: collapse to that gap from the
 *   cluster's top-left, in reading order. `cols` only applies here.
 */
export function arrangeEntities(
  entityIds: string[],
  mode: BatchLayoutMode,
  opts: Pick<LayoutDirective, 'gap' | 'cols'> = {},
): boolean {
  if (opts.gap !== undefined) return packEntities(entityIds, mode, opts)

  const geometryById = new Map(
    currentSnapSnapshotEntities().map((entity) => [entity.id, entity] as const),
  )
  const boxes: Box[] = []
  for (const id of entityIds) {
    const entity = geometryById.get(id)
    if (!entity) continue
    boxes.push({
      id,
      x: entity.canvasX,
      y: entity.canvasY,
      width: entity.width,
      height: entity.height,
    })
  }

  const targets = arrangeInSpan(boxes, mode)
  if (!targets) return false

  const session = beginGestureSession()
  let changed = false
  for (const [id, pos] of targets) {
    if (moveEntityTo(id, pos.x, pos.y)) changed = true
  }
  if (changed) scheduleWorkspaceAutosave()
  session.finalize()
  if (changed) requestLayout()
  return changed
}

/**
 * Pack entities tight into a row/column/grid at a fixed gap from the cluster's
 * top-left, in reading order (top-to-bottom, left-to-right) so the result
 * follows reading order regardless of caller id order. The `--gap` path of the
 * `arrange` verb; the toolbar never reaches here.
 */
function packEntities(
  entityIds: string[],
  mode: BatchLayoutMode,
  opts: Pick<LayoutDirective, 'gap' | 'cols'>,
): boolean {
  const withBounds = entityIds
    .map((id) => ({ id, bounds: entityBoundsById(id) }))
    .filter((e): e is { id: string; bounds: WorkspaceBounds } => e.bounds !== null)
  if (withBounds.length < 2) return false

  // Reading-order sort. The band tolerates minor vertical misalignment so a
  // rough row doesn't sort by pixel-exact y. ponytail: fixed band; upgrade to
  // per-row clustering if rows of very different heights sort wrong.
  const band = Math.max(GRID_SIZE, Math.min(...withBounds.map((e) => e.bounds.height)) / 2)
  withBounds.sort((a, b) =>
    Math.abs(a.bounds.y - b.bounds.y) > band
      ? a.bounds.y - b.bounds.y
      : a.bounds.x - b.bounds.x,
  )

  const ids = withBounds.map((e) => e.id)
  let positions: { canvasX: number; canvasY: number }[]
  try {
    positions = applyLayoutDirective({
      layout: { kind: mode, gap: opts.gap, cols: opts.cols },
      items: ids.map((id) => ({ id })),
    }).positions
  } catch {
    return false
  }

  const session = beginGestureSession()
  let changed = false
  for (let i = 0; i < ids.length; i++) {
    if (moveEntityTo(ids[i], positions[i].canvasX, positions[i].canvasY)) changed = true
  }
  if (changed) scheduleWorkspaceAutosave()
  session.finalize()
  if (changed) requestLayout()
  return changed
}

/**
 * Move an entity to an absolute canvas position. Groups drag their descendants
 * by the same delta; drawings carry their strokes (stored in absolute coords,
 * so a bare origin write would leave the ink behind). Returns false when the
 * entity is missing or already there.
 */
function moveEntityTo(id: string, targetX: number, targetY: number): boolean {
  const entity = findMovableEntity(id)
  if (!entity) return false
  const dx = targetX - entity.canvasX
  const dy = targetY - entity.canvasY
  if (dx === 0 && dy === 0) return false
  entity.canvasX = targetX
  entity.canvasY = targetY
  shiftDrawingStrokes(id, dx, dy)
  if (entityKindById(id) === 'group') {
    for (const descId of descendantEntityIdsForGroup(id)) {
      const desc = findMovableEntity(descId)
      if (!desc) continue
      desc.canvasX += dx
      desc.canvasY += dy
      shiftDrawingStrokes(descId, dx, dy)
    }
  }
  // Direct field writes (unlike the per-kind mutators) don't self-dirty, so the
  // canvas won't repaint until an unrelated event dirties it — mark it here.
  markDirty('canvas', 'sidebar')
  return true
}

// --- Device Commands (pages and file entities) ---

/**
 * The kind-specific half of a device command. Pages derive their size from
 * the preset and recenter an active focus presentation; file entities carry
 * explicit width/height, so presets write size and orientation swaps it.
 */
interface DeviceTarget {
  getMetadata(): Record<string, unknown> | undefined
  setMetadata(meta: Record<string, unknown>): void
  applyPreset(presetIndex: number): void
  currentContentSize(): { width: number; height: number }
  clearPresetIndex?(): void
  applyOrientation?(orientation: DeviceOrientation): void
  recenterId?: string
}

function pageDeviceTarget(pageId: string): DeviceTarget | null {
  const page = pages.find((p) => p.id === pageId)
  if (!page) return null
  return {
    getMetadata: () => page.metadata,
    setMetadata: (meta) => { page.metadata = meta },
    applyPreset: (presetIndex) => { page.presetIndex = presetIndex },
    currentContentSize: () => pageContentSize(page),
    recenterId: pageId,
  }
}

function fileDeviceTarget(fileId: string): DeviceTarget | null {
  const entity = fileEntities.find((e) => e.id === fileId)
  if (!entity) return null
  return {
    getMetadata: () => entity.metadata,
    setMetadata: (meta) => { entity.metadata = meta },
    applyPreset: (presetIndex) => {
      const preset = VIEWPORT_PRESETS[presetIndex]
      entity.presetIndex = presetIndex
      entity.width = preset.width
      entity.height = preset.height
    },
    currentContentSize: () => ({ width: entity.width, height: entity.height }),
    clearPresetIndex: () => { entity.presetIndex = undefined },
    applyOrientation: (orientation) => {
      // Swap width/height when changing orientation (only for preset sizes)
      const currentOrientation = deviceOrientationFromMetadata(entity.metadata ?? {})
      if (currentOrientation !== orientation && entity.presetIndex !== undefined) {
        const temp = entity.width
        entity.width = entity.height
        entity.height = temp
      }
    },
  }
}

function setDevicePreset(target: DeviceTarget, presetIndex: number): void {
  mutateWorkspace(() => {
    target.applyPreset(presetIndex)
    let meta = clearCustomPageSizeMetadata(target.getMetadata()) ?? {}
    // Auto-assign device based on the new preset
    const matchedDevice = deviceForPresetIndex(presetIndex)
    meta = setDeviceIdMetadata(meta, matchedDevice?.id ?? null)
    target.setMetadata(meta)
    if (target.recenterId) recenterFocusPresentation(target.recenterId)
  })
}

function setDeviceCustom(target: DeviceTarget): void {
  mutateWorkspace(() => {
    const size = target.currentContentSize()
    let meta = setCustomPageSizeMetadata(target.getMetadata(), size)
    meta = setDeviceIdMetadata(meta, null)
    target.setMetadata(meta)
    target.clearPresetIndex?.()
    if (target.recenterId) recenterFocusPresentation(target.recenterId)
  })
}

function setDeviceTargetOrientation(target: DeviceTarget, orientation: DeviceOrientation): void {
  mutateWorkspace(() => {
    target.applyOrientation?.(orientation)
    target.setMetadata(setDeviceOrientationMetadata(target.getMetadata() ?? {}, orientation))
    if (target.recenterId) recenterFocusPresentation(target.recenterId)
  })
}

function toggleDeviceFlag(
  target: DeviceTarget,
  read: (meta: Record<string, unknown>) => boolean,
  write: (meta: Record<string, unknown>, value: boolean) => Record<string, unknown>,
): void {
  mutateWorkspace(() => {
    const meta = target.getMetadata() ?? {}
    target.setMetadata(write(meta, !read(meta)))
  })
}

function validPresetIndex(presetIndex: number): boolean {
  return presetIndex >= 0 && presetIndex < VIEWPORT_PRESETS.length
}

export function setPagePreset(pageId: string, presetIndex: number): void {
  if (!validPresetIndex(presetIndex)) return
  const target = pageDeviceTarget(pageId)
  if (target) setDevicePreset(target, presetIndex)
}

export function setPageColorScheme(
  pageId: string,
  colorScheme: PageColorScheme | null,
): void {
  const page = pages.find((p) => p.id === pageId)
  if (!page) return
  mutateWorkspace(() => {
    page.colorScheme = colorScheme ?? undefined
  })
}

export function setPageCustom(pageId: string): void {
  const target = pageDeviceTarget(pageId)
  if (target) setDeviceCustom(target)
}

export function setDeviceOrientation(pageId: string, orientation: DeviceOrientation): void {
  const target = pageDeviceTarget(pageId)
  if (target) setDeviceTargetOrientation(target, orientation)
}

export function toggleDeviceShell(pageId: string): void {
  const target = pageDeviceTarget(pageId)
  if (target) toggleDeviceFlag(target, showDeviceFrameFromMetadata, setShowDeviceFrameMetadata)
}

export function toggleSvgDeviceShell(pageId: string): void {
  const target = pageDeviceTarget(pageId)
  if (target) toggleDeviceFlag(target, useSvgDeviceShellFromMetadata, setUseSvgDeviceShellMetadata)
}

export function setFilePreset(fileId: string, presetIndex: number): void {
  if (!validPresetIndex(presetIndex)) return
  const target = fileDeviceTarget(fileId)
  if (target) setDevicePreset(target, presetIndex)
}

export function setFileCustom(fileId: string): void {
  const target = fileDeviceTarget(fileId)
  if (target) setDeviceCustom(target)
}

export function setFileDeviceOrientation(fileId: string, orientation: DeviceOrientation): void {
  const target = fileDeviceTarget(fileId)
  if (target) setDeviceTargetOrientation(target, orientation)
}

export function toggleFileDeviceShell(fileId: string): void {
  const target = fileDeviceTarget(fileId)
  if (target) toggleDeviceFlag(target, showDeviceFrameFromMetadata, setShowDeviceFrameMetadata)
}

// --- Annotation Commands ---

export {
  createAnnotation,
  updateAnnotationStatus,
  addAnnotationReply,
  deleteAnnotation,
} from '../workspace-annotations'
