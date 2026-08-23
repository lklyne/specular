import type { LayoutUpdateData, WorkspaceBounds } from './types'
import {
  projectPointToScreen,
  unprojectFromScreen,
  unprojectPointFromScreen,
} from './scene-projection'

/** The camera a payload was built at, in the shape the projection takes. */
function cameraOf(layout: LayoutUpdateData) {
  return { zoom: layout.zoom, pan: layout.pan }
}

export type CanvasPoint = { x: number; y: number }
export type ScreenPoint = { x: number; y: number }
export type ScreenRect = { left: number; top: number; width: number; height: number }

export function canvasToScreenX(layout: LayoutUpdateData, x: number): number {
  return canvasToScreenPoint(layout, { x, y: 0 }).x
}

export function canvasToScreenY(layout: LayoutUpdateData, y: number): number {
  return canvasToScreenPoint(layout, { x: 0, y }).y
}

export function canvasToScreenPoint(layout: LayoutUpdateData, point: CanvasPoint): ScreenPoint {
  return projectPointToScreen(point, cameraOf(layout), layout.canvasOrigin)
}

export function screenPointToCanvasPoint(
  clientX: number,
  clientY: number,
  layout: LayoutUpdateData,
): CanvasPoint {
  return unprojectPointFromScreen(
    { x: clientX, y: clientY },
    cameraOf(layout),
    layout.canvasOrigin,
  )
}

export function screenRectToCanvasRect(
  rect: ScreenRect,
  layout: LayoutUpdateData,
): WorkspaceBounds {
  return unprojectFromScreen(
    { x: rect.left, y: rect.top, width: rect.width, height: rect.height },
    cameraOf(layout),
    layout.canvasOrigin,
  )
}

export function toOverlayY(layout: LayoutUpdateData, value: number): number {
  return value - layout.canvasOrigin.y
}

export function clientYToWindowY(clientY: number, layout: LayoutUpdateData): number {
  return clientY + layout.canvasOrigin.y
}
