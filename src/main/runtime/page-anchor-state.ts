/**
 * Page-anchor lifecycle for canvas items (see shared/page-anchor.ts for the
 * concept). This module owns the runtime side:
 *
 * - resolving an entity's anchor from where it sits (creation, drag end)
 * - expanding drag/nudge id sets so anchored entities travel with their page
 * - converting a region marquee's canvas rect into the page-relative document
 *   rect stored on page-anchored regions, and back to canvas space for
 *   main-side consumers (region↔page geometry lives here)
 * - clearing anchors when their page is deleted
 *
 * The document-binding gate (hide while the page shows a different URL)
 * lives in document-binding.ts.
 *
 * Anchorable entity kinds today: text, drawing, and shape. The mechanism is
 * generic — a kind opts in by carrying a `pageAnchor` field and appearing in
 * `anchorableEntities()` below. Annotations bind through the same
 * `pageAnchor` field, written once at creation (workspace-annotations.ts).
 */

import {
  canonicalPageUrl,
  pageAnchorFor,
  type PageAnchor,
  type PageAnchorTarget,
} from '../../shared/page-anchor'
import type { Annotation, WorkspaceBounds } from '../../shared/types'
import { pages } from './runtime-context'
import { pageBodyCanvasBounds } from './runtime-geometry'
import { textEntities } from './text-entity-state'
import { drawingEntities } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { pageAnchorScrollShift, pageAnchorElementShift } from './page-anchor-scroll'
import { workspaceAnnotations } from './workspace-model'
import { markDirty } from './layout-dirty'
import { DOC_ARRAY_ENTITY_ORDER, getActiveDoc } from './workspace-doc'
import { captureElementForEntity } from './element-attachment-capture'
import {
  anchorableEntities,
  findAnchorableEntity,
  type AnchorableEntity,
} from './anchorable-entity-store'

export { anchorableEntities, findAnchorableEntity }

/** Runtime pages as anchor targets, in back-to-front stack order. */
function pageAnchorTargets(): PageAnchorTarget[] {
  const rank = new Map(
    getActiveDoc()
      .getArray<string>(DOC_ARRAY_ENTITY_ORDER)
      .toArray()
      .map((id, index) => [id, index]),
  )
  return pages
    .map((page) => ({
      id: page.id,
      url: page.url,
      bounds: pageBodyCanvasBounds(page),
    }))
    .sort((a, b) => (rank.get(a.id) ?? -1) - (rank.get(b.id) ?? -1))
}

export function resolvePageAnchorForBounds(bounds: {
  x: number
  y: number
  width: number
  height: number
}): PageAnchor | null {
  return pageAnchorFor(bounds, pageAnchorTargets())
}

function sameAnchor(a: PageAnchor | undefined, b: PageAnchor | null): boolean {
  if (!a && !b) return true
  if (!a || !b) return false
  return a.pageId === b.pageId && a.pageUrl === b.pageUrl
}

/**
 * Fold an anchored entity's accumulated scroll and element-attachment shifts
 * into its stored coordinates and refresh the anchor's references — the
 * apparent position is unchanged, but stored coords now mean what the user
 * sees, so the placement test below (and every canvas-coordinate consumer) is
 * honest. Both references ride this tracked mutation (a real user move), so
 * undo restores the pre-fold coords and references together. Returns true when
 * coordinates moved.
 */
function rebaseAnchorScroll(entity: AnchorableEntity): boolean {
  const anchor = entity.pageAnchor
  if (!anchor) return false
  const scroll = pageAnchorScrollShift(anchor)
  const element = pageAnchorElementShift(anchor)
  const shiftX = scroll.x + element.x
  const shiftY = scroll.y + element.y
  if (!shiftX && !shiftY) return false
  const page = pages.find((candidate) => candidate.id === anchor.pageId)
  if (!page) return false
  entity.canvasX -= shiftX
  entity.canvasY -= shiftY
  // Drawing strokes are stored in absolute canvas coords, so they move with
  // the folded shift or the bbox drifts away from the visible ink.
  const drawing = drawingEntities.find((candidate) => candidate.id === entity.id)
  if (drawing) {
    drawing.strokes = drawing.strokes.map((stroke) => ({
      ...stroke,
      points: stroke.points.map((point) => ({ x: point.x - shiftX, y: point.y - shiftY })),
    }))
  }
  // A fresh object, not an in-place mutation — the doc diff-sync detects
  // object fields by identity. Reset each reference the fold consumed: the
  // scroll offset to the page's live scroll (only when scroll-following, to
  // keep a frame-pinned anchor frame-pinned), and the element to its live
  // document position so the folded element shift isn't re-applied.
  const liveElement = anchor.element
    ? page.elementPositions?.get(anchor.element.selector)
    : undefined
  entity.pageAnchor = {
    ...anchor,
    ...(anchor.scrollY === undefined
      ? {}
      : { scrollX: page.scrollX ?? 0, scrollY: page.scrollY ?? 0 }),
    ...(anchor.element && liveElement
      ? { element: { ...anchor.element, docX: liveElement.docX, docY: liveElement.docY } }
      : {}),
  }
  return true
}

/**
 * Recompute an entity's anchor from where it currently sits: anchored when
 * its center is inside a page's body, free otherwise. Grouped entities never
 * anchor — group membership already owns their movement. Returns true when
 * the anchor changed.
 */
