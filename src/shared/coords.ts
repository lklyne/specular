import type { LayoutUpdateData, WorkspaceBounds } from './types'

export type CanvasPoint = { x: number; y: number }
export type ScreenPoint = { x: number; y: number }
export type ScreenRect = { left: number; top: number; width: number; height: number }

export function canvasToScreenX(layout: LayoutUpdateData, x: number): number {
  return x * layout.zoom + layout.pan.x + layout.canvasOrigin.x
}

export function canvasToScreenY(layout: LayoutUpdateData, y: number): number {
  return y * layout.zoom + layout.pan.y + layout.canvasOrigin.y
}

export function canvasToScreenPoint(layout: LayoutUpdateData, point: CanvasPoint): ScreenPoint {
  return {
    x: canvasToScreenX(layout, point.x),
    y: canvasToScreenY(layout, point.y),
  }
}

export function screenPointToCanvasPoint(
  clientX: number,
  clientY: number,
  layout: LayoutUpdateData,
): CanvasPoint {
  return {
    x: (clientX - layout.canvasOrigin.x - layout.pan.x) / layout.zoom,
    y: (clientY - layout.canvasOrigin.y - layout.pan.y) / layout.zoom,
  }
}

export function screenRectToCanvasRect(
  rect: ScreenRect,
  layout: LayoutUpdateData,
): WorkspaceBounds {
  return {
    x: (rect.left - layout.canvasOrigin.x - layout.pan.x) / layout.zoom,
    y: (rect.top - layout.canvasOrigin.y - layout.pan.y) / layout.zoom,
    width: rect.width / layout.zoom,
    height: rect.height / layout.zoom,
  }
}

export function toOverlayY(layout: LayoutUpdateData, value: number): number {
  return value - layout.canvasOrigin.y
}

export function clientYToWindowY(clientY: number, layout: LayoutUpdateData): number {
  return clientY + layout.canvasOrigin.y
}
