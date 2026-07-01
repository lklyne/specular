import type { CanvasSceneEntity, LayoutUpdateData, WorkspaceBounds } from './types'

export type CanvasPoint = { x: number; y: number }
export type ScreenPoint = { x: number; y: number }
export type ScreenRect = { left: number; top: number; width: number; height: number }
export type CanvasRect = { canvasX: number; canvasY: number; width: number; height: number }

/**
 * The three numbers that map canvas space to screen space: `pan` and `zoom`
 * (the camera) plus `canvasOrigin` (the fixed toolbar/sidebar inset). A
 * `LayoutUpdateData` is structurally a `Camera`, so payload-driven callers can
 * still pass the whole layout; live callers (hit-testing, popovers) pass the
 * nudge-driven camera so their projection tracks the gesture instead of the
 * last payload (ADR 0023 Phase 2).
 */
export type Camera = {
  pan: CanvasPoint
  zoom: number
  canvasOrigin: CanvasPoint
}

export function canvasToScreenX(cam: Camera, x: number): number {
  return x * cam.zoom + cam.pan.x + cam.canvasOrigin.x
}

export function canvasToScreenY(cam: Camera, y: number): number {
  return y * cam.zoom + cam.pan.y + cam.canvasOrigin.y
}

export function canvasToScreenPoint(cam: Camera, point: CanvasPoint): ScreenPoint {
  return {
    x: canvasToScreenX(cam, point.x),
    y: canvasToScreenY(cam, point.y),
  }
}

/** Project a canvas-space rect to a window-space screen rect. */
export function canvasRectToScreenRect(
  cam: Camera,
  rect: { canvasX: number; canvasY: number; width: number; height: number },
): ScreenRect {
  return {
    left: canvasToScreenX(cam, rect.canvasX),
    top: canvasToScreenY(cam, rect.canvasY),
    width: rect.width * cam.zoom,
    height: rect.height * cam.zoom,
  }
}

export function screenPointToCanvasPoint(
  clientX: number,
  clientY: number,
  cam: Camera,
): CanvasPoint {
  return {
    x: (clientX - cam.canvasOrigin.x - cam.pan.x) / cam.zoom,
    y: (clientY - cam.canvasOrigin.y - cam.pan.y) / cam.zoom,
  }
}

export function screenRectToCanvasRect(
  rect: ScreenRect,
  cam: Camera,
): WorkspaceBounds {
  return {
    x: (rect.left - cam.canvasOrigin.x - cam.pan.x) / cam.zoom,
    y: (rect.top - cam.canvasOrigin.y - cam.pan.y) / cam.zoom,
    width: rect.width / cam.zoom,
    height: rect.height / cam.zoom,
  }
}

export function toOverlayY(layout: LayoutUpdateData, value: number): number {
  return value - layout.canvasOrigin.y
}

/**
 * Outer visual rect (device shell / card frame) of an entity in canvas space.
 * Pages/files carry it explicitly (the shell insets differ per device and the
 * page vs file inset direction differs); every other kind's visual rect is just
 * its base rect. Consumers project this through the camera for selection chrome,
 * hit-testing, and shell rendering (ADR 0023 Phase 2).
 */
export function visualCanvasRect(entity: CanvasSceneEntity): CanvasRect {
  if (entity.kind === 'page' || entity.kind === 'file') {
    return {
      canvasX: entity.visualCanvasX,
      canvasY: entity.visualCanvasY,
      width: entity.visualWidth,
      height: entity.visualHeight,
    }
  }
  return { canvasX: entity.canvasX, canvasY: entity.canvasY, width: entity.width, height: entity.height }
}

/** Window-space bounds an entity occupies, named to drop in for the removed
 *  `screenX/screenY/screenWidth/screenHeight` payload fields. */
export type ScreenBounds = {
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
}

/** Project an entity's outer visual (shell) rect to window space via `cam`. */
export function entityVisualScreenRect(entity: CanvasSceneEntity, cam: Camera): ScreenBounds {
  const r = visualCanvasRect(entity)
  return {
    screenX: canvasToScreenX(cam, r.canvasX),
    screenY: canvasToScreenY(cam, r.canvasY),
    screenWidth: r.width * cam.zoom,
    screenHeight: r.height * cam.zoom,
  }
}

/** Project an entity's inner content rect to window space via `cam`. Equals
 *  `entityVisualScreenRect` for entities without a device shell. */
export function entityContentScreenRect(entity: CanvasSceneEntity, cam: Camera): ScreenBounds {
  const r = contentCanvasRect(entity)
  return {
    screenX: canvasToScreenX(cam, r.canvasX),
    screenY: canvasToScreenY(cam, r.canvasY),
    screenWidth: r.width * cam.zoom,
    screenHeight: r.height * cam.zoom,
  }
}

/**
 * Inner content (web viewport) rect of an entity in canvas space. Present as
 * explicit fields on pages/files only when the device frame insets the body;
 * otherwise it's the entity's base rect.
 */
export function contentCanvasRect(entity: CanvasSceneEntity): CanvasRect {
  if (
    (entity.kind === 'page' || entity.kind === 'file') &&
    entity.contentCanvasX !== undefined &&
    entity.contentCanvasY !== undefined &&
    entity.contentCanvasWidth !== undefined &&
    entity.contentCanvasHeight !== undefined
  ) {
    return {
      canvasX: entity.contentCanvasX,
      canvasY: entity.contentCanvasY,
      width: entity.contentCanvasWidth,
      height: entity.contentCanvasHeight,
    }
  }
  return { canvasX: entity.canvasX, canvasY: entity.canvasY, width: entity.width, height: entity.height }
}
