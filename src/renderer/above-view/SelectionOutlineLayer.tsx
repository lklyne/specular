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

import type {
  ProjectedDrawingEntity,
  ProjectedFileEntity,
  ProjectedGroupEntity,
  ProjectedLayoutData,
  ProjectedPageEntity,
  ProjectedShapeEntity,
  ProjectedTextEntity,
} from '../../shared/scene-projection'
import { memo, type CSSProperties, useMemo } from 'react'
import { SELECTION_OUTLINE_PADDING_PX } from '../../shared/canvas-hit-geometry'
import { selectionColor } from '../canvas-bg/canvasBgConstants'
import { CornerResizeHandle, EdgeResizeHandle } from '../canvas-bg/ResizeHandles'
import { SelectionResizeGrid } from '../canvas-bg/SelectionResizeGrid'
import { useHoveredEntityId } from '../shared/hooks/useHoveredEntityId'

interface PageOutlineProps {
  page: ProjectedPageEntity
  originY: number
  isDark: boolean
  showResizeHandles: boolean
}

function SelectionOutlineBox({
  span,
  originY,
  isDark,
  showResizeHandles,
  cursor,
}: {
  span: Pick<SelectedEntitySpan, 'screenX' | 'screenY' | 'screenWidth' | 'screenHeight'>
  originY: number
  isDark: boolean
  showResizeHandles: boolean
  cursor?: CSSProperties['cursor']
}) {
  return (
    <div
      className="absolute border"
      style={{
        left: span.screenX - SELECTION_OUTLINE_PADDING_PX,
        top: span.screenY - SELECTION_OUTLINE_PADDING_PX - originY,
        width: span.screenWidth + SELECTION_OUTLINE_PADDING_PX * 2,
        height: span.screenHeight + SELECTION_OUTLINE_PADDING_PX * 2,
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

function PageSelectionOverlay({ page, originY, isDark, showResizeHandles }: PageOutlineProps) {
  return (
    <SelectionOutlineBox
      span={page}
      originY={originY}
      isDark={isDark}
      showResizeHandles={showResizeHandles}
    />
  )
}

function PageHoverOutline({
  page,
  originY,
  isDark,
}: {
  page: ProjectedPageEntity
  originY: number
  isDark: boolean
}) {
  return (
    <SelectionOutlineBox
      span={page}
      originY={originY}
      isDark={isDark}
      showResizeHandles={false}
    />
  )
}

interface EntityOutlineProps {
  entity:
    | ProjectedTextEntity
    | ProjectedFileEntity
    | ProjectedDrawingEntity
    | ProjectedShapeEntity
  originY: number
  isDark: boolean
  isSelected: boolean
  showResizeHandles: boolean
}

function EntitySelectionOverlay({
  entity,
  originY,
  isDark,
  isSelected,
  showResizeHandles,
}: EntityOutlineProps) {
  return (
    <SelectionOutlineBox
      span={entity}
      originY={originY}
      isDark={isDark}
      showResizeHandles={isSelected && showResizeHandles}
      cursor={isSelected && entity.kind === 'drawing' ? 'grab' : undefined}
    />
  )
}

export interface SelectedEntitySpan {
  id: string
  kind: 'page' | 'text' | 'file' | 'drawing' | 'shape' | 'group'
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

  const pad = SELECTION_OUTLINE_PADDING_PX

  return (
    <div
      className="absolute border"
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
  showResizeHandles,
}: {
  group: ProjectedGroupEntity
  originY: number
  isDark: boolean
  showResizeHandles: boolean
}) {
  return (
    <SelectionOutlineBox
      span={group}
      originY={originY}
      isDark={isDark}
      showResizeHandles={showResizeHandles}
    />
  )
}

export const SelectionOutlineLayer = memo(function SelectionOutlineLayer({
  layoutData,
  isDark,
  marqueePreviewIds,
  reorderGhostId,
  reorderGhostSpan,
  suppressFocusedId,
  suppressPageHover = false,
}: {
  layoutData: ProjectedLayoutData
  isDark: boolean
  marqueePreviewIds: Set<string> | null
  /** The focus session's target — a page, or a note drawn fullscreen by
   *  FocusedNoteLayer (ADR 0021). Its own selection box and resize handles are
   *  suppressed for a clean read, but every other item's selection/hover
   *  outline still renders so annotations stay interactive. */
  suppressFocusedId?: string | null
  /** Command-drag places an item above a page without binding to it, so the
   *  page's hover outline must disappear with the disabled drop target. */
  suppressPageHover?: boolean
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
  // Operand ids (ADR 0034, `resolveSelectionScope`): groups in the selection
  // expanded to every descendant. The multi-select bbox unions these, not
  // `selectedIdSet` (top-level members only) — a group contributes its full
  // extent instead of nothing.
  const operandIdSet = useMemo(
    () => new Set(layoutData.selectionOperandIds),
    [layoutData.selectionOperandIds],
  )
  const hoveredEntityId = useHoveredEntityId()

  const pages = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is ProjectedPageEntity => e.kind === 'page',
      ),
    [layoutData.entities],
  )
  const textEntities = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is ProjectedTextEntity => e.kind === 'text',
      ),
    [layoutData.entities],
  )
  const fileEntities = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is ProjectedFileEntity => e.kind === 'file',
      ),
    [layoutData.entities],
  )
  const drawingEntities = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is ProjectedDrawingEntity => e.kind === 'drawing',
      ),
    [layoutData.entities],
  )
  const shapeEntities = useMemo(
    () =>
      layoutData.entities.filter(
        (e): e is ProjectedShapeEntity => e.kind === 'shape',
      ),
    [layoutData.entities],
  )

  // Pages render outline if selected, hovered, or in marquee preview.
  const visiblePages = useMemo(
    () =>
      pages.filter(
        (f) =>
          f.id !== reorderGhostId &&
          f.id !== suppressFocusedId &&
          (selectedIdSet.has(f.id) ||
            (!suppressPageHover && f.id === hoveredEntityId) ||
            marqueePreviewIds?.has(f.id)),
      ),
    [pages, selectedIdSet, hoveredEntityId, marqueePreviewIds, reorderGhostId, suppressPageHover, suppressFocusedId],
  )

  // Non-page entities render outline if selected, hovered, or in marquee preview.
  const visibleEntities = useMemo(
    () =>
      [...textEntities, ...fileEntities, ...drawingEntities, ...shapeEntities].filter(
        (e) =>
          e.id !== reorderGhostId &&
          e.id !== suppressFocusedId &&
          (selectedIdSet.has(e.id) ||
            e.id === hoveredEntityId ||
            marqueePreviewIds?.has(e.id)),
      ),
    [textEntities, fileEntities, drawingEntities, shapeEntities, selectedIdSet, hoveredEntityId, marqueePreviewIds, reorderGhostId, suppressFocusedId],
  )

  // Multi-select bounding box: aggregate all selected *operands'* rects. A
  // selected group contributes its own rect (the visual unit, padding
  // included) so the box wraps the group border, plus its descendants via the
  // operand set. During a reorder drag the dragged item is included twice —
  // once at its destination slot (the grayscale placeholder, via
  // `layoutData`) and once at the cursor (`reorderGhostSpan`) — so the box
  // wraps the drop location *and* resizes toward the lifted item as it moves
  // (FigJam parity). Its per-item outline is still dropped below, so only the
  // group box tracks them, not a crisp outline.
  const allSelectedEntities: SelectedEntitySpan[] = useMemo(() => {
    if (!isMultiSelect) return []
    const contributes = (id: string) => operandIdSet.has(id) && id !== suppressFocusedId
    const byKind: SelectedEntitySpan[][] = [
      pages,
      textEntities,
      fileEntities,
      drawingEntities,
      shapeEntities,
      layoutData.groups ?? [],
    ]
    const out = byKind.flatMap((spans) => spans.filter((span) => contributes(span.id)))
    if (reorderGhostSpan) out.push(reorderGhostSpan)
    return out
  }, [isMultiSelect, pages, textEntities, fileEntities, drawingEntities, shapeEntities, layoutData.groups, operandIdSet, reorderGhostSpan, suppressFocusedId])

  // Group selection overlay — render whenever a group is selected. The
  // canvas-bg `GroupSelectionOverlayLayer` used to suppress this when the
  // group had a descendant page (handing off to the legacy aboveView path);
  // now aboveView owns it unconditionally, so we render in both cases.
  const selectedGroups = useMemo(
    () => (layoutData.groups ?? []).filter((group) => selectedIdSet.has(group.id)),
    [layoutData.groups, selectedIdSet],
  )
  const marqueePreviewGroups = useMemo(
    () =>
      (layoutData.groups ?? []).filter(
        (group) =>
          !selectedIdSet.has(group.id) &&
          marqueePreviewIds?.has(group.id),
      ),
    [layoutData.groups, marqueePreviewIds, selectedIdSet],
  )

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
        return (
          <EntitySelectionOverlay
            key={`selection-outline-${entity.id}`}
            entity={entity}
            originY={originY}
            isDark={isDark}
            isSelected={isSelected}
            showResizeHandles={!isMultiSelect}
          />
        )
      })}
      {selectedGroups.map((group) => (
        <GroupSelectionOverlay
          key={`selection-outline-${group.id}`}
          group={group}
          originY={originY}
          isDark={isDark}
          showResizeHandles={!isMultiSelect}
        />
      ))}
      {marqueePreviewGroups.map((group) => (
        <GroupSelectionOverlay
          key={`marquee-outline-${group.id}`}
          group={group}
          originY={originY}
          isDark={isDark}
          showResizeHandles={false}
        />
      ))}
    </>
  )
})
