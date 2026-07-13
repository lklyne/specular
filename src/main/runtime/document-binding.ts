/**
 * Document binding — "is this item on its page's current document?"
 *
 * Page-bound items record the URL their page showed when they were placed:
 * anchored entities in `PageAnchor.pageUrl`, annotations in
 * `metadata.pageUrl`. While the page shows a different URL the item is
 * off-document and leaves the layout broadcast entirely — main owns the gate
 * at this seam, so renderers never re-derive visibility from URLs. The
 * sidebar shares the underlying predicate with a different policy: it dims
 * off-document rows instead of hiding them (sidebar-builder.ts).
 */

import type { Annotation } from '../../shared/types'
import { annotationContextPageId } from '../../shared/annotation-utils'
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
 * Whether an anchored entity is off its page's current document. Hidden
 * entities are omitted from the scene payload, which removes them from
 * rendering and renderer-side hit-testing in one place.
 */
export function entityHiddenByPageAnchor(entity: { id: string; pageAnchor?: PageAnchor }): boolean {
  const anchor = entity.pageAnchor
  return anchor ? offPageDocument(anchor.pageId, anchor.pageUrl) : false
}

/**
 * Whether an annotation is off its context page's current document. Hidden
 * annotations are omitted from the layout broadcast — badges, region rects,
 * thread popovers, and live-bbox subscriptions all disappear with them.
 */
export function annotationHiddenByPageDocument(annotation: Annotation): boolean {
  return offPageDocument(annotationContextPageId(annotation), annotation.metadata?.pageUrl)
}
