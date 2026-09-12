import { useCallback, useEffect, useRef } from 'react'
import { drawChromeCanvas, type ChromeCanvasItem } from './chromeCanvasDraw'

/**
 * Full-window canvas that renders page borders and device shells at the
 * screen geometry this renderer projected. Drawing rather than transforming
 * DOM is what keeps strokes from being bitmap-scaled during a zoom gesture.
 */
export function ChromeCanvasSurface({
  pages,
  fileEntities,
  isDark,
}: {
  pages: ChromeCanvasItem[]
  fileEntities: ChromeCanvasItem[]
  isDark: boolean
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  // Latest draw inputs, read by the resize-triggered redraw without re-binding
  // the listener every tick (same pattern as CanvasGridSurface, #265).
  const drawInputs = useRef({ pages, fileEntities, isDark })
  drawInputs.current = { pages, fileEntities, isDark }

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
  }, [pages, fileEntities, isDark, draw])

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
