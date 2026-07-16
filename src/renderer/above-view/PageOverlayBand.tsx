import type { ReactNode } from 'react'
import type { CanvasScenePageEntity, LayoutUpdateData } from '../../shared/types'

// How far (screen px) a page-anchored overlay may travel past the page's
// content band before it is fully clipped. The band's gradient mask fades
// exactly this strip, so an overlay straddling the band edge fades only the
// part that has left the page instead of the whole element at once.
export const BAND_FADE_MARGIN = 48

/**
 * Per-page clipping container for page-anchored overlays (region rects,
 * comment badges, element highlights). Spans the page's content band plus a
 * fade margin above and below: `overflow: hidden` clips paint and hit-testing
 * at the margin, and a vertical gradient mask fades the margin strips.
 * Horizontal edges clip hard — scroll is vertical, so nothing fades sideways.
 *
 * Children keep their overlay coordinates unchanged (window-space x, y offset
 * by the canvas origin) — an inner wrapper cancels the band's own origin.
 */
export function PageOverlayBand({
  page,
  layoutData,
  zIndex,
  children,
}: {
  page: CanvasScenePageEntity
  layoutData: LayoutUpdateData
  zIndex?: number
  children: ReactNode
}) {
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
      <div className="absolute" style={{ left: -left, top: -top }}>
        {children}
      </div>
    </div>
  )
}
