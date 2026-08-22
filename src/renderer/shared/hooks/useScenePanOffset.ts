import { useEffect, useState } from 'react'
import type { ViewportNudge } from '../../../shared/types'
import {
  computeSceneCameraTransform,
  type SceneCameraTransform,
} from '../../../shared/scene-camera-transform'

/**
 * Live screen-space transform to apply to the canvas scene so it tracks a
 * pan or zoom gesture before the debounced `layout-update` catches up. Main
 * pushes a viewport nudge on every pan/zoom; the scene container is
 * positioned at `payload.{pan,zoom}`, but the live camera is at
 * `nudge.{pan,zoom}`. Mapping payload-screen coords to live-screen coords
 * gives a translate+scale. `sceneOrigin` is the fixed canvas inset expressed
 * in this renderer's coordinate system; it must participate in zoom math.
 * This self-reconciles to identity as soon as a fresh layout-update lands
 * with the matching pan/zoom.
 */
export function useSceneCameraTransform(
  onViewportNudge: (cb: (nudge: ViewportNudge) => void) => () => void,
  payload: { pan: { x: number; y: number }; zoom: number },
  sceneOrigin: { x: number; y: number },
): SceneCameraTransform {
  const [nudge, setNudge] = useState<ViewportNudge | null>(null)
  useEffect(() => onViewportNudge(setNudge), [onViewportNudge])
  return computeSceneCameraTransform(payload, nudge, sceneOrigin)
}