export function reanchorEntityById(entityId: string): boolean {
  const entity = findAnchorableEntity(entityId)
  if (!entity) return false
  const rebased = rebaseAnchorScroll(entity)
  const next = entity.parentGroupId
    ? null
    : resolvePageAnchorForBounds({
        x: entity.canvasX,
        y: entity.canvasY,
        width: entity.width,
        height: entity.height,
      })
  if (sameAnchor(entity.pageAnchor, next)) {
    if (rebased) markDirty('canvas', 'sidebar')
    // Drag-end over the same page keeps the anchor object but sits over
    // (possibly) different content, so re-capture the reference element
    // (ADR 0032). No-op when the entity is free.
    if (entity.pageAnchor) captureElementForEntity(entityId)
    return rebased
  }
  if (next) {
    // Anchored entities scroll-follow: stamp the page's scroll offset so the
    // scene projection can shift by the delta since (page-anchor-scroll.ts).
    const page = pages.find((candidate) => candidate.id === next.pageId)
    next.scrollX = page?.scrollX ?? 0
    next.scrollY = page?.scrollY ?? 0
    entity.pageAnchor = next
    // Newly anchored (creation or drop-on): find the element under the item's
    // center. Dragging off deletes the anchor, which clears the attachment.
    captureElementForEntity(entityId)
  } else {
    delete entity.pageAnchor
  }
  markDirty('canvas', 'sidebar')
  return true
}

/**
 * Attach an entity to a specific page, bypassing placement resolution — used
 * when a page and its anchored items are cloned together (paste, duplicate)
 * and the clones must re-attach to the cloned page rather than whatever page
 * happens to sit under them. Stamps the page's live scroll reference and
 * re-derives the element attachment (ADR 0032: the attachment is derived
 * metadata, so a clone re-captures rather than inheriting the source's).
 */
export function anchorEntityToPage(entityId: string, pageId: string): boolean {
  const entity = findAnchorableEntity(entityId)
  const page = pages.find((candidate) => candidate.id === pageId)
  if (!entity || !page) return false
  const pageUrl = canonicalPageUrl(page.url)
  entity.pageAnchor = {
    pageId,
    ...(pageUrl ? { pageUrl } : {}),
    scrollX: page.scrollX ?? 0,
    scrollY: page.scrollY ?? 0,
  }
  captureElementForEntity(entityId)
  markDirty('canvas', 'sidebar')
  return true
}

/**
 * Ids of entities anchored to any page in `ids`, for expanding a drag/nudge
 * set — anchored entities travel with their page. Already-present ids are
 * not duplicated.
 */
export function withPageAnchoredEntityIds(ids: string[]): string[] {
  const pageIds = new Set(ids.filter((id) => pages.some((page) => page.id === id)))
  if (!pageIds.size) return ids
  const present = new Set(ids)
  const carried = anchorableEntities()
    .filter((entity) => entity.pageAnchor && pageIds.has(entity.pageAnchor.pageId))
    .map((entity) => entity.id)
    .filter((id) => !present.has(id))
  return carried.length ? [...ids, ...carried] : ids
}

/**
 * Convert a region marquee's canvas rect into the page-relative document rect
 * stored on a page-anchored region. A page body occupies canvas space 1:1 with
 * its own CSS pixels at this layer (region-select already treats them so), so
 * the only terms are the page body's canvas origin and its current scroll
 * offset: `docRect = canvasRect - pageBodyOrigin + scroll`. Returns null if the
 * page is gone (caller keeps the canvasRect anchor — a defensive fallback; the
 * page id came from a live grab, so this shouldn't happen).
 */
export function canvasRectToPageDocRect(
  canvasRect: WorkspaceBounds,
  pageId: string,
): WorkspaceBounds | null {
  const page = pages.find((candidate) => candidate.id === pageId)
  if (!page) return null
  const body = pageBodyCanvasBounds(page)
  return {
    x: canvasRect.x - body.x + (page.scrollX ?? 0),
    y: canvasRect.y - body.y + (page.scrollY ?? 0),
    width: canvasRect.width,
    height: canvasRect.height,
  }
}

/**
 * The current canvas rect of a region annotation, for main-side consumers
 * (focus/zoom bounds, presence cursor) that need where the region sits on the
 * surface right now. A canvas-anchored region returns its stored `canvasRect`
 * unchanged. A page-anchored region's `docRect` is inverted through its page's
 * body origin and live scroll offset, so it tracks the page as it moves and
 * scrolls. Null when the page a `docRect` names is gone.
 */
export function regionCanvasRect(
  annotation: Pick<Annotation, 'anchor' | 'pageAnchor'>,
): WorkspaceBounds | null {
  const anchor = annotation.anchor
  if (anchor.type !== 'region') return null
  if (!('docRect' in anchor)) return anchor.canvasRect
  const pageId = annotation.pageAnchor?.pageId
  if (!pageId) return null
  const page = pages.find((candidate) => candidate.id === pageId)
  if (!page) return null
  const body = pageBodyCanvasBounds(page)
  // Element-follow (ADR 0032): the region tracks the element under its center
  // through page reflow, alongside the scroll-follow subtraction below.
  const element = pageAnchorElementShift(annotation.pageAnchor)
  return {
    x: body.x + anchor.docRect.x - element.x - (page.scrollX ?? 0),
    y: body.y + anchor.docRect.y - element.y - (page.scrollY ?? 0),
    width: anchor.docRect.width,
    height: anchor.docRect.height,
  }
}

/** Anchors referencing a deleted page are cleared — the item goes canvas-bound. */
export function clearPageAnchorsForPage(pageId: string): void {
  let changed = false
  for (const entity of anchorableEntities()) {
    if (entity.pageAnchor?.pageId !== pageId) continue
    delete entity.pageAnchor
    changed = true
  }
  for (const annotation of workspaceAnnotations) {
    if (annotation.pageAnchor?.pageId !== pageId) continue
    delete annotation.pageAnchor
    changed = true
  }
  if (changed) markDirty('canvas', 'sidebar')
}
