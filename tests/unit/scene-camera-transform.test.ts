import { describe, expect, it } from 'vitest'
import {
  cameraAfterSceneTransform,
  computeSceneCameraTransform,
  IDENTITY_SCENE_CAMERA_TRANSFORM,
} from '../../src/shared/scene-camera-transform'

describe('scene camera transform', () => {
  it('returns identity when there is no live viewport nudge', () => {
    expect(
      computeSceneCameraTransform(
        { pan: { x: 10, y: 20 }, zoom: 1 },
        null,
        { x: 0, y: 80 },
      ),
    ).toBe(IDENTITY_SCENE_CAMERA_TRANSFORM)
  })

  it('keeps a fixed canvas origin fixed while zooming the scene', () => {
    const payload = { pan: { x: 10, y: 10 }, zoom: 1 }
    const nudge = { pan: { x: 10, y: 10 }, zoom: 2 }
    const origin = { x: 0, y: 80 }
    const transform = computeSceneCameraTransform(payload, nudge, origin)

    const payloadScreenY = origin.y + payload.pan.y + 100 * payload.zoom
    const transformedScreenY =
      transform.y + transform.scale * payloadScreenY
    const expectedLiveScreenY = origin.y + nudge.pan.y + 100 * nudge.zoom

    expect(transform).toEqual({ x: -10, y: -90, scale: 2 })
    expect(transformedScreenY).toBe(expectedLiveScreenY)
  })

  it('recovers the exact live camera used to draw an untransformed grid', () => {
    const payload = { pan: { x: -42, y: 27 }, zoom: 0.8 }
    const nudge = { pan: { x: 113, y: -19 }, zoom: 1.35 }
    const origin = { x: 12, y: 80 }
    const transform = computeSceneCameraTransform(payload, nudge, origin)

    expect(cameraAfterSceneTransform(payload, transform, origin)).toEqual(nudge)
  })
})
