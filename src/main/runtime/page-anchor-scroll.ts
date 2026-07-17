/**
 * Scroll-follow shift for page-anchored entities. Its own module (not
 * page-anchor-state.ts) so entity-state modules can import it without a
 * cycle — page-anchor-state imports them for `anchorableEntities()`.
 */

import type { PageAnchor } from '../../shared/page-anchor'
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
