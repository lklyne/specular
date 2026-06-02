import type {
  CanvasSceneEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { snapToGrid, screenPointToCanvasPoint } from '../../shared/gesture-utils'
import { unionScreenBounds } from './canvasGeometry'

function buildSelectedPageIdSet(selectedEntityIds: string[]): Set<string> {
  return new Set(selectedEntityIds)
}

function buildSelectionBounds(layoutData: LayoutUpdateData) {
  return unionScreenBounds(layoutData.entities, layoutData.selectedEntityIds)
}

export function buildPendingPlacementPreview(
  layoutData: LayoutUpdateData,
  placementCursor: { clientX: number; clientY: number } | null,
) {
  if (!layoutData.pendingPlacement || !placementCursor) return null
  const point = screenPointToCanvasPoint(
    placementCursor.clientX,
    placementCursor.clientY,
    layoutData,
  )
  const snappedX = snapToGrid(point.x)
  const snappedY = snapToGrid(point.y)
  return {
    entityKind: layoutData.pendingPlacement.entityKind,
    shapeKind: layoutData.pendingPlacement.shapeKind,
    textStyle: layoutData.pendingPlacement.textStyle,
    color: layoutData.pendingPlacement.color,
    textSize: layoutData.pendingPlacement.textSize,
    zoom: layoutData.zoom,
    left: layoutData.canvasOrigin.x + layoutData.pan.x + snappedX * layoutData.zoom,
    top: layoutData.canvasOrigin.y + layoutData.pan.y + snappedY * layoutData.zoom,
    width: layoutData.pendingPlacement.width * layoutData.zoom,
    height: layoutData.pendingPlacement.height * layoutData.zoom,
  }
}

function buildActionAnchorTop(
  layoutData: LayoutUpdateData,
  selectionBounds: ReturnType<typeof unionScreenBounds>,
): number {
  return selectionBounds
    ? Math.max(layoutData.canvasOrigin.y + 40, selectionBounds.top)
    : layoutData.canvasOrigin.y + 40
}

function buildHighlightedEntities(
  entities: CanvasSceneEntity[],
  selectedEntityIdSet: Set<string>,
  hoveredEntityId: string | null,
) {
  return entities.filter(
    (entity) => selectedEntityIdSet.has(entity.id) || entity.id === hoveredEntityId,
  )
}
