import type { Annotation, LayoutUpdateData } from '../../shared/types'
import {
  annotationContextPageId,
  annotationMatchesPageUrl,
} from '../../shared/annotation-utils'
import { canvasRectToScreenRect } from './annotationMath'

// A region rect drawn over a page refers to that page's document; once the
// page navigates elsewhere the rect would outline unrelated content, so it
// hides until the page returns to the annotation's URL.
function regionPageStillCurrent(annotation: Annotation, layoutData: LayoutUpdateData): boolean {
  const pageId = annotationContextPageId(annotation)
  if (!pageId) return true
  const page = layoutData.entities.find(
    (entity) => entity.kind === 'page' && entity.id === pageId,
  )
  if (!page || page.kind !== 'page') return true
  return annotationMatchesPageUrl(annotation, page.url)
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
    (a) =>
      a.anchor.type === 'region' &&
      a.status !== 'resolved' &&
      a.status !== 'dismissed' &&
      regionPageStillCurrent(a, layoutData),
  )
  if (regionAnnotations.length === 0) return null

  return (
    <>
      {regionAnnotations.map((annotation) => {
        if (annotation.anchor.type !== 'region') return null
        const screen = canvasRectToScreenRect(layoutData, annotation.anchor.canvasRect)

        return (
          <button
            key={annotation.id}
            type="button"
            data-overlay-ui
            data-viewport-passthrough
            aria-label="Open region select annotation"
            className={`${interactive ? 'pointer-events-auto' : 'pointer-events-none'} absolute rounded border-2 border-dashed border-rose-400/70 bg-rose-400/5 opacity-50 hover:bg-rose-400/10 hover:opacity-100`}
            style={{ left: screen.left, top: screen.top - layoutData.canvasOrigin.y, width: screen.width, height: screen.height, cursor: 'pointer' }}
            onClick={() => onOpenThread(annotation.id)}
          />
        )
      })}
    </>
  )
}
