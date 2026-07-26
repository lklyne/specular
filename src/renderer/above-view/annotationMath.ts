import type {
  Annotation,
  AnnotationCreateRequest,
  AnnotationDrawing,
  AnnotationDrawingPoint,
  AnnotationDrawingStroke,
  CanvasScenePageEntity,
  CanvasSceneEntity,
  DevtoolsPanelDomRect,
  LayoutUpdateData,
  WorkspaceBounds,
} from '../../shared/types'
import {
  canvasToScreenX,
  canvasToScreenY,
  toOverlayY,
} from '../../shared/gesture-utils'
import { pageDocumentToScreen, pageViewportToScreen } from '../../shared/page-space'
import { correctDocRectForElement } from '../../shared/element-attachment'
import { selectionBbox } from '../../shared/selection-bbox'


export interface PendingAnnotation {
  /** Stable id for this draft, used to subscribe live element bbox updates
   *  while the composer is open (ADR 0006). */
  draftId: string
  request: AnnotationCreateRequest
  composerX: number
  composerY: number
  composerWidth: number
}

/**
 * Live-bbox lookup contract used by the popover positioners. The renderer
 * subscribes element-anchored popovers and the composer to per-page bbox
 * updates; positioning consults this lookup so popovers track page scroll.
 */
export interface AnnotationLiveBboxLookup {
  get: (annotationId: string) => DevtoolsPanelDomRect | undefined
  isStale: (annotationId: string) => boolean
}

export interface DrawingSession {
  strokes: AnnotationDrawingStroke[]
  bounds: AnnotationDrawing['bounds']
}

export function snapPointTo45Degrees(
  origin: AnnotationDrawingPoint,
  point: AnnotationDrawingPoint,
): AnnotationDrawingPoint {
  const dx = point.x - origin.x
  const dy = point.y - origin.y
  const distance = Math.hypot(dx, dy)
  if (distance === 0) return point

  const increment = Math.PI / 4
  const snappedAngle = Math.round(Math.atan2(dy, dx) / increment) * increment
  return {
    x: origin.x + Math.cos(snappedAngle) * distance,
    y: origin.y + Math.sin(snappedAngle) * distance,
  }
}

export function drawingBounds(
  strokes: AnnotationDrawingStroke[],
): AnnotationDrawing['bounds'] {
  let left = Infinity
  let top = Infinity
  let right = -Infinity
  let bottom = -Infinity
  for (const stroke of strokes) {
    // Inflate each stroke by half its width so the bounding rect matches the
    // visible band (otherwise a straight horizontal/vertical line collapses
    // to a 1px-tall bbox and becomes impossible to drag).
    const pad = stroke.width / 2
    for (const point of stroke.points) {
      if (point.x - pad < left) left = point.x - pad
      if (point.y - pad < top) top = point.y - pad
      if (point.x + pad > right) right = point.x + pad
      if (point.y + pad > bottom) bottom = point.y + pad
    }
  }
  if (!Number.isFinite(left)) {
    return { x: 0, y: 0, width: 1, height: 1 }
  }
  return {
    x: left,
    y: top,
    width: Math.max(1, right - left),
    height: Math.max(1, bottom - top),
  }
}

export function canvasRectToScreenRect(
  layout: LayoutUpdateData,
  canvasRect: { x: number; y: number; width: number; height: number },
  minSize = 4,
): { left: number; top: number; right: number; bottom: number; width: number; height: number } {
  const left = canvasToScreenX(layout, canvasRect.x)
  const top = canvasToScreenY(layout, canvasRect.y)
  const right = canvasToScreenX(layout, canvasRect.x + canvasRect.width)
  const bottom = canvasToScreenY(layout, canvasRect.y + canvasRect.height)
  return {
    left,
    top,
    right,
    bottom,
    width: Math.max(minSize, right - left),
    height: Math.max(minSize, bottom - top),
  }
}

/**
 * Canvas-space union bounds for the selection popup's Annotate button:
 * the shared `selectionBbox` union for a real multi-selection (2+ non-group
 * entities), or — the one case `selectionBbox` deliberately excludes — a
 * lone selected group's own rect. Null when neither applies, which the
 * caller reads as "no Annotate button" (single non-group entity selected).
 */
