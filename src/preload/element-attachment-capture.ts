/**
 * Element attachment capture (ADR 0030).
 *
 * Finds the DOM element under a document point and returns a selector plus
 * that element's document position — the raw material for `PageAnchor.element`
 * (`shared/page-anchor.ts`). Capture is fire-and-forget enrichment: it never
 * blocks placement, and an unresolvable result degrades to `document.body`
 * (tracking the document top, which is a no-op scroll-follow) rather than
 * failing — the render-time correction this feeds is never a visibility gate.
 *
 * Capture rule: hit-test the point, walk up past trivial wrappers to the
 * nearest *meaningful* element (has an id/role/classes, and a reasonable
 * size), stopping at `body`. When that comes up empty — the hit-test found
 * nothing, or nothing on the way up qualified — fall back to scanning for
 * the nearest meaningful element whose vertical span contains that document
 * Y. `document.body` is the final fallback either way.
 */

import {
  buildUniqueSelector,
  elementSelectorParts,
  pickContentElementAtPoint,
} from './dom-element-utils'

export interface CapturedElementAnchor {
  selector: string
  docX: number
  docY: number
  viewportPositioned?: boolean
}

// An element smaller than this in either dimension reads as a bare inline
// wrapper (a `<span>` around one glyph, an icon's `<i>`) — not something a
// user would think of as "the thing under my annotation."
const MIN_MEANINGFUL_DIMENSION = 8

// An element spanning more than this multiple of the viewport in both
// dimensions is almost certainly a structural shell (`<body>`, a full-bleed
// layout wrapper) rather than a piece of content, so it's treated the same
// as an unstyled wrapper and skipped in favor of walking further up (which
// bottoms out at `body` anyway).
const MAX_MEANINGFUL_VIEWPORT_RATIO = 2

/**
 * Whether an element is worth naming as a reference point: it carries an id,
 * ARIA role, or class (the same signals `elementSelectorParts` prefers for a
 * readable selector segment), and its footprint is neither a sliver nor a
 * full-page shell.
 */
function isMeaningfulElement(element: Element): boolean {
  // `elementSelectorParts` already prefers id, then role, then classes — the
  // same "does this element carry an identity" signal we want here.
  const { remainder } = elementSelectorParts(element)
  if (!remainder) return false

  const rect = element.getBoundingClientRect()
  if (rect.width < MIN_MEANINGFUL_DIMENSION || rect.height < MIN_MEANINGFUL_DIMENSION) return false

  const maxWidth = window.innerWidth * MAX_MEANINGFUL_VIEWPORT_RATIO
  const maxHeight = window.innerHeight * MAX_MEANINGFUL_VIEWPORT_RATIO
  if (rect.width > maxWidth && rect.height > maxHeight) return false

  return true
}

/** Walk up from `start` to the nearest meaningful ancestor, stopping at (and excluding) `body`. Null if none qualifies. */
function walkUpToMeaningful(start: Element): Element | null {
  let current: Element | null = start
  while (current && current !== document.body) {
    if (isMeaningfulElement(current)) return current
    current = current.parentElement
  }
  return null
}

function documentPosition(element: Element): { docX: number; docY: number } {
  const rect = element.getBoundingClientRect()
  return { docX: rect.left + window.scrollX, docY: rect.top + window.scrollY }
}

/** Fixed descendants and sticky rails move in viewport space during scroll.
 * Walk ancestors because the meaningful element is often a plain child of
 * the element that actually establishes the positioning behavior. */
export function isViewportPositionedElement(element: Element): boolean {
  let current: Element | null = element
  while (current && current !== document.body) {
    const position = window.getComputedStyle(current).position
    if (position === 'fixed' || position === 'sticky') return true
    current = current.parentElement
  }
  return false
}

/**
 * Rare fallback for when the hit-test and walk-up come up empty (a miss, or
 * every ancestor up to `body` is a trivial wrapper): scan for meaningful
 * elements whose vertical span contains `docY`, and return the one
 * horizontally closest to `docX`. A bounded, one-shot scan — this only runs
 * when the primary path finds nothing.
 */
function nearestMeaningfulAtY(docX: number, docY: number): Element | null {
  let best: Element | null = null
  let bestDistance = Infinity
  for (const element of document.body.querySelectorAll('*')) {
    if (!isMeaningfulElement(element)) continue
    const rect = element.getBoundingClientRect()
    const top = rect.top + window.scrollY
    const bottom = top + rect.height
    if (docY < top || docY > bottom) continue
    const center = rect.left + window.scrollX + rect.width / 2
    const distance = Math.abs(center - docX)
    if (distance < bestDistance) {
      bestDistance = distance
      best = element
    }
  }
  return best
}

/**
 * Capture the reference element at a document point, per the ADR 0030 rule.
 * Returns null only when the page has no `document.body` to fall back to.
 */
export function captureElementAtDocumentPoint(docX: number, docY: number): CapturedElementAnchor | null {
  if (!document.body) return null

  const viewportX = docX - window.scrollX
  const viewportY = docY - window.scrollY
  const hit = pickContentElementAtPoint(viewportX, viewportY)
  const meaningful = hit ? walkUpToMeaningful(hit) : null
  const target = meaningful ?? nearestMeaningfulAtY(docX, docY) ?? document.body

  const viewportPositioned = isViewportPositionedElement(target)
  return {
    selector: buildUniqueSelector(target),
    ...documentPosition(target),
    ...(viewportPositioned ? { viewportPositioned: true } : {}),
  }
}
