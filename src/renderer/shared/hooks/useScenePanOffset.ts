import { useEffect, useState } from 'react'
import type { ViewportNudge } from '../../../shared/types'

const ZERO = { x: 0, y: 0 }

/**
 * Live screen-space offset to apply to the canvas scene so it tracks a pan
 * before the debounced `layout-update` catches up. Main pushes a viewport
 * nudge on every pan/zoom; the offset is `livePan − payloadPan`, which
 * self-reconciles to zero as soon as a fresh layout-update lands with the
 * matching pan.
 *
 * Pan-only: a zoom change rescales the scene rather than translating it, so
 * when the nudge's zoom differs from the payload we yield no offset and let the
 * next layout-update reposition. Zoom gestures are comparatively rare and the
 * payload lands within a couple of frames. See #257.
 */
export function useScenePanOffset(
  onViewportNudge: (cb: (nudge: ViewportNudge) => void) => () => void,
  payload: { pan: { x: number; y: number }; zoom: number },
): { x: number; y: number } {
  const [nudge, setNudge] = useState<ViewportNudge | null>(null)
  useEffect(() => onViewportNudge(setNudge), [onViewportNudge])

  if (!nudge || nudge.zoom !== payload.zoom) return ZERO
  const x = nudge.pan.x - payload.pan.x
  const y = nudge.pan.y - payload.pan.y
  if (x === 0 && y === 0) return ZERO
  return { x, y }
}
