import type { ViewportNudge } from './types'

export interface SceneCamera {
  pan: { x: number; y: number }
  zoom: number
}

export interface SceneCameraTransform {
  x: number
  y: number
  scale: number
}

export const IDENTITY_SCENE_CAMERA_TRANSFORM: SceneCameraTransform = {
  x: 0,
  y: 0,
  scale: 1,
}

/**
 * Maps coordinates rendered for `payload` onto the live nudge camera.
 *
 * A scene coordinate is rendered as:
 *   screen = sceneOrigin + payload.pan + canvas * payload.zoom
 *
 * The origin term must participate in the transform when zoom changes. Leaving
 * it out produces `(scale - 1) * sceneOrigin` drift (most visibly the toolbar
 * height on the y axis in canvas-bg).
 */
export function computeSceneCameraTransform(
  payload: SceneCamera,
  nudge: ViewportNudge | null,
  sceneOrigin: { x: number; y: number },
): SceneCameraTransform {
  if (!nudge || payload.zoom <= 0) return IDENTITY_SCENE_CAMERA_TRANSFORM

  const scale = nudge.zoom / payload.zoom
  const x =
    sceneOrigin.x +
    nudge.pan.x -
    scale * (sceneOrigin.x + payload.pan.x)
  const y =
    sceneOrigin.y +
    nudge.pan.y -
    scale * (sceneOrigin.y + payload.pan.y)

  if (x === 0 && y === 0 && scale === 1) {
    return IDENTITY_SCENE_CAMERA_TRANSFORM
  }
  return { x, y, scale }
}

/** Returns the camera represented by applying a scene transform to a payload. */
export function cameraAfterSceneTransform(
  payload: SceneCamera,
  transform: SceneCameraTransform,
  sceneOrigin: { x: number; y: number },
): SceneCamera {
  return {
    pan: {
      x:
        transform.x +
        transform.scale * (sceneOrigin.x + payload.pan.x) -
        sceneOrigin.x,
      y:
        transform.y +
        transform.scale * (sceneOrigin.y + payload.pan.y) -
        sceneOrigin.y,
    },
    zoom: payload.zoom * transform.scale,
  }
}
