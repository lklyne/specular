import { useEffect, useRef } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { PagePopupAnchor } from '../../shared/page-frames'
import type { ProjectedPageEntity } from '../../shared/scene-projection'
import type { FocusPresentationMode } from '../../shared/types'
import { drawItemSnapshot, itemGeometry, pageChromeItem } from '../shared/chromeItemDraw'
import { prepareScreenCanvas } from '../shared/screenCanvas'
import { popupHasClosed, prunePageFrames, usePageFrames } from './usePageFrames'

/**
 * Where a popup of `size` sits relative to the element that opened it, in
 * page CSS px: below and left-aligned, the way Chromium places its own
 * pickers, flipped above when the viewport has no room below, and slid left
 * to stay inside the viewport.
 */
export function placePopup(
  anchor: PagePopupAnchor,
  size: { width: number; height: number },
  viewport: { width: number; height: number },
): { x: number; y: number } {
  let y = anchor.y + anchor.height
  if (y + size.height > viewport.height && anchor.y - size.height >= 0) y = anchor.y - size.height
  const x = Math.max(0, Math.min(anchor.x, viewport.width - size.width))
  return { x, y }
}

/**
 * canvas-bg's texture pass: every presented page's latest painted frame,
 * scaled into the content rect the chrome pass already framed (ADR 0038).
 * Mounted after `ChromeCanvasSurface` — this is where native page views used
 * to sit in the window's stacking order, above borders and shells and below
 * aboveView.
 *
 * Draw model mirrors the offscreen-rendering lab (`LabCanvas.tsx`): a dirty
 * flag set on frame arrival or a prop/theme/resize change, drained by one rAF
 * loop that redraws every page when set. A single page's frame redraws the
 * whole surface — measured fine at lab page counts, see ADR 0038.
 */
export function PageTextureSurface({
  api,
  texturePages,
  focusPageId,
  focusMode,
  isDark,
}: {
  api: Pick<CanvasBgElectronAPI, 'pagePopupAnchor'>
  texturePages: ProjectedPageEntity[]
  focusPageId: string | null
  focusMode: FocusPresentationMode | null
  isDark: boolean
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const dirtyRef = useRef(true)
  const drawInputs = useRef({ texturePages, focusPageId, focusMode })
  drawInputs.current = { texturePages, focusPageId, focusMode }

  const frames = usePageFrames(
    () => {
      dirtyRef.current = true
    },
    (pageId) => api.pagePopupAnchor(pageId).catch(() => null),
  )

  // A page that leaves the scene stops arriving, so nothing else would ever
  // close its bitmap.
  useEffect(() => {
    prunePageFrames(frames, new Set(texturePages.map((page) => page.id)))
  }, [frames, texturePages])

  useEffect(() => {
    dirtyRef.current = true
  }, [texturePages, focusPageId, focusMode, isDark])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    let raf = 0

    const draw = () => {
      raf = requestAnimationFrame(draw)
      if (!dirtyRef.current) return
      dirtyRef.current = false
      const prepared = prepareScreenCanvas(canvas, window.devicePixelRatio || 1)
      if (!prepared) return
      const { ctx: c } = prepared
      const { texturePages: pages, focusPageId: fillPageId, focusMode: mode } = drawInputs.current
      const now = performance.now()

      for (const page of pages) {
        const frame = frames.frames.get(page.id)
        if (!frame) continue // No first frame yet — the border ring already frames the empty interior.

        const isFill = mode === 'fill' && page.id === fillPageId
        const item = isFill ? pageChromeItem(page, { showDeviceFrame: false }) : pageChromeItem(page)
        const g = itemGeometry(item)
        drawItemSnapshot(c, g, frame.bitmap)

        const popup = frames.popups.get(page.id)
        if (!popup) continue
        if (popupHasClosed(popup, now)) {
          popup.bitmap.close()
          frames.popups.delete(page.id)
          continue
        }
        // Nothing to place it by until the page has answered; a frame later is
        // better than a flash at the page origin.
        if (!popup.anchor) continue
        // The popup's own texture carries no CSS size, only device pixels —
        // recover it via the page frame's own device-pixel-to-CSS ratio,
        // then reproject through the same content-rect scale as the page.
        const pageDeviceScale = frame.meta.width / frame.meta.cssWidth
        const displayZoom = g.contentW / frame.meta.cssWidth
        const size = {
          width: popup.meta.width / pageDeviceScale,
          height: popup.meta.height / pageDeviceScale,
        }
        const at = placePopup(popup.anchor, size, {
          width: frame.meta.cssWidth,
          height: frame.meta.cssHeight,
        })
        c.drawImage(
          popup.bitmap,
          g.contentX + at.x * displayZoom,
          g.contentY + at.y * displayZoom,
          size.width * displayZoom,
          size.height * displayZoom,
        )
      }
    }
    raf = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(raf)
  }, [frames])

  useEffect(() => {
    const onResize = () => {
      dirtyRef.current = true
    }
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      data-page-texture-canvas
      className="pointer-events-none absolute inset-0 h-full w-full"
    />
  )
}