export function selectionAnnotationBounds(
  entities: readonly CanvasSceneEntity[],
  entityIds: readonly string[],
): WorkspaceBounds | null {
  if (entityIds.length === 1) {
    const entity = entities.find((candidate) => candidate.id === entityIds[0])
    if (entity?.kind === 'group') {
      return { x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height }
    }
  }
  return selectionBbox(entities, entityIds, 'canvas')
}

export function annotationScreenPos(
  annotation: Annotation,
  layout: LayoutUpdateData,
  liveBboxes?: AnnotationLiveBboxLookup,
): { x: number; y: number; transform: string } | null {
  const railAnchor = (
    page: LayoutUpdateData['entities'][number],
    preferredY: number,
  ): { x: number; y: number; transform: string } => {
    const y = Math.min(
      Math.max(preferredY, toOverlayY(layout, page.screenY + 10)),
      toOverlayY(layout, page.screenY + page.screenHeight - 10),
    )
    const rightX = page.screenX + page.screenWidth + 12
    const leftX = page.screenX - 12
    const canUseRight = rightX + 280 <= window.innerWidth
    return canUseRight
      ? { x: rightX, y, transform: 'translate(0, -50%)' }
      : { x: leftX, y, transform: 'translate(-100%, -50%)' }
  }

  const anchor = annotation.anchor
  if (anchor.type === 'canvas') {
    return {
      x: canvasToScreenX(layout, anchor.canvasX),
      y: canvasToScreenY(layout, anchor.canvasY),
      transform: 'translate(0, -50%)',
    }
  }
  if (anchor.type === 'region') {
    if (!('docRect' in anchor)) {
      const centerX = canvasToScreenX(
        layout,
        anchor.canvasRect.x + anchor.canvasRect.width / 2,
      )
      const bottom = canvasToScreenY(
        layout,
        anchor.canvasRect.y + anchor.canvasRect.height,
      )
      return {
        x: centerX,
        y: toOverlayY(layout, bottom),
        transform: 'translate(-50%, 0)',
      }
    }
    // Page-anchored region: derive the thread anchor from the page-relative
    // document rect so it scroll-follows, exactly like the region overlay.
    const pageId = annotation.pageAnchor?.pageId
    const page = pageId
      ? layout.entities.find(
          (entity): entity is CanvasScenePageEntity =>
            entity.kind === 'page' && entity.id === pageId,
        )
      : undefined
    if (!page) return null
    // Element-follow (ADR 0032): correct the docRect for its reference
    // element's movement before mapping, matching the region overlay.
    const docRect = correctDocRectForElement(
      anchor.docRect,
      annotation.pageAnchor?.element,
      page.elementPositions,
    )
    const rect = pageDocumentToScreen(docRect, page, layout)
    return {
      x: rect.left + rect.width / 2,
      y: rect.top + rect.height,
      transform: 'translate(-50%, 0)',
    }
  }
  if (anchor.type === 'page' || anchor.type === 'element') {
    const page = layout.entities.find((f) => f.id === anchor.pageId)
    if (!page) return null
    if (anchor.type === 'element' && anchor.boundingBox) {
      const topInset = 8
      const rightInset = 8
      // Prefer the live bbox the page reports on scroll/resize (ADR 0006).
      // The stored `anchor.boundingBox` is captured at creation and goes
      // stale the moment the page scrolls.
      const liveBbox = liveBboxes?.get(annotation.id)
      const bb = liveBbox ?? anchor.boundingBox
      // The thread popover maps through the entity's outer frame (device
      // shell included), then tucks the popover just inside the element's
      // top-right corner and clamps it within the page bounds. That
      // divergence from the content-frame default is deliberate — it lives
      // here as post-processing on the shared transform, not as a second
      // transform.
      const rect = pageViewportToScreen(bb, page, layout, 'entity')
      const pageTop = toOverlayY(layout, page.screenY)
      const x = Math.max(
        page.screenX + rightInset,
        Math.min(
          rect.left + rect.width - rightInset,
          page.screenX + page.screenWidth - rightInset,
        ),
      )
      const y = Math.max(
        pageTop + topInset,
        Math.min(rect.top + topInset, pageTop + page.screenHeight - topInset),
      )
      return { x, y, transform: 'translate(-100%, 0)' }
    }
    if (anchor.type === 'page') {
      const y = toOverlayY(layout, page.screenY + anchor.offsetY * page.screenHeight)
      return railAnchor(page, y)
    }
    return railAnchor(page, toOverlayY(layout, page.screenY + page.screenHeight / 2))
  }
  return null
}

