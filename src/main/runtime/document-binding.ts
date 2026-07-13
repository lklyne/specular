/**
 * Document binding — "is this item on its page's current document?"
 *
 * Page-bound items — anchored entities and page-anchored annotations alike —
 * record their binding in one place: `pageAnchor { pageId, pageUrl }`
 * (shared/page-anchor.ts). While the page shows a different URL the item is
 * off-document and leaves the layout broadcast entirely — main owns the gate
 * at this seam, so renderers never re-derive visibility from URLs. The
 * sidebar shares the underlying predicate with a different policy: it dims
 * off-document rows instead of hiding them (sidebar-builder.ts).
 */

import { matchesPageUrl, type PageAnchor } from '../../shared/page-anchor'
import { findPageById } from './runtime-context'

/**
 * Whether a recorded document URL is off its context page's current document
 * (cached `page.url`). False when the item has no page context, the page is
 * gone, or either side lacks a URL — only a live page showing a *different*
 * document hides anything.
 */
export function offPageDocument(
  pageId: string | null | undefined,
  recordedUrl: string | undefined,
): boolean {
  if (!pageId) return false
  const page = findPageById(pageId)
  if (!page) return false
  return !matchesPageUrl(recordedUrl, page.url)
}

/**
 * Whether a page-bound item (anchored entity or annotation) is off its
 * page's current document. Hidden items are omitted from the layout
 * broadcast, which removes them from rendering and renderer-side
 * hit-testing in one place. Items without a `pageAnchor` are canvas-bound
 * and never hide.
 */
export function hiddenByPageAnchor(item: { id?: string; pageAnchor?: PageAnchor }): boolean {
  const anchor = item.pageAnchor
  return anchor ? offPageDocument(anchor.pageId, anchor.pageUrl) : false
}
