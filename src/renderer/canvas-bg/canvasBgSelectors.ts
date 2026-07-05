import type {
  LayoutUpdateData,
} from '../../shared/types'
import { snapToGrid, screenPointToCanvasPoint } from '../../shared/gesture-utils'

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

