import { useEffect, useState } from 'react'
import type { ViewportNudge } from '../../../shared/types'

const IDENTITY = { x: 0, y: 0, scale: 1 }

/**
 * Live screen-space transform to apply to the canvas scene so it tracks a
 * pan or zoom gesture before the debounced `layout-update` catches up. Main
 * pushes a viewport nudge on every pan/zoom; the scene container is
 * positioned at `payload.{pan,zoom}`, but the live camera is at
 * `nudge.{pan,zoom}`. Mapping payload-screen coords to live-screen coords
 * gives a translate+scale: with `s = nudge.zoom / payload.zoom`,
 * `x = nudge.pan.x - s*payload.pan.x`, `y = nudge.pan.y - s*payload.pan.y`.
 * This self-reconciles to identity as soon as a fresh layout-update lands
 * with the matching pan/zoom.
 */
export function useSceneCameraTransform(
  onViewportNudge: (cb: (nudge: ViewportNudge) => void) => () => void,
  payload: { pan: { x: number; y: number }; zoom: number },
): { x: number; y: number; scale: number } {
  const [nudge, setNudge] = useState<ViewportNudge | null>(null)
  useEffect(() => onViewportNudge(setNudge), [onViewportNudge])

  if (!nudge || payload.zoom <= 0) return IDENTITY
  const scale = nudge.zoom / payload.zoom
  const x = nudge.pan.x - scale * payload.pan.x
  const y = nudge.pan.y - scale * payload.pan.y
  if (x === 0 && y === 0 && scale === 1) return IDENTITY
  return { x, y, scale }
}
