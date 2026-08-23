/**
 * Canvas space → screen space, in one place.
 *
 * A scene entity's authored geometry is canvas-space; where it lands on screen
 * is a function of the camera and the fixed canvas inset (`sceneOrigin`, the
 * toolbar band). Main projects for the native `WebContentsView` bounds it must
 * hand Chromium in window pixels; renderers project for everything they draw.
 * Both call through here so the two can't drift apart — a second copy of
 * `canvasX * zoom + pan.x` is the bug this module exists to prevent.
 *
 * Device-shelled entities carry two rects: the shell (bezel) and the body (the
 * web viewport inside it). The insets between them are canvas-space, so they
 * scale with zoom like everything else. Pages anchor at the shell — `canvasX`
 * is the bezel's top-left, and the body sits inset from it. File entities
 * anchor at the body and grow the shell outward. Each kind's projector below
 * encodes its own convention.
 */

import {
  CUSTOM_SHELL_INSETS,
  shellInsetsForDevice,
  type DeviceOrientation,
  type ShellInsets,
} from './device-catalog'

export interface CanvasRect {
  x: number
  y: number
  width: number
  height: number
}

export interface ScreenRect {
  x: number
  y: number
  width: number
  height: number
}

export interface ScenePoint {
  x: number
  y: number
}

export interface SceneCamera {
  zoom: number
  pan: ScenePoint
}

/** The 1px card border a shell-less page draws outside its body. */
export const CARD_BORDER_WIDTH = 1

/** Screen geometry as it rides a scene entity: the outer rect, plus the inner
 *  body rect for the kinds that have one. */
export interface SceneEntityScreenGeometry {
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  contentScreenX?: number
  contentScreenY?: number
  contentScreenWidth?: number
  contentScreenHeight?: number
}

export function projectToScreen(
  rect: CanvasRect,
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): ScreenRect {
  return {
    x: sceneOrigin.x + rect.x * camera.zoom + camera.pan.x,
    y: sceneOrigin.y + rect.y * camera.zoom + camera.pan.y,
    width: rect.width * camera.zoom,
    height: rect.height * camera.zoom,
  }
}

export function unprojectFromScreen(
  rect: ScreenRect,
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): CanvasRect {
  return {
    x: (rect.x - sceneOrigin.x - camera.pan.x) / camera.zoom,
    y: (rect.y - sceneOrigin.y - camera.pan.y) / camera.zoom,
    width: rect.width / camera.zoom,
    height: rect.height / camera.zoom,
  }
}

export function projectPointToScreen(
  point: ScenePoint,
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): ScenePoint {
  return {
    x: sceneOrigin.x + point.x * camera.zoom + camera.pan.x,
    y: sceneOrigin.y + point.y * camera.zoom + camera.pan.y,
  }
}

export function unprojectPointFromScreen(
  point: ScenePoint,
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): ScenePoint {
  return {
    x: (point.x - sceneOrigin.x - camera.pan.x) / camera.zoom,
    y: (point.y - sceneOrigin.y - camera.pan.y) / camera.zoom,
  }
}

/** Grow a body rect outward into its shell rect. */
export function outsetByShell(
  rect: ScreenRect,
  insets: ShellInsets,
  zoom: number,
): ScreenRect {
  return {
    x: rect.x - insets.left * zoom,
    y: rect.y - insets.top * zoom,
    width: rect.width + (insets.left + insets.right) * zoom,
    height: rect.height + (insets.top + insets.bottom) * zoom,
  }
}

/** Shrink a shell rect inward into its body rect. */
export function insetByShell(
  rect: ScreenRect,
  insets: ShellInsets,
  zoom: number,
): ScreenRect {
  return {
    x: rect.x + insets.left * zoom,
    y: rect.y + insets.top * zoom,
    width: rect.width - (insets.left + insets.right) * zoom,
    height: rect.height - (insets.top + insets.bottom) * zoom,
  }
}

/** The device-shell insets an entity's metadata resolves to, or null when it
 *  draws no shell. `null` deviceId with a shell on is the custom-size case. */
export function shellInsetsFor(entity: {
  showDeviceFrame?: boolean
  deviceId?: string | null
  deviceOrientation?: DeviceOrientation
}): ShellInsets | null {
  if (entity.showDeviceFrame !== true) return null
  if (!entity.deviceId) return CUSTOM_SHELL_INSETS
  return shellInsetsForDevice(entity.deviceId, entity.deviceOrientation ?? 'portrait')
}

