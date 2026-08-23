import type { ProjectedLayoutData, ProjectedPageEntity } from '../../shared/scene-projection'
import { memo } from 'react'
import type { Annotation } from '../../shared/types'
import { pageDocumentToScreen } from '../../shared/page-space'
import { correctDocRectForElement } from '../../shared/element-attachment'
import { shouldFastFollowPageScroll } from '../../shared/page-anchor'
import { canvasRectToScreenRect } from './annotationMath'
import { PageOverlayBand } from './PageOverlayBand'

interface RegionGeometry {
  left: number
  top: number
  width: number
  height: number
  /** Set for page-anchored (`docRect`) regions — the region renders inside
   *  that page's overlay band, which clips and edge-fades it. */
  pageId?: string
}

/**
 * Screen geometry for a region annotation, in overlay coordinates (y already
 * offset by the canvas origin). Canvas-anchored regions (`canvasRect`) map
 * through the canvas transform; page-anchored regions (`docRect`) map through
 * `pageDocumentToScreen` against their page so they scroll-follow. Null when
 * a page-anchored region's page is absent from the scene (defensive —
 * off-URL regions are already dropped main-side).
 */
function regionScreenGeometry(
  annotation: Annotation,
  layoutData: ProjectedLayoutData,
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
    }
  }
  const pageId = annotation.pageAnchor?.pageId
  const page = pageId
    ? layoutData.entities.find(
        (entity): entity is ProjectedPageEntity =>
          entity.kind === 'page' && entity.id === pageId,
      )
    : undefined
  if (!page) return null
  // Element-follow (ADR 0032): shift the docRect by how far its reference
  // element has moved before mapping to screen, so the region tracks page
  // content through reflow — the same correction main applies in regionCanvasRect.
  const docRect = correctDocRectForElement(
    anchor.docRect,
    annotation.pageAnchor?.element,
    page.elementPositions,
  )
  const rect = pageDocumentToScreen(docRect, page, layoutData)
  return { left: rect.left, top: rect.top, width: rect.width, height: rect.height, pageId }
}

export const RegionSelectAnnotations = memo(function RegionSelectAnnotations({
  annotations,
  interactive,
  layoutData,
  onOpenThread,
}: {
  annotations: Annotation[]
  interactive: boolean
  layoutData: ProjectedLayoutData
  onOpenThread: (annotationId: string) => void
}) {
  const regionAnnotations = annotations.filter(
    (a) => a.anchor.type === 'region' && a.status !== 'resolved' && a.status !== 'dismissed',
  )
  if (regionAnnotations.length === 0) return null

  const renderRegion = (annotation: Annotation, geom: RegionGeometry) => (
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
      }}
      onClick={() => onOpenThread(annotation.id)}
    />
  )

  const placed = regionAnnotations
    .map((annotation) => ({ annotation, geom: regionScreenGeometry(annotation, layoutData) }))
    .filter((entry): entry is { annotation: Annotation; geom: RegionGeometry } => entry.geom !== null)

  const byPage = new Map<
    string,
    { pageId: string; followScroll: boolean; entries: typeof placed }
  >()
  for (const entry of placed) {
    if (!entry.geom.pageId) continue
    const anchor = entry.annotation.pageAnchor
    const page = layoutData.entities.find(
      (entity): entity is ProjectedPageEntity =>
        entity.kind === 'page' && entity.id === entry.geom.pageId,
    )
    const liveElement = anchor?.element
      ? page?.elementPositions?.[anchor.element.selector]
      : undefined
    const followScroll = shouldFastFollowPageScroll(anchor, liveElement)
    const key = `${entry.geom.pageId}:${followScroll ? 'document' : 'viewport'}`
    const group = byPage.get(key)
    if (group) group.entries.push(entry)
    else {
      byPage.set(key, {
        pageId: entry.geom.pageId,
        followScroll,
        entries: [entry],
      })
    }
  }

  return (
    <>
      {placed
        .filter((entry) => !entry.geom.pageId)
        .map((entry) => renderRegion(entry.annotation, entry.geom))}
      {[...byPage.entries()].map(([key, group]) => {
        const page = layoutData.entities.find(
          (entity): entity is ProjectedPageEntity =>
            entity.kind === 'page' && entity.id === group.pageId,
        )
        if (!page) return null
        return (
          <PageOverlayBand
            key={key}
            page={page}
            layoutData={layoutData}
            followScroll={group.followScroll}
          >
            {group.entries.map((entry) => renderRegion(entry.annotation, entry.geom))}
          </PageOverlayBand>
        )
      })}
    </>
  )
})
