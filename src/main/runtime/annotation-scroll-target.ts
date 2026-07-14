/**
 * Scroll-to-comment targeting (docs/plans/scroll-tracking.md phase 4).
 *
 * Clicking a comment should smooth-scroll its page until the commented content
 * is in view. The eased scroll ramp already exists (the page preload's
 * `dispatchScroll` handler, ridden by the CLI `scroll` verb); the one new thing
 * is *where* to scroll, which differs by anchor type.
 *
 * `computeAnnotationScrollTarget` is pure and synchronous — it answers "what
 * document position should be revealed", nothing more — so it is trivially
 * testable under the electron stub, where the IPC send is not observable.
 * `dispatchScrollToAnnotation` layers the impure parts on top: the ~1/3-down
 * offset, the current-scroll delta, and the `sendPageIpc` ramp dispatch.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import type { Annotation } from '../../shared/types'
import { pages } from './runtime-context'
import { pageBodyCanvasBounds } from './runtime-geometry'
import { sendPageIpc } from './page-ipc'
import { queryPageElements } from './page-queries'

export interface AnnotationScrollTarget {
  pageId: string
  /** Position in the page's document CSS px to bring into view. */
  documentY: number
}

/**
 * The document position a comment click should reveal, per anchor type, or null
 * when there is nothing to scroll:
 *
 * - **canvas point** → null. Marks canvas space, no page position exists.
 * - **region, canvas-anchored** (no `docRect`) → null. Same reason (plan
 *   decision, 2026-07-13).
 * - **region, page-anchored** (`docRect`) → `docRect.y`. The clean primary
 *   case: phase 3 stores exactly this.
 * - **element** → best-effort from the stored bbox (`boundingBox.y` plus the
 *   page's current scroll). Element anchors already scroll-follow via the live
 *   bbox round trip, so this is a fallback; `dispatchScrollToAnnotation`
 *   refines it against the live DOM when the selector still resolves.
 * - **page** (`offsetY`) → `offsetY × scrollHeight`. Needs the page's document
 *   height, which the phase-1 offset broadcast now also carries.
 *
 * A target whose page is gone returns null — it can't be revealed.
 */
export function computeAnnotationScrollTarget(
  annotation: Pick<Annotation, 'anchor' | 'pageAnchor'>,
): AnnotationScrollTarget | null {
  const anchor = annotation.anchor
  switch (anchor.type) {
    case 'canvas':
      return null
    case 'region': {
      if (!('docRect' in anchor)) return null
      const pageId = annotation.pageAnchor?.pageId
      if (!pageId || !pages.some((page) => page.id === pageId)) return null
      return { pageId, documentY: anchor.docRect.y }
    }
    case 'element': {
      const page = pages.find((candidate) => candidate.id === anchor.pageId)
      if (!page || !anchor.boundingBox) return null
      return { pageId: anchor.pageId, documentY: anchor.boundingBox.y + (page.scrollY ?? 0) }
    }
    case 'page': {
      const page = pages.find((candidate) => candidate.id === anchor.pageId)
      if (!page) return null
      return { pageId: anchor.pageId, documentY: anchor.offsetY * (page.scrollHeight ?? 0) }
    }
  }
}

/** Land the anchor ~1/3 down the viewport rather than flush at the top: a
 *  sticky site header often covers top-pinned content, and content at the very
 *  top edge reads as cut off (plan lines 203-205). */
const VIEWPORT_ANCHOR_FRACTION = 1 / 3

/**
 * Re-resolve an element anchor's selector against the live page and read the
 * hit element's document Y. Returns null when the selector is stale/empty or
 * the query fails, so the caller keeps the stored-bbox fallback. Best-effort:
 * never throws.
 */
async function resolveElementDocumentY(
  pageId: string,
  selector: string,
): Promise<number | null> {
  try {
    const results = await queryPageElements(pageId, selector, 1)
    if (!Array.isArray(results) || results.length === 0) return null
    const first = results[0] as { position?: { documentY?: number } } | null
    const documentY = first?.position?.documentY
    return typeof documentY === 'number' ? documentY : null
  } catch {
    return null
  }
}

/**
 * Smooth-scroll a comment's page until its anchor lands ~1/3 down the viewport,
 * reusing the preload's eased ramp. No-op when there is nothing to reveal
 * (canvas point, canvas-anchored region, missing page). Fire-and-forget: the
 * click handler does not await it.
 */
export async function dispatchScrollToAnnotation(
  annotation: Pick<Annotation, 'anchor' | 'pageAnchor'>,
): Promise<void> {
  const target = computeAnnotationScrollTarget(annotation)
  if (!target) return
  const page = pages.find((candidate) => candidate.id === target.pageId)
  if (!page) return

  let documentY = target.documentY
  if (annotation.anchor.type === 'element') {
    const liveY = await resolveElementDocumentY(annotation.anchor.pageId, annotation.anchor.selector)
    if (liveY != null) documentY = liveY
  }

  const body = pageBodyCanvasBounds(page)
  const targetScrollY = Math.max(0, documentY - body.height * VIEWPORT_ANCHOR_FRACTION)
  const deltaY = targetScrollY - (page.scrollY ?? 0)
  // Probe `resolveScrollTarget` from the page's viewport center — the page body
  // occupies canvas space 1:1 with its own CSS px, so body.width/height are the
  // viewport dimensions.
  await sendPageIpc(page.id, ipcChannels.dispatchScroll, {
    x: body.width / 2,
    y: body.height / 2,
    deltaX: 0,
    deltaY,
  }).catch(() => {})
}
