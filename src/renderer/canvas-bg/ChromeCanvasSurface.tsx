import { useCallback, useEffect, useRef } from 'react'
import { drawChromeCanvas, type ChromeCanvasItem } from './chromeCanvasDraw'
import type { FrozenPageBitmaps } from '../shared/useFrozenPageBitmaps'

/**
 * Full-window canvas that renders page borders and device shells at the
 * screen geometry this renderer projected. Drawing rather than transforming
 * DOM is what keeps strokes from being bitmap-scaled during a zoom gesture.
 */
export function ChromeCanvasSurface({
  pages,
  fileEntities,
  snapshots,
  isDark,
  dragFrozenPageIds,
}: {
  pages: ChromeCanvasItem[]
  fileEntities: ChromeCanvasItem[]
  snapshots: FrozenPageBitmaps
  isDark: boolean
  /** Pages above-view is drawing for a drag freeze; skipped here so no page
   *  is drawn twice. */
  dragFrozenPageIds?: ReadonlySet<string>
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  // Latest draw inputs, read by the resize-triggered redraw without re-binding
  // the listener every tick (same pattern as CanvasGridSurface, #265).
  const drawInputs = useRef({ pages, fileEntities, snapshots, isDark, dragFrozenPageIds })
  drawInputs.current = { pages, fileEntities, snapshots, isDark, dragFrozenPageIds }

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    drawChromeCanvas({
      canvas,
      ...drawInputs.current,
      devicePixelRatio: window.devicePixelRatio || 1,
    })
  }, [])

  useEffect(() => {
    draw()
  }, [pages, fileEntities, snapshots, isDark, dragFrozenPageIds, draw])

  useEffect(() => {
    window.addEventListener('resize', draw)
    return () => window.removeEventListener('resize', draw)
  }, [draw])

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      data-chrome-canvas
      className="pointer-events-none absolute inset-0 h-full w-full"
    />
  )
}