const PENDING_VIEWPORT_PADDING = 8
const PENDING_COMPOSER_MARGIN = 8
const PENDING_COMPOSER_MIN_HEIGHT = 52

/**
 * Position the pending element composer adjacent to the element bbox itself
 * (ADR 0006). Prefers below + left-aligned with the element; flips above when
 * there's no room. Anchoring to the element keeps the composer near the click
 * even when the page entity is much larger than the viewport — anchoring to
 * the page bounds in that case bumped the composer to the top of the screen.
 */
export function elementAnchoredComposerPosition({
  elementLeft,
  elementTop,
  elementHeight,
  composerWidth,
}: {
  elementLeft: number
  elementTop: number
  elementHeight: number
  composerWidth: number
}): { composerX: number; composerY: number } {
  const composerX = Math.min(
    Math.max(elementLeft, PENDING_VIEWPORT_PADDING),
    window.innerWidth - composerWidth - PENDING_VIEWPORT_PADDING,
  )
  const belowY = elementTop + elementHeight + PENDING_COMPOSER_MARGIN
  const aboveY = elementTop - PENDING_COMPOSER_MARGIN - PENDING_COMPOSER_MIN_HEIGHT
  const canRenderBelow =
    belowY + PENDING_COMPOSER_MIN_HEIGHT <= window.innerHeight - PENDING_VIEWPORT_PADDING
  const composerY = canRenderBelow
    ? belowY
    : Math.max(PENDING_VIEWPORT_PADDING, aboveY)
  return { composerX, composerY }
}

/**
 * Translate a pending element annotation's bbox into an overlay-coord rect.
 * Prefers the live bbox the page reports on scroll (ADR 0006); falls back to
 * the click-time `anchor.boundingBox`. Returns null when neither is
 * available or the page isn't on the canvas anymore.
 */
export function pendingElementScreenRect(
  pending: PendingAnnotation,
  layout: LayoutUpdateData,
  liveBboxes?: AnnotationLiveBboxLookup,
): { left: number; top: number; width: number; height: number } | null {
  const anchor = pending.request.anchor
  if (anchor.type !== 'element') return null
  const bbox = liveBboxes?.get(pending.draftId) ?? anchor.boundingBox
  if (!bbox) return null
  const page = layout.entities.find((candidate) => candidate.id === anchor.pageId)
  if (!page) return null
  return pageViewportToScreen(bbox, page, layout)
}

/**
 * Render-time positioner for an element-anchored pending composer. The
 * stored `composerX/Y/Width` on `PendingAnnotation` is the click-time
 * fallback; we prefer the live bbox the page reports on scroll so the
 * composer follows page content (ADR 0006).
 */
export function pendingElementComposerPosition(
  pending: PendingAnnotation,
  layout: LayoutUpdateData,
  liveBboxes?: AnnotationLiveBboxLookup,
): { left: number; top: number; width: number } {
  const fallback = {
    left: pending.composerX,
    top: pending.composerY,
    width: pending.composerWidth,
  }
  const anchor = pending.request.anchor
  if (anchor.type !== 'element') return fallback
  const liveBbox = liveBboxes?.get(pending.draftId)
  if (!liveBbox) return fallback

  const elementRect = pendingElementScreenRect(pending, layout, liveBboxes)
  if (!elementRect) return fallback
  const composerWidth = pending.composerWidth
  const { composerX, composerY } = elementAnchoredComposerPosition({
    elementLeft: elementRect.left,
    elementTop: elementRect.top,
    elementHeight: elementRect.height,
    composerWidth,
  })
  return { left: composerX, top: composerY, width: composerWidth }
}