/**
 * Pages project to whole pixels because a `WebContentsView` is positioned in
 * window pixels: main rounds once, and every chrome layer that has to sit flush
 * against that view has to round the same way or leave a seam.
 *
 * `sceneOrigin.y` is added after the round, not before, so the toolbar band is
 * a rigid offset rather than something the rounding can absorb.
 */
export function projectPageToScreen(
  page: {
    canvasX: number
    canvasY: number
    width: number
    height: number
    showDeviceFrame?: boolean
    deviceId?: string | null
    deviceOrientation?: DeviceOrientation
  },
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): { shell: ScreenRect; content: ScreenRect } {
  const { zoom, pan } = camera
  const insets = shellInsetsFor(page)
  const contentWidth = Math.round(page.width * zoom)
  const contentHeight = Math.round(page.height * zoom)
  const insetLeft = Math.round((insets?.left ?? 0) * zoom)
  const insetTop = Math.round((insets?.top ?? 0) * zoom)
  const insetRight = Math.round((insets?.right ?? 0) * zoom)
  const insetBottom = Math.round((insets?.bottom ?? 0) * zoom)

  const snapLeft = sceneOrigin.x + Math.round(page.canvasX * zoom + pan.x)
  const snapTop = sceneOrigin.y + Math.round(page.canvasY * zoom + pan.y)
  const content = {
    x: snapLeft + insetLeft,
    y: snapTop + insetTop,
    width: contentWidth,
    height: contentHeight,
  }
  const shell = insets
    ? {
        x: snapLeft,
        y: snapTop,
        width: contentWidth + insetLeft + insetRight,
        height: contentHeight + insetTop + insetBottom,
      }
    : {
        x: content.x - CARD_BORDER_WIDTH,
        y: content.y - CARD_BORDER_WIDTH,
        width: contentWidth + 2 * CARD_BORDER_WIDTH,
        height: contentHeight + 2 * CARD_BORDER_WIDTH,
      }
  return { shell, content }
}

/** A file entity anchors at its body; the shell, when shown, wraps outward. */
export function projectFileToScreen(
  entity: {
    canvasX: number
    canvasY: number
    width: number
    height: number
    showDeviceFrame?: boolean
    deviceId?: string | null
    deviceOrientation?: DeviceOrientation
  },
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): { shell: ScreenRect; content: ScreenRect } {
  const content = projectToScreen(
    { x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height },
    camera,
    sceneOrigin,
  )
  const insets = shellInsetsFor(entity)
  return { shell: insets ? outsetByShell(content, insets, camera.zoom) : content, content }
}

/** Every scene-entity kind's screen geometry, in the shape the entity carries it. */
export function projectSceneEntity(
  entity: {
    kind: string
    canvasX: number
    canvasY: number
    width: number
    height: number
    showDeviceFrame?: boolean
    deviceId?: string | null
    deviceOrientation?: DeviceOrientation
  },
  camera: SceneCamera,
  sceneOrigin: ScenePoint,
): SceneEntityScreenGeometry {
  if (entity.kind === 'page') {
    const { shell, content } = projectPageToScreen(entity, camera, sceneOrigin)
    const outer = entity.showDeviceFrame === true ? shell : content
    return {
      screenX: outer.x,
      screenY: outer.y,
      screenWidth: outer.width,
      screenHeight: outer.height,
      // A page always reports its body rect: the web viewport is what the
      // native view occupies whether or not a bezel is drawn around it.
      contentScreenX: content.x,
      contentScreenY: content.y,
      contentScreenWidth: content.width,
      contentScreenHeight: content.height,
    }
  }
  if (entity.kind === 'file') {
    const { shell, content } = projectFileToScreen(entity, camera, sceneOrigin)
    const shown = entity.showDeviceFrame === true
    return {
      screenX: shell.x,
      screenY: shell.y,
      screenWidth: shell.width,
      screenHeight: shell.height,
      contentScreenX: shown ? content.x : undefined,
      contentScreenY: shown ? content.y : undefined,
      contentScreenWidth: shown ? content.width : undefined,
      contentScreenHeight: shown ? content.height : undefined,
    }
  }
  const rect = projectToScreen(
    { x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height },
    camera,
    sceneOrigin,
  )
  return {
    screenX: rect.x,
    screenY: rect.y,
    screenWidth: rect.width,
    screenHeight: rect.height,
  }
}
