/**
 * Scroll-follow shift for page-anchored entities. Its own module (not
 * page-anchor-state.ts) so entity-state modules can import it without a
 * cycle — page-anchor-state imports them for `anchorableEntities()`.
 */

import type { PageAnchor } from '../../shared/page-anchor'
import { elementAttachmentShift } from '../../shared/element-attachment'
import { pages } from './runtime-context'

/**
 * How far the anchor's page has scrolled since the anchor was written, in
 * canvas units (a page body is 1:1 with its own CSS pixels in canvas space).
 * Zero for free entities, gone pages, and anchors without a scroll reference
 * (frame-pinned — see shared/page-anchor.ts).
 */
export function pageAnchorScrollShift(anchor: PageAnchor | undefined): { x: number; y: number } {
  if (!anchor || anchor.scrollY === undefined) return { x: 0, y: 0 }
  const page = pages.find((candidate) => candidate.id === anchor.pageId)
  if (!page) return { x: 0, y: 0 }
  return {
    x: (page.scrollX ?? 0) - (anchor.scrollX ?? 0),
    y: (page.scrollY ?? 0) - (anchor.scrollY ?? 0),
  }
}

/**
 * Element-attachment shift for an anchor (ADR 0030): how far the anchor's
 * reference element has moved in the page's document since the anchor was
 * captured, in the same subtraction convention as `pageAnchorScrollShift`
 * (consumers compute `stored − shift`). Zero when the anchor carries no
 * element, the page is gone, or the selector has not resolved to a live
 * position — the item renders at its stored geometry, never hides.
 */
export function pageAnchorElementShift(anchor: PageAnchor | undefined): { x: number; y: number } {
  if (!anchor?.element) return { x: 0, y: 0 }
  const page = pages.find((candidate) => candidate.id === anchor.pageId)
  if (!page) return { x: 0, y: 0 }
  const live = page.elementPositions?.get(anchor.element.selector)
  return elementAttachmentShift(anchor.element, live)
}
