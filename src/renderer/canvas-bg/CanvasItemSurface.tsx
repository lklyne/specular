import { useCallback, useEffect, useLayoutEffect, useMemo, useRef } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { PagePopupAnchor } from '../../shared/page-frames'
import {
  drawItemChrome,
  drawItemSnapshot,
  itemGeometry,
  readChromeColors,
  type ItemGeometry,
} from '../shared/chromeItemDraw'
import { prepareScreenCanvas } from '../shared/screenCanvas'
import type { CanvasItemDraw } from './canvasItemDrawOrder'
import { popupHasClosed, prunePageFrames, usePageFrames, type PageFrameStore } from './usePageFrames'

/**
 * How far an item's paint reaches past its shell rect at zoom 1: the bezel's
 * drop shadow (16px offset plus its blur) and the outset border ring.
 */
const ITEM_PAINT_REACH = 48

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

/** Whether nothing an item paints can land inside a `width` × `height` canvas. */
function itemOffCanvas(g: ItemGeometry, width: number, height: number): boolean {
  const reach = ITEM_PAINT_REACH * g.displayZoom
  return (
    g.shellX + g.shellW + reach < 0 ||
    g.shellY + g.shellH + reach < 0 ||
    g.shellX - reach > width ||
    g.shellY - reach > height
  )
}

/** A page's latest frame in its content rect, then any open popup over it. */
function drawPageFrame(
  ctx: CanvasRenderingContext2D,
  frames: PageFrameStore,
  pageId: string,
  g: ItemGeometry,
  now: number,
): void {
  const frame = frames.frames.get(pageId)
  if (!frame) return // No first frame yet — the border ring already frames the empty interior.
  drawItemSnapshot(ctx, g, frame.bitmap)

  const popup = frames.popups.get(pageId)
  if (!popup) return
  if (popupHasClosed(popup, now)) {
    popup.bitmap.close()
    frames.popups.delete(pageId)
    return
  }
  // Nothing to place it by until the page has answered; a frame later is
  // better than a flash at the page origin.
  if (!popup.anchor) return
  // The popup's own texture carries no CSS size, only device pixels —
  // recover it via the page frame's own device-pixel-to-CSS ratio, then
  // reproject through the same content-rect scale as the page.
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
  ctx.drawImage(
    popup.bitmap,
    g.contentX + at.x * displayZoom,
    g.contentY + at.y * displayZoom,
    size.width * displayZoom,
    size.height * displayZoom,
  )
}

/**
 * canvas-bg's item pass (ADR 0038): every page's and device-framed file's
 * shell, border, and live texture on one canvas below aboveView. Each item
 * paints whole before the next, in scene order, so a page's bezel and content
 * share its place in the stack.
 *
 * A change to the items or theme repaints before the browser paints, keeping
 * the pass in step with the DOM layers around it. Page frames and resizes
 * request one paint on the next animation frame. One page's frame repaints
 * the whole surface, measured fine at lab page counts (ADR 0038).
 */
export function CanvasItemSurface({
  api,
  draws,
  isDark,
}: {
  api: Pick<CanvasBgElectronAPI, 'pagePopupAnchor' | 'requestPageFrames'>
  draws: CanvasItemDraw[]
  isDark: boolean
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const scheduledPaint = useRef(0)
  const paintRef = useRef(() => {})
  const presentedPageIds = useRef(new Set<string>())
  const inputsRef = useRef({ draws, isDark })
  inputsRef.current = { draws, isDark }

  const requestPaint = useCallback(() => {
    if (scheduledPaint.current) return
    scheduledPaint.current = requestAnimationFrame(() => {
      scheduledPaint.current = 0
      paintRef.current()
    })
  }, [])

  const frames = usePageFrames(requestPaint, (pageId) => api.pagePopupAnchor(pageId).catch(() => null))

  const paint = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const prepared = prepareScreenCanvas(canvas, window.devicePixelRatio || 1)
    if (!prepared) return
    const { ctx, width, height, dpr } = prepared
    const { draws: items, isDark: dark } = inputsRef.current
    const { borderColor, bezelColor } = readChromeColors(canvas)
    const now = performance.now()
    for (const draw of items) {
      const g = itemGeometry(draw.item)
      if (itemOffCanvas(g, width, height)) continue
      if (draw.chrome) drawItemChrome(ctx, draw.item, g, dark, bezelColor, borderColor, dpr)
      if (draw.pageId) drawPageFrame(ctx, frames, draw.pageId, g, now)
    }
  }, [frames])
  paintRef.current = paint

  // A pan moves every item but changes no page's membership, so the prune
  // below keys on the ids rather than on each new draw list.
  const pageIdsKey = useMemo(
    () => draws.flatMap((draw) => (draw.pageId ? [draw.pageId] : [])).join('\n'),
    [draws],
  )

  // A page that leaves the scene stops arriving, so nothing else would ever
  // close its bitmap.
  useEffect(() => {
    const pageIds = new Set(pageIdsKey ? pageIdsKey.split('\n') : [])
    prunePageFrames(frames, pageIds)
    // The listener is installed by usePageFrames above. A static page may
    // have sent its only frame before mount or before it entered the scene.
    const added = [...pageIds].filter((id) => !presentedPageIds.current.has(id))
    presentedPageIds.current = pageIds
    if (added.length) api.requestPageFrames(added)
  }, [api, frames, pageIdsKey])

  useEffect(() => () => {
    presentedPageIds.current.clear()
    prunePageFrames(frames, new Set())
  }, [frames])

  useLayoutEffect(() => {
    // This paint already shows every frame that has arrived.
    cancelAnimationFrame(scheduledPaint.current)
    scheduledPaint.current = 0
    paint()
  }, [draws, isDark, paint])

  // Theme tokens can land after this layout effect; repaint once they have.
  useEffect(() => {
    requestPaint()
  }, [isDark, requestPaint])

  useEffect(() => {
    window.addEventListener('resize', requestPaint)
    return () => {
      window.removeEventListener('resize', requestPaint)
      cancelAnimationFrame(scheduledPaint.current)
      scheduledPaint.current = 0
    }
  }, [requestPaint])

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      data-canvas-item-surface
      className="pointer-events-none absolute inset-0 h-full w-full"
    />
  )
}
