import { useLayoutEffect, useRef, type ReactNode } from 'react'
import type { CanvasScenePageEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import {
  scrollFollowTransform,
  type PageScrollOffset,
} from './page-overlay-scroll-follow'

// Resolved lazily — this module is imported by files that node-env unit
// tests load, where `window` doesn't exist.
function electronApi(): CanvasBgElectronAPI {
  return (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI
}

// How far (screen px) a page-anchored overlay may travel past the page's
// content band before it is fully clipped. The band's gradient mask fades
// exactly this strip, so an overlay straddling the band edge fades only the
// part that has left the page instead of the whole element at once.
export const BAND_FADE_MARGIN = 48

/**
 * Per-page clipping container for page-anchored overlays (region rects,
 * comment badges, element highlights, anchored shapes). Spans the page's
 * content band plus a fade margin above and below: `overflow: hidden` clips
 * paint and hit-testing at the margin, and a vertical gradient mask fades the
 * margin strips. Horizontal edges clip hard — scroll is vertical, so nothing
 * fades sideways.
 *
 * Children keep their overlay coordinates unchanged (window-space x, y offset
 * by the canvas origin) — an inner wrapper cancels the band's own origin.
 *
 * `followScroll` is for children positioned in *document* space from the
 * broadcast scroll offset (region rects, anchored shapes): the band listens
 * to the fast-path scroll channel and shifts the inner wrapper by the delta
 * between the live offset and the broadcast one, so the content tracks the
 * page's native compositor scroll instead of lagging the debounced layout
 * rebuild. Each broadcast resets the baseline. Frame-pinned children
 * (page-offset badges) must not opt in — they don't move with scroll.
 */
export function PageOverlayBand({
  page,
  layoutData,
  zIndex,
  followScroll,
  children,
}: {
  page: CanvasScenePageEntity
  layoutData: LayoutUpdateData
  zIndex?: number
  followScroll?: boolean
  children: ReactNode
}) {
  const innerRef = useRef<HTMLDivElement>(null)
  const liveOffsetRef = useRef<PageScrollOffset | null>(null)
  const incorporatedOffsetRef = useRef<PageScrollOffset>({
    pageId: page.id,
    scrollX: page.scrollX ?? 0,
    scrollY: page.scrollY ?? 0,
  })
  const scaleRef = useRef({ x: 1, y: 1 })

  const applyResidual = (live: PageScrollOffset | null) => {
    if (innerRef.current) {
      innerRef.current.style.transform = scrollFollowTransform(
        live,
        incorporatedOffsetRef.current,
        scaleRef.current,
      )
    }
  }

  // Reconcile a fresh authoritative layout before the browser paints it.
  // Children have already rendered from page.scrollX/Y at this point, so the
  // only valid transform is latest-live minus that incorporated baseline.
  useLayoutEffect(() => {
    const scaleX = page.width > 0 ? (page.contentScreenWidth ?? page.screenWidth) / page.width : 1
    const scaleY =
      page.height > 0 ? (page.contentScreenHeight ?? page.screenHeight) / page.height : 1
    incorporatedOffsetRef.current = {
      pageId: page.id,
      scrollX: page.scrollX ?? 0,
      scrollY: page.scrollY ?? 0,
    }
    scaleRef.current = { x: scaleX, y: scaleY }
    applyResidual(followScroll ? liveOffsetRef.current : null)
  }, [
    followScroll,
    page.id,
    page.scrollX,
    page.scrollY,
    page.width,
    page.height,
    page.contentScreenWidth,
    page.contentScreenHeight,
    page.screenWidth,
    page.screenHeight,
  ])

  // The live-scroll subscription belongs to the page identity, not the
  // freshly rebuilt scene object. This avoids dropping events while every
  // authoritative layout unsubscribes and resubscribes the listener.
  useLayoutEffect(() => {
    liveOffsetRef.current = null
    applyResidual(null)
    if (!followScroll) return
    const apply = (live: PageScrollOffset) => {
      if (live.pageId !== incorporatedOffsetRef.current.pageId) return
      liveOffsetRef.current = live
      applyResidual(live)
    }
    return electronApi().onPageScrollLive(apply)
  }, [followScroll, page.id])

  // Horizontal extent is the outer page bounds, not the content band: badges
  // pin to the page's right edge, which a device shell insets out of the
  // content band. Vertical extent stays content-based — that's what scroll
  // moves anchors across.
  const left = page.screenX
  const width = page.screenWidth
  const contentTop = (page.contentScreenY ?? page.screenY) - layoutData.canvasOrigin.y
  const top = contentTop - BAND_FADE_MARGIN
  const height = (page.contentScreenHeight ?? page.screenHeight) + BAND_FADE_MARGIN * 2
  const mask = `linear-gradient(to bottom, transparent, black ${BAND_FADE_MARGIN}px, black calc(100% - ${BAND_FADE_MARGIN}px), transparent)`
  return (
    <div
      className="pointer-events-none absolute overflow-hidden"
      style={{ left, top, width, height, zIndex, maskImage: mask, WebkitMaskImage: mask }}
    >
      <div ref={innerRef} className="absolute" style={{ left: -left, top: -top }}>
        {children}
      </div>
    </div>
  )
}
