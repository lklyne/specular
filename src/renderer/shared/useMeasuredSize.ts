/**
 * Measure a DOM element's content box, rAF-throttled.
 *
 * The measurement primitive behind content-sized entities (see CONTEXT.md,
 * "Content-sized bounds"): main owns entity bounds, but the size of a text
 * body is only knowable after layout, so the renderer measures and reports it.
 * This hook is the measuring half only — rounding, clamping, grid snapping,
 * and the report back to main are policy, and belong to the caller.
 *
 * Returns null until the first measurement lands.
 */

import { useEffect, useState } from 'react'

export interface MeasuredSize {
  width: number
  height: number
}

export function useMeasuredSize(
  ref: React.MutableRefObject<HTMLElement | null>,
  enabled: boolean,
): MeasuredSize | null {
  const [size, setSize] = useState<MeasuredSize | null>(null)
  useEffect(() => {
    const el = ref.current
    if (!enabled || !el) return
    let pendingFrame = 0
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return
      // Coalesce to one update per frame: a resize gesture fires the observer
      // far more often than the layout can usefully change.
      if (pendingFrame) cancelAnimationFrame(pendingFrame)
      pendingFrame = requestAnimationFrame(() => {
        setSize({ width: entry.contentRect.width, height: entry.contentRect.height })
      })
    })
    observer.observe(el)
    return () => {
      observer.disconnect()
      if (pendingFrame) cancelAnimationFrame(pendingFrame)
    }
  }, [enabled, ref])
  return enabled ? size : null
}
