/**
 * SelectionOutlineLayer — selection outlines, multi-selection bounding box,
 * and resize handles. Lives in aboveView so it paints above page WCVs.
 *
 * Resize hit-tests run in `useCanvasPointerRouter` against entity geometry,
 * so the handles here are visual-only — pointer-events stay off.
 *
 * Coordinates are overlay-local: aboveView's WCV origin sits at
 * `canvasOrigin.y`, so window-space `screenY` is offset by that amount
 * (matching `useAnchoredPosition` and the rest of aboveView).
 */

import { useMemo } from 'react'
import type {
  CanvasSceneDrawingEntity,
  CanvasSceneFileEntity,
  CanvasScenePageEntity,
  CanvasSceneGroupEntity,
  CanvasSceneShapeEntity,
  CanvasSceneTextEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { selectionColor } from '../canvas-bg/canvasBgConstants'
import { aspectRatioResizeModeForCanvasFile } from '../canvas-bg/entityConstants'
import {
  MIN_FILE_HEIGHT,
  MIN_FILE_WIDTH,
  MIN_GROUP_HEIGHT,
  MIN_GROUP_WIDTH,
  MIN_SHAPE_HEIGHT,
  MIN_SHAPE_WIDTH,
  MIN_TEXT_HEIGHT,
  MIN_TEXT_WIDTH,
} from '../canvas-bg/entityConstants'
import { MULTI_SELECTION_OUTLINE_PADDING_PX } from '../../shared/canvas-hit-geometry'
import { entityResizesAutomatically } from '../../shared/hit-test'
import { CornerResizeHandle, EdgeResizeHandle } from '../canvas-bg/ResizeHandles'
import { SelectionResizeGrid } from '../canvas-bg/SelectionResizeGrid'

interface PageOutlineProps {
  page: CanvasScenePageEntity
  originY: number
  isDark: boolean
  showResizeHandles: boolean
}

function PageSelectionOverlay({ page, originY, isDark, showResizeHandles }: PageOutlineProps) {
  const zoom = page.width > 0 ? page.screenWidth / page.width : 1
  return (
    <div
      className="absolute border-2"
      style={{
        left: page.screenX - 2,
        top: page.screenY - 2 - originY,
        width: page.screenWidth + 4,
        height: page.screenHeight + 4,
        borderColor: selectionColor(isDark),
        pointerEvents: 'none',
      }}
      data-overlay-ui
    >
      {showResizeHandles ? (
        <SelectionResizeGrid
          id={page.id}
          width={page.width}
          height={page.height}
          canvasX={page.canvasX}
          canvasY={page.canvasY}
          zoom={zoom}
          minWidth={320}
          minHeight={200}
          onResize={() => {
            /* hit-test in router drives resize; visual handles only */
          }}
          isDark={isDark}
        />
      ) : null}
    </div>
  )
}

function PageHoverOutline({
  page,
  originY,
  isDark,
}: {
  page: CanvasScenePageEntity
  originY: number
  isDark: boolean
}) {
  return (
    <div
      className="absolute border-2"
      style={{
        left: page.screenX - 2,
        top: page.screenY - 2 - originY,
        width: page.screenWidth + 4,
        height: page.screenHeight + 4,
        borderColor: selectionColor(isDark),
        pointerEvents: 'none',
      }}
      data-overlay-ui
    />
  )
}

interface EntityOutlineProps {
  entity:
    | CanvasSceneTextEntity
    | CanvasSceneFileEntity
    | CanvasSceneDrawingEntity
    | CanvasSceneShapeEntity
  originY: number
  borderRadius: number
  isDark: boolean
  isSelected: boolean
  showResizeHandles: boolean
}

function EntitySelectionOverlay({
  entity,
  originY,
  borderRadius,
  isDark,
  isSelected,
  showResizeHandles,
}: EntityOutlineProps) {
  const minWidth =
    entity.kind === 'text'
      ? MIN_TEXT_WIDTH
      : entity.kind === 'file'
        ? MIN_FILE_WIDTH
        : entity.kind === 'shape'
          ? MIN_SHAPE_WIDTH
          : 16
  const minHeight =
    entity.kind === 'text'
      ? MIN_TEXT_HEIGHT
      : entity.kind === 'file'
        ? MIN_FILE_HEIGHT
        : entity.kind === 'shape'
          ? MIN_SHAPE_HEIGHT
          : 16
  const aspectRatioResizeMode =
    entity.kind === 'file' ? aspectRatioResizeModeForCanvasFile(entity.file) : 'off'
  const zoom = entity.width > 0 ? entity.screenWidth / entity.width : 1

  return (
    <div
      className="absolute border-2"
      style={{
        left: entity.screenX - 2,
        top: entity.screenY - 2 - originY,
        width: entity.screenWidth + 4,
        height: entity.screenHeight + 4,
        borderColor: selectionColor(isDark),
        borderRadius,
        pointerEvents: 'none',
        cursor: isSelected && entity.kind === 'drawing' ? 'grab' : undefined,
      }}
      data-overlay-ui
    >
      {isSelected && showResizeHandles ? (
        <SelectionResizeGrid
          id={entity.id}
          width={entity.width}
          height={entity.height}
          canvasX={entity.canvasX}
          canvasY={entity.canvasY}
          zoom={zoom}
          minWidth={minWidth}
          minHeight={minHeight}
          onResize={() => {
            /* hit-test in router drives resize; visual handles only */
          }}
          aspectRatioResizeMode={aspectRatioResizeMode}
          isDark={isDark}
        />
      ) : null}
    </div>
  )
}

export interface SelectedEntitySpan {
  id: string
  kind: 'page' | 'text' | 'file' | 'drawing' | 'shape'
  canvasX: number
  canvasY: number
  width: number
  height: number
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
}

function MultiSelectionBoundingBox({
  selectedEntities,
  originY,
  isDark,
}: {
  selectedEntities: SelectedEntitySpan[]
  originY: number
  isDark: boolean
}) {
  const screenBbox = useMemo(() => {
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    for (const e of selectedEntities) {
      minX = Math.min(minX, e.screenX)
      minY = Math.min(minY, e.screenY)
      maxX = Math.max(maxX, e.screenX + e.screenWidth)
      maxY = Math.max(maxY, e.screenY + e.screenHeight)
    }
    return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
  }, [selectedEntities])

  const pad = MULTI_SELECTION_OUTLINE_PADDING_PX

  return (
    <div
      className="absolute border-2"
      style={{
        left: screenBbox.x - pad,
        top: screenBbox.y - pad - originY,
        width: screenBbox.width + pad * 2,
        height: screenBbox.height + pad * 2,
        borderColor: selectionColor(isDark),
        borderStyle: 'solid',
        pointerEvents: 'none',
      }}
      data-overlay-ui
    >
      <EdgeResizeHandle edge="top" />
      <EdgeResizeHandle edge="right" />
      <EdgeResizeHandle edge="bottom" />
      <EdgeResizeHandle edge="left" />
      <CornerResizeHandle corner="top-left" isDark={isDark} />
      <CornerResizeHandle corner="top-right" isDark={isDark} />
      <CornerResizeHandle corner="bottom-left" isDark={isDark} />
      <CornerResizeHandle corner="bottom-right" isDark={isDark} />
    </div>
  )
}

function GroupSelectionOverlay({
  group,
  originY,
  isDark,
}: {
  group: CanvasSceneGroupEntity
  originY: number
  isDark: boolean
}) {
  const zoom = group.width > 0 ? group.screenWidth / group.width : 1
  return (
    <div
      className="absolute border-2"
      data-overlay-ui
      style={{
        left: group.screenX - 2,
        top: group.screenY - 2 - originY,
        width: group.screenWidth + 4,
        height: group.screenHeight + 4,
        borderColor: selectionColor(isDark),
        borderRadius: 2,
        pointerEvents: 'none',
      }}
    >
      <SelectionResizeGrid
        id={group.id}
        width={group.width}
        height={group.height}
        canvasX={group.canvasX}
        canvasY={group.canvasY}
        zoom={zoom}
        minWidth={MIN_GROUP_WIDTH}
        minHeight={MIN_GROUP_HEIGHT}
        onResize={() => {
          /* hit-test in router drives resize; visual handles only */
        }}
        isDark={isDark}
      />
    </div>
  )
}

export function SelectionOutlineLayer({
  layoutData,
  isDark,
  marqueePreviewIds,
  reorderGhostId,
  reorderGhostSpan,
}: {
  layoutData: LayoutUpdateData
  isDark: boolean
  marqueePreviewIds: Set<string> | null
  /** While a reorder drag is in flight (ADR 0015 D7, Phase D), drop this entity's
   *  *per-item* outline — a crisp box fights the grayscale placeholder at its
   *  destination slot and the 50% ghost under the cursor. It stays in the
   *  multi-select bounding box (placeholder slot via `layoutData`, cursor via
   *  `reorderGhostSpan`), so the box wraps both. */
  reorderGhostId?: string | null
  /** While a reorder drag is in flight, the lifted item's ghost rect at the
   *  cursor — added to the multi-select bounding box so it resizes toward the
   *  cursor as the item is dragged out (FigJam parity). */
  reorderGhostSpan?: SelectedEntitySpan | null
}) {
  const originY = layoutData.canvasOrigin.y
  const selectedIdSet = useMemo(
    () => new Set(layoutData.selectedEntityIds),
    [layoutData.selectedEntityIds],
  )
  const isMultiSelect = selectedIdSet.size > 1
  const hoveredEntityId = layoutData.hover?.id ?? null

  const pages = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is CanvasScenePageEntity => e.kind === 'page',
      ),
    [layoutData.entities],
  )
  const textEntities = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is CanvasSceneTextEntity => e.kind === 'text',
      ),
    [layoutData.entities],
  )
  const fileEntities = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is CanvasSceneFileEntity => e.kind === 'file',
      ),
    [layoutData.entities],
  )
  const drawingEntities = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is CanvasSceneDrawingEntity => e.kind === 'drawing',
      ),
    [layoutData.entities],
  )
  const shapeEntities = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is CanvasSceneShapeEntity => e.kind === 'shape',
      ),
    [layoutData.entities],
  )

  // Pages render outline if selected, hovered, or in marquee preview.
  const visiblePages = useMemo(
    () =>
      pages.filter(
        (f) =>
          f.id !== reorderGhostId &&
          (selectedIdSet.has(f.id) ||
            f.id === hoveredEntityId ||
            marqueePreviewIds?.has(f.id)),
      ),
    [pages, selectedIdSet, hoveredEntityId, marqueePreviewIds, reorderGhostId],
  )

  // Non-page entities render outline if selected, hovered, or in marquee preview.
  const visibleEntities = useMemo(
    () =>
      [...textEntities, ...fileEntities, ...drawingEntities, ...shapeEntities].filter(
        (e) =>
          e.id !== reorderGhostId &&
          (selectedIdSet.has(e.id) ||
            e.id === hoveredEntityId ||
            marqueePreviewIds?.has(e.id)),
      ),
    [textEntities, fileEntities, drawingEntities, shapeEntities, selectedIdSet, hoveredEntityId, marqueePreviewIds, reorderGhostId],
  )

  // Multi-select bounding box: aggregate all selected entities' rects. During a
  // reorder drag the dragged item is included twice — once at its destination
  // slot (the grayscale placeholder, via `layoutData`) and once at the cursor
  // (`reorderGhostSpan`) — so the box wraps the drop location *and* resizes
  // toward the lifted item as it moves (FigJam parity). Its per-item outline is
  // still dropped below, so only the group box tracks them, not a crisp outline.
  const allSelectedEntities: SelectedEntitySpan[] = useMemo(() => {
    if (!isMultiSelect) return []
    const out: SelectedEntitySpan[] = []
    for (const f of pages) if (selectedIdSet.has(f.id)) out.push(f)
    for (const e of textEntities) if (selectedIdSet.has(e.id)) out.push(e)
    for (const e of fileEntities) if (selectedIdSet.has(e.id)) out.push(e)
    for (const e of drawingEntities) if (selectedIdSet.has(e.id)) out.push(e)
    for (const e of shapeEntities) if (selectedIdSet.has(e.id)) out.push(e)
    if (reorderGhostSpan) out.push(reorderGhostSpan)
    return out
  }, [isMultiSelect, pages, textEntities, fileEntities, drawingEntities, shapeEntities, selectedIdSet, reorderGhostSpan])

  // Group selection overlay — render whenever a group is selected. The
  // canvas-bg `GroupSelectionOverlayLayer` used to suppress this when the
  // group had a descendant page (handing off to the legacy aboveView path);
  // now aboveView owns it unconditionally, so we render in both cases.
  const selectedGroupId = layoutData.selectedGroupId ?? null
  const selectedGroup = useMemo(() => {
    if (!selectedGroupId) return null
    return (layoutData.groups ?? []).find((g) => g.id === selectedGroupId) ?? null
  }, [selectedGroupId, layoutData.groups])

  return (
    <>
      {isMultiSelect && allSelectedEntities.length > 1 ? (
        <MultiSelectionBoundingBox
          selectedEntities={allSelectedEntities}
          originY={originY}
          isDark={isDark}
        />
      ) : null}
      {visiblePages.map((page) => {
        const isSelected = selectedIdSet.has(page.id)
        if (isSelected) {
          return (
            <PageSelectionOverlay
              key={`selection-outline-${page.id}`}
              page={page}
              originY={originY}
              isDark={isDark}
              showResizeHandles={!isMultiSelect}
            />
          )
        }
        return (
          <PageHoverOutline
            key={`selection-outline-${page.id}`}
            page={page}
            originY={originY}
            isDark={isDark}
          />
        )
      })}
      {visibleEntities.map((entity) => {
        const isSelected = selectedIdSet.has(entity.id)
        const borderRadius = entity.kind === 'text' ? 0 : 4
        return (
          <EntitySelectionOverlay
            key={`selection-outline-${entity.id}`}
            entity={entity}
            originY={originY}
            borderRadius={borderRadius}
            isDark={isDark}
            isSelected={isSelected}
            showResizeHandles={!isMultiSelect && !entityResizesAutomatically(entity)}
          />
        )
      })}
      {selectedGroup ? (
        <GroupSelectionOverlay
          group={selectedGroup}
          originY={originY}
          isDark={isDark}
        />
      ) : null}
    </>
  )
}
