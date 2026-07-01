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

import { useMemo, type CSSProperties } from 'react'
import type {
  CanvasSceneDrawingEntity,
  CanvasSceneEntity,
  CanvasSceneFileEntity,
  CanvasScenePageEntity,
  CanvasSceneGroupEntity,
  CanvasSceneShapeEntity,
  CanvasSceneTextEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { entityVisualScreenRect, type Camera } from '../../shared/coords'
import { selectionColor } from '../canvas-bg/canvasBgConstants'
import { MULTI_SELECTION_OUTLINE_PADDING_PX } from '../../shared/canvas-hit-geometry'
import { CornerResizeHandle, EdgeResizeHandle } from '../canvas-bg/ResizeHandles'
import { SelectionResizeGrid } from '../canvas-bg/SelectionResizeGrid'

interface PageOutlineProps {
  page: CanvasScenePageEntity
  camera: Camera
  originY: number
  isDark: boolean
  showResizeHandles: boolean
}

function SelectionOutlineBox({
  span,
  camera,
  originY,
  isDark,
  showResizeHandles,
  cursor,
}: {
  span: SelectedEntitySpan
  camera: Camera
  originY: number
  isDark: boolean
  showResizeHandles: boolean
  cursor?: CSSProperties['cursor']
}) {
  const b = entityVisualScreenRect(span, camera)
  return (
    <div
      className="absolute border-2"
      style={{
        left: b.screenX - 2,
        top: b.screenY - 2 - originY,
        width: b.screenWidth + 4,
        height: b.screenHeight + 4,
        borderColor: selectionColor(isDark),
        pointerEvents: 'none',
        cursor,
      }}
      data-overlay-ui
    >
      {showResizeHandles ? (
        <SelectionResizeGrid isDark={isDark} />
      ) : null}
    </div>
  )
}

function PageSelectionOverlay({ page, camera, originY, isDark, showResizeHandles }: PageOutlineProps) {
  return (
    <SelectionOutlineBox
      span={page}
      camera={camera}
      originY={originY}
      isDark={isDark}
      showResizeHandles={showResizeHandles}
    />
  )
}

function PageHoverOutline({
  page,
  camera,
  originY,
  isDark,
}: {
  page: CanvasScenePageEntity
  camera: Camera
  originY: number
  isDark: boolean
}) {
  return (
    <SelectionOutlineBox
      span={page}
      camera={camera}
      originY={originY}
      isDark={isDark}
      showResizeHandles={false}
    />
  )
}

interface EntityOutlineProps {
  entity:
    | CanvasSceneTextEntity
    | CanvasSceneFileEntity
    | CanvasSceneDrawingEntity
    | CanvasSceneShapeEntity
  camera: Camera
  originY: number
  isDark: boolean
  isSelected: boolean
  showResizeHandles: boolean
}

function EntitySelectionOverlay({
  entity,
  camera,
  originY,
  isDark,
  isSelected,
  showResizeHandles,
}: EntityOutlineProps) {
  return (
    <SelectionOutlineBox
      span={entity}
      camera={camera}
      originY={originY}
      isDark={isDark}
      showResizeHandles={isSelected && showResizeHandles}
      cursor={isSelected && entity.kind === 'drawing' ? 'grab' : undefined}
    />
  )
}

/** A selectable scene entity whose outer visual rect the layer projects through
 *  the camera to draw its outline / bounding box (ADR 0023 Phase 2). */
export type SelectedEntitySpan = CanvasSceneEntity

function MultiSelectionBoundingBox({
  selectedEntities,
  camera,
  originY,
  isDark,
}: {
  selectedEntities: SelectedEntitySpan[]
  camera: Camera
  originY: number
  isDark: boolean
}) {
  const screenBbox = useMemo(() => {
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    for (const e of selectedEntities) {
      const b = entityVisualScreenRect(e, camera)
      minX = Math.min(minX, b.screenX)
      minY = Math.min(minY, b.screenY)
      maxX = Math.max(maxX, b.screenX + b.screenWidth)
      maxY = Math.max(maxY, b.screenY + b.screenHeight)
    }
    return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
  }, [selectedEntities, camera])

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
  camera,
  originY,
  isDark,
}: {
  group: CanvasSceneGroupEntity
  camera: Camera
  originY: number
  isDark: boolean
}) {
  return (
    <SelectionOutlineBox
      span={group}
      camera={camera}
      originY={originY}
      isDark={isDark}
      showResizeHandles
    />
  )
}

export function SelectionOutlineLayer({
  layoutData,
  isDark,
  marqueePreviewIds,
  reorderGhostId,
  reorderGhostSpan,
  suppressPageId,
}: {
  layoutData: LayoutUpdateData
  isDark: boolean
  marqueePreviewIds: Set<string> | null
  /** The focused page during a focus session (ADR 0021): its own selection box
   *  and resize handles are suppressed for a clean read, but every other item's
   *  selection/hover outline still renders so annotations stay interactive. */
  suppressPageId?: string | null
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
  // layoutData is structurally a Camera; project entity rects through it (the
  // layer rides the reproject container, so the payload camera is correct).
  const camera: Camera = layoutData
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
          f.id !== suppressPageId &&
          (selectedIdSet.has(f.id) ||
            f.id === hoveredEntityId ||
            marqueePreviewIds?.has(f.id)),
      ),
    [pages, selectedIdSet, hoveredEntityId, marqueePreviewIds, reorderGhostId, suppressPageId],
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
    for (const f of pages) if (selectedIdSet.has(f.id) && f.id !== suppressPageId) out.push(f)
    for (const e of textEntities) if (selectedIdSet.has(e.id)) out.push(e)
    for (const e of fileEntities) if (selectedIdSet.has(e.id)) out.push(e)
    for (const e of drawingEntities) if (selectedIdSet.has(e.id)) out.push(e)
    for (const e of shapeEntities) if (selectedIdSet.has(e.id)) out.push(e)
    if (reorderGhostSpan) out.push(reorderGhostSpan)
    return out
  }, [isMultiSelect, pages, textEntities, fileEntities, drawingEntities, shapeEntities, selectedIdSet, reorderGhostSpan, suppressPageId])

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
          camera={camera}
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
              camera={camera}
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
            camera={camera}
            originY={originY}
            isDark={isDark}
          />
        )
      })}
      {visibleEntities.map((entity) => {
        const isSelected = selectedIdSet.has(entity.id)
        return (
          <EntitySelectionOverlay
            key={`selection-outline-${entity.id}`}
            entity={entity}
            camera={camera}
            originY={originY}
            isDark={isDark}
            isSelected={isSelected}
            showResizeHandles={!isMultiSelect}
          />
        )
      })}
      {selectedGroup ? (
        <GroupSelectionOverlay
          group={selectedGroup}
          camera={camera}
          originY={originY}
          isDark={isDark}
        />
      ) : null}
    </>
  )
}
