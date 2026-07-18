/**
 * Element-attachment render correction (ADR 0030). A page-anchored item stores
 * the document position its reference DOM element sat at when the anchor was
 * captured. When the page reflows, the element moves to a new document
 * position and the item follows it. This module is the one formula for that
 * correction, shared by main (scene builders, `regionCanvasRect`) and the
 * renderer (region rendering) so the two surfaces can never disagree.
 */

import type { PageAnchor } from './page-anchor'

/** Live document positions of a page's tracked selectors, keyed by selector.
 *  The broadcast form of the runtime page's `elementPositions` map. */
export type ElementLivePositions = Record<string, { docX: number; docY: number }>

/**
 * The element-attachment shift, in the same *subtraction* convention as the
 * scroll-follow shift: consumers compute `stored − shift`. Recorded position
 * minus live position, so an element that moved DOWN in the document
 * (`live.docY > recorded docY`) yields a negative `y`, and `stored − (negative)`
 * moves the item DOWN by the same amount — element and item travel together.
 *
 * Zero when the attachment is absent or the selector has not resolved to a live
 * position: the item renders at its stored geometry, it never hides.
 */
export function elementAttachmentShift(
  element: { docX: number; docY: number } | undefined,
  live: { docX: number; docY: number } | undefined,
): { x: number; y: number } {
  if (!element || !live) return { x: 0, y: 0 }
  return { x: element.docX - live.docX, y: element.docY - live.docY }
}

/**
 * Apply the element correction to a page-relative document rect (a region
 * annotation's `docRect`). Looks the element's live position up in the page's
 * broadcast positions and shifts the rect's origin so it follows the element.
 * Pass-through when the anchor carries no element or its selector is unresolved.
 */
export function correctDocRectForElement<
  R extends { x: number; y: number; width: number; height: number },
>(docRect: R, element: PageAnchor['element'], positions: ElementLivePositions | undefined): R {
  if (!element) return docRect
  const shift = elementAttachmentShift(element, positions?.[element.selector])
  if (!shift.x && !shift.y) return docRect
  return { ...docRect, x: docRect.x - shift.x, y: docRect.y - shift.y }
}
