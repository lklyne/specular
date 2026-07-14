import type {
  Annotation,
  CanvasScenePageEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { pageDocumentToScreen } from '../../shared/page-space'
import { canvasRectToScreenRect } from './annotationMath'

// How far (screen px) a page-anchored region may travel outside its page's
// content band before it fully fades — mirrors CommentBadgesLayer's
// BADGE_FADE_MARGIN. Keeps a scrolled-out region from painting over the page
// chrome above the frame or floating across the canvas after scroll.
const REGION_FADE_MARGIN = 48

interface RegionGeometry {
  left: number
  top: number
  width: number
  height: number
  /** Edge fade as a page-anchored region leaves its page's content band;
   *  always 1 for canvas-anchored regions. */
  opacity: number
}

/**
 * Screen geometry for a region annotation, in overlay coordinates (y already
 * offset by the canvas origin). Canvas-anchored regions (`canvasRect`) map
 * through the canvas transform; page-anchored regions (`docRect`) map through
 * `pageDocumentToScreen` against their page so they scroll-follow, and fade at
 * the content-frame edges. Null when a page-anchored region's page is absent
 * from the scene (defensive — off-URL regions are already dropped main-side).
 */
function regionScreenGeometry(
  annotation: Annotation,
  layoutData: LayoutUpdateData,
): RegionGeometry | null {
  const anchor = annotation.anchor
  if (anchor.type !== 'region') return null
  if (!('docRect' in anchor)) {
    const screen = canvasRectToScreenRect(layoutData, anchor.canvasRect)
    return {
      left: screen.left,
      top: screen.top - layoutData.canvasOrigin.y,
      width: screen.width,
      height: screen.height,
      opacity: 1,
    }
  }
  const pageId = annotation.pageAnchor?.pageId
  const page = pageId
    ? layoutData.entities.find(
        (entity): entity is CanvasScenePageEntity =>
          entity.kind === 'page' && entity.id === pageId,
      )
    : undefined
  if (!page) return null
  const rect = pageDocumentToScreen(anchor.docRect, page, layoutData)
  const opacity = regionContentOpacity(rect.top, page, layoutData)
  if (opacity <= 0) return null
  return { left: rect.left, top: rect.top, width: rect.width, height: rect.height, opacity }
}

// Fade a page-anchored region as its top edge leaves the page's vertical
// content band. Models CommentBadgesLayer's `badgeOpacity`: 1 inside the band,
// ramps to 0 across REGION_FADE_MARGIN of overflow, 0 beyond. `anchorTop` is
// in overlay coordinates (already offset by the canvas origin).
function regionContentOpacity(
  anchorTop: number,
  page: CanvasScenePageEntity,
  layoutData: LayoutUpdateData,
): number {
  const contentScreenY = page.contentScreenY ?? page.screenY
  const contentScreenHeight = page.contentScreenHeight ?? page.screenHeight
  const top = contentScreenY - layoutData.canvasOrigin.y
  const bottom = top + contentScreenHeight
  const overflow = Math.max(top - anchorTop, anchorTop - bottom, 0)
  if (overflow <= 0) return 1
  return Math.max(0, 1 - overflow / REGION_FADE_MARGIN)
}

export function RegionSelectAnnotations({
  annotations,
  interactive,
  layoutData,
  onOpenThread,
}: {
  annotations: Annotation[]
  interactive: boolean
  layoutData: LayoutUpdateData
  onOpenThread: (annotationId: string) => void
}) {
  const regionAnnotations = annotations.filter(
    (a) => a.anchor.type === 'region' && a.status !== 'resolved' && a.status !== 'dismissed',
  )
  if (regionAnnotations.length === 0) return null

  return (
    <>
      {regionAnnotations.map((annotation) => {
        const geom = regionScreenGeometry(annotation, layoutData)
        if (!geom) return null

        return (
          <button
            key={annotation.id}
            type="button"
            data-overlay-ui
            data-viewport-passthrough
            aria-label="Open region select annotation"
            className={`${interactive ? 'pointer-events-auto' : 'pointer-events-none'} absolute rounded border-2 border-dashed border-rose-400/70 bg-rose-400/5 opacity-50 hover:bg-rose-400/10 hover:opacity-100`}
            style={{
              left: geom.left,
              top: geom.top,
              width: geom.width,
              height: geom.height,
              cursor: 'pointer',
              // Only override the class-based opacity while a page-anchored
              // region is fading out; a fully-visible region keeps the
              // opacity-50 / hover:opacity-100 affordance.
              ...(geom.opacity < 1 ? { opacity: geom.opacity } : {}),
            }}
            onClick={() => onOpenThread(annotation.id)}
          />
        )
      })}
    </>
  )
}
