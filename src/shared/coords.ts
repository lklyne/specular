import type { WorkspaceBounds } from './types'
import {
  projectPointToScreen,
  unprojectFromScreen,
  unprojectPointFromScreen,
  type SceneView,
} from './scene-projection'

/** The camera a payload was built at, in the shape the projection takes. */
function cameraOf(layout: SceneView) {
  return { zoom: layout.zoom, pan: layout.pan }
}

export type CanvasPoint = { x: number; y: number }
export type ScreenPoint = { x: number; y: number }
export type ScreenRect = { left: number; top: number; width: number; height: number }

export function canvasToScreenX(layout: SceneView, x: number): number {
  return canvasToScreenPoint(layout, { x, y: 0 }).x
}

export function canvasToScreenY(layout: SceneView, y: number): number {
  return canvasToScreenPoint(layout, { x: 0, y }).y
}

export function canvasToScreenPoint(layout: SceneView, point: CanvasPoint): ScreenPoint {
  return projectPointToScreen(point, cameraOf(layout), layout.canvasOrigin)
}

export function screenPointToCanvasPoint(
  clientX: number,
  clientY: number,
  layout: SceneView,
): CanvasPoint {
  return unprojectPointFromScreen(
    { x: clientX, y: clientY },
    cameraOf(layout),
    layout.canvasOrigin,
  )
}

export function screenRectToCanvasRect(
  rect: ScreenRect,
  layout: SceneView,
): WorkspaceBounds {
  return unprojectFromScreen(
    { x: rect.left, y: rect.top, width: rect.width, height: rect.height },
    cameraOf(layout),
    layout.canvasOrigin,
  )
}

export function toOverlayY(layout: SceneView, value: number): number {
  return value - layout.canvasOrigin.y
}

export function clientYToWindowY(clientY: number, layout: SceneView): number {
  return clientY + layout.canvasOrigin.y
}
