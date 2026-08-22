import { useCallback, useEffect, useRef } from 'react'
import type { SceneCameraTransform } from '../../shared/scene-camera-transform'
import { drawChromeCanvas, type ChromeCanvasItem } from './chromeCanvasDraw'
import type { FrozenPageBitmaps } from '../shared/useFrozenPageBitmaps'

/**
 * Full-window canvas that renders page borders and device shells in screen
 * space from the live camera on every tick. Sits above the CSS-transformed
 * chrome container so strokes never get bitmap-scaled during a zoom gesture.
 */
export function ChromeCanvasSurface({
  pages,
  fileEntities,
  snapshots,
  transform,
  isDark,
  dragFrozenPageIds,
}: {
  pages: ChromeCanvasItem[]
  fileEntities: ChromeCanvasItem[]
  snapshots: FrozenPageBitmaps
  transform: SceneCameraTransform
  isDark: boolean
  /** Pages above-view is drawing for a drag freeze; skipped here so no page
   *  is drawn twice. */
  dragFrozenPageIds?: ReadonlySet<string>
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  // Latest draw inputs, read by the resize-triggered redraw without re-binding
  // the listener every tick (same pattern as CanvasGridSurface, #265).
  const drawInputs = useRef({ pages, fileEntities, snapshots, transform, isDark, dragFrozenPageIds })
  drawInputs.current = { pages, fileEntities, snapshots, transform, isDark, dragFrozenPageIds }

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
  }, [pages, fileEntities, snapshots, transform, isDark, dragFrozenPageIds, draw])

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
