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

/**
 * aboveView's WCV origin sits at `canvasOrigin.y` below the toolbar, so a
 * pointer event's `clientY` (relative to the overlay) must add that offset
 * to land in window space, where scene entities' `screenY` lives.
 */
export function clientYToWindowY(clientY: number, layout: LayoutUpdateData): number {
  return clientY + layout.canvasOrigin.y
}

export interface ScreenContentBounds {
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  contentScreenX?: number
  contentScreenY?: number
  contentScreenWidth?: number
  contentScreenHeight?: number
}

/**
 * A page/file entity's outer `screen*` bounds include device-shell chrome;
 * `contentScreen*` (when present) is the inner web-viewport rect. Falls back
 * to the outer bounds when there's no shell.
 */
export function pageContentBounds(page: ScreenContentBounds): ScreenRect {
  return {
    left: page.contentScreenX ?? page.screenX,
    top: page.contentScreenY ?? page.screenY,
    width: page.contentScreenWidth ?? page.screenWidth,
    height: page.contentScreenHeight ?? page.screenHeight,
  }
}

/** Whether a window-space point (`clientX`, `windowY`) falls inside a page's content rect. */
export function isPointerInPageContent(
  clientX: number,
  windowY: number,
  page: ScreenContentBounds,
): boolean {
  const bounds = pageContentBounds(page)
  return (
    clientX >= bounds.left &&
    clientX <= bounds.left + bounds.width &&
    windowY >= bounds.top &&
    windowY <= bounds.top + bounds.height
  )
}
