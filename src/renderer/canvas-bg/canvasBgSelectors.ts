import type {
  LayoutUpdateData,
} from '../../shared/types'
import { snapToGrid, screenPointToCanvasPoint } from '../../shared/gesture-utils'
import { projectToScreen } from '../../shared/scene-projection'

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
  const rect = projectToScreen(
    {
      x: snapToGrid(point.x),
      y: snapToGrid(point.y),
      width: layoutData.pendingPlacement.width,
      height: layoutData.pendingPlacement.height,
    },
    { zoom: layoutData.zoom, pan: layoutData.pan },
    layoutData.canvasOrigin,
  )
  return {
    entityKind: layoutData.pendingPlacement.entityKind,
    shapeKind: layoutData.pendingPlacement.shapeKind,
    textStyle: layoutData.pendingPlacement.textStyle,
    color: layoutData.pendingPlacement.color,
    textSize: layoutData.pendingPlacement.textSize,
    zoom: layoutData.zoom,
    left: rect.x,
    top: rect.y,
    width: rect.width,
    height: rect.height,
  }
}

