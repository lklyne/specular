import { useEffect, useState } from 'react'
import type { ViewportNudge } from '../../../shared/types'

export type Camera = { pan: { x: number; y: number }; zoom: number }

/**
 * The live camera pushed by main on every pan/zoom, ahead of the debounced
 * `layout-update`. Falls back to the payload's camera until the first nudge and
 * whenever main and the payload agree. Under ADR 0023 Phase 1 pan/zoom no longer
 * rebuild the scene, so this is the only per-frame viewport signal the renderers
 * get — they reproject their scene transform from it (see `sceneReprojectTransform`).
 */
export function useSceneCamera(
  onViewportNudge: (cb: (nudge: ViewportNudge) => void) => () => void,
  payload: Camera,
): Camera {
  const [nudge, setNudge] = useState<ViewportNudge | null>(null)
  useEffect(() => onViewportNudge(setNudge), [onViewportNudge])
  return nudge ?? { pan: payload.pan, zoom: payload.zoom }
}

/**
 * CSS transform that reprojects a scene container whose children were positioned
 * for `payload`'s camera so they appear at `live`'s camera instead. A child at
 * container-local position `origin + canvas*payload.zoom + payload.pan` lands at
 * `origin + canvas*live.zoom + live.pan` — exact for both pan and zoom.
 *
 * Under ADR 0023 Phase 2 the payload no longer carries screen coords: each layer
 * projects its canvas coords through the *payload* camera (`canvasToScreenX(layout,
 * …)`) to position within this container, and this reproject maps that to the live
 * camera. Hit-testing instead reads the live camera directly, so interaction stays
 * exact during a gesture even though the container render trails to the next payload.
 *
 * `originLocalY` is the canvas origin's y in the container's own top-left frame:
 * `canvasOrigin.y` for bgView (no inset), `0` for the above-view / cursor
 * overlays (their WCV top sits at `canvasOrigin.y`, which their children already
 * subtract). The container must set `transform-origin: 0 0`.
 */
export function sceneReprojectTransform(
  payload: { pan: { x: number; y: number }; zoom: number; canvasOrigin: { x: number; y: number } },
  live: Camera,
  originLocalY: number,
): string {
  const scale = live.zoom / payload.zoom
  const ox = payload.canvasOrigin.x
  const tx = ox + live.pan.x - scale * (ox + payload.pan.x)
  const ty = originLocalY + live.pan.y - scale * (originLocalY + payload.pan.y)
  return `translate(${tx}px, ${ty}px) scale(${scale})`
}
