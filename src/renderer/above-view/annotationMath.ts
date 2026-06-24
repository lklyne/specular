import type {
  Annotation,
  AnnotationCreateRequest,
  AnnotationDrawing,
  AnnotationDrawingPoint,
  AnnotationDrawingStroke,
  DevtoolsPanelDomRect,
  LayoutUpdateData,
} from '../../shared/types'
import {
  canvasToScreenX,
  canvasToScreenY,
  toOverlayY,
} from '../../shared/gesture-utils'


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

export function pathD(points: AnnotationDrawingPoint[]): string {
  if (!points.length) return ''
  if (points.length === 1) {
    const [point] = points
    return `M ${point.x} ${point.y} L ${point.x + 0.01} ${point.y + 0.01}`
  }
  if (points.length === 2) {
    return points
      .map((point, index) => `${index === 0 ? 'M' : 'L'} ${point.x} ${point.y}`)
      .join(' ')
  }

  const [firstPoint, secondPoint] = points
  let path = `M ${firstPoint.x} ${firstPoint.y}`
  const firstMidpoint = {
    x: (firstPoint.x + secondPoint.x) / 2,
    y: (firstPoint.y + secondPoint.y) / 2,
  }

  path += ` Q ${firstPoint.x} ${firstPoint.y} ${firstMidpoint.x} ${firstMidpoint.y}`

  for (let index = 1; index < points.length - 1; index += 1) {
    const point = points[index]
    const nextPoint = points[index + 1]
    const midpoint = {
      x: (point.x + nextPoint.x) / 2,
      y: (point.y + nextPoint.y) / 2,
    }
    path += ` Q ${point.x} ${point.y} ${midpoint.x} ${midpoint.y}`
  }

  const lastPoint = points[points.length - 1]
  path += ` Q ${lastPoint.x} ${lastPoint.y} ${lastPoint.x} ${lastPoint.y}`
  return path
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
  if (anchor.type === 'file') {
    const fileEntity = layout.entities.find((e) => e.id === anchor.fileId && e.kind === 'file')
    if (!fileEntity) return null
    return {
      x: fileEntity.screenX + fileEntity.screenWidth,
      y: toOverlayY(layout, fileEntity.screenY),
      transform: 'translate(-100%, 0)',
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
      const x =
        page.screenX +
        (bb.x + bb.width) *
          (page.screenWidth / page.width) -
        rightInset
      const y =
        page.screenY +
        bb.y * (page.screenHeight / page.height) +
        topInset
      const clampedX = Math.max(
        page.screenX + rightInset,
        Math.min(x, page.screenX + page.screenWidth - rightInset),
      )
      const clampedY = Math.max(
        page.screenY + topInset,
        Math.min(y, page.screenY + page.screenHeight - topInset),
      )
      return {
        x: clampedX,
        y: toOverlayY(layout, clampedY),
        transform: 'translate(-100%, 0)',
      }
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
  const contentScreenX =
    'contentScreenX' in page && page.contentScreenX != null ? page.contentScreenX : page.screenX
  const contentScreenY =
    'contentScreenY' in page && page.contentScreenY != null ? page.contentScreenY : page.screenY
  const contentScreenWidth =
    'contentScreenWidth' in page && page.contentScreenWidth != null
      ? page.contentScreenWidth
      : page.screenWidth
  const contentScreenHeight =
    'contentScreenHeight' in page && page.contentScreenHeight != null
      ? page.contentScreenHeight
      : page.screenHeight
  const scaleX = contentScreenWidth / page.width
  const scaleY = contentScreenHeight / page.height
  return {
    left: contentScreenX + bbox.x * scaleX,
    top: toOverlayY(layout, contentScreenY + bbox.y * scaleY),
    width: bbox.width * scaleX,
    height: bbox.height * scaleY,
  }
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

