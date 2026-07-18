/**
 * Page anchoring — the generic "hook to a page" utility.
 *
 * A canvas entity is either free-form (the default) or carries a `PageAnchor`
 * tying it to a page entity and the URL that page showed when the entity was
 * placed. Anchored entities keep their positions in canvas coordinates — the
 * placement *relative to the page* is implicit, and stays correct because
 * anchored entities move with their page (the drag/nudge sets expand to
 * include them). The anchor answers two questions the coordinates can't:
 * which page owns this item (sidebar nesting), and which document it belongs
 * to (visuals hide when the page navigates to a different URL).
 *
 * For entities, anchoring is decided by placement, not by a mode: an entity
 * created with its center inside a page's body anchors to that page;
 * dragging it off clears the anchor; dropping it onto a page sets one.
 * Grouped entities never auto-anchor — group membership already owns their
 * movement.
 *
 * Annotations carry the same `pageAnchor`, written once at creation
 * (element/page anchors from their anchor page; region anchors iff the
 * marquee grabbed page content; canvas points never) and never re-resolved —
 * an annotation's binding is part of what it says.
 */

export interface PageAnchor {
  /** The page entity this item is anchored to. */
  pageId: string
  /** Canonical URL (hash stripped) of the document the item was placed on.
   *  Absent when the page had no URL at placement — such anchors always
   *  count as "on the current page". */
  pageUrl?: string
  /**
   * Page scroll offset (page CSS px) recorded when the anchor was written.
   * Present iff the entity kind scroll-follows: the entity renders shifted by
   * the delta between the page's live scroll and this reference, so it tracks
   * the document content it was placed over (like an annotation's docRect,
   * expressed as a delta so `canvasX/Y` stays the stored truth). Absent means
   * frame-pinned — the entity ignores page scroll (text and drawings today).
   */
  scrollX?: number
  scrollY?: number
  /**
   * Element attachment (ADR 0030): a DOM selector for the item's reference
   * element, plus that element's document position at the moment it was
   * captured. Written by a fire-and-forget preload query at creation and
   * drag-end — the user never chooses the element, placement does, so this
   * is derived metadata, not a decision.
   *
   * `canvasX/Y` (or `docRect`, for regions) stays the stored truth. At render
   * time, `(element's live document position − docX/docY)` is applied as a
   * shift alongside the scroll-follow shift above, and folds into stored
   * coordinates on reanchor the same way. This is a render-time correction,
   * never a visibility gate: an unresolvable selector means zero shift — the
   * item renders at its stored geometry, it never hides.
   */
  element?: {
    selector: string
    docX: number
    docY: number
    /** The selected element lives inside a fixed or sticky containing rail.
     * Its authoritative element correction already accounts for scrolling,
     * so the generic fast document-scroll transform must not also move it. */
    viewportPositioned?: boolean
  }
}

/** Whether an anchored item's renderer should apply the live document-scroll
 * residual. Fixed/sticky attachments are already positioned by the element
 * correction and would otherwise receive the scroll delta twice. */
export function shouldFastFollowPageScroll(
  anchor: PageAnchor | undefined,
  liveElement?: { viewportPositioned?: boolean },
): boolean {
  return (
    anchor?.scrollY !== undefined &&
    anchor.element?.viewportPositioned !== true &&
    liveElement?.viewportPositioned !== true
  )
}

export interface PageAnchorTarget {
  id: string
  url?: string | null
  /** The page's body bounds in canvas coordinates (content area, inside any
   *  device shell). */
  bounds: { x: number; y: number; width: number; height: number }
}

/**
 * Canonical form for page-URL comparison: hash stripped, otherwise the URL
 * as-is. Non-URL strings pass through so file:// fixtures and dev servers
 * with unusual schemes still compare by exact string.
 */
export function canonicalPageUrl(value: string | undefined | null): string | undefined {
  if (!value) return undefined
  const trimmed = value.trim()
  if (!trimmed) return undefined
  try {
    const parsed = new URL(trimmed)
    parsed.hash = ''
    return parsed.toString()
  } catch {
    return trimmed
  }
}

/**
 * Whether a recorded URL still names the page's current document. False only
 * when both sides carry a URL and they disagree (hash-insensitive). A missing
 * recorded URL always matches, as does a page with no URL yet (loading).
 */
export function matchesPageUrl(
  recordedUrl: string | undefined | null,
  currentUrl: string | undefined | null,
): boolean {
  const recorded = canonicalPageUrl(recordedUrl)
  const current = canonicalPageUrl(currentUrl)
  if (!recorded || !current) return true
  return recorded === current
}

/**
 * Resolve the anchor for an entity from its bounds: the topmost page whose
 * body contains the entity's center, or null when the entity sits on empty
 * canvas. `pages` must be in back-to-front stack order — the last hit wins.
 */
export function pageAnchorFor(
  entityBounds: { x: number; y: number; width: number; height: number },
  pages: readonly PageAnchorTarget[],
): PageAnchor | null {
  const centerX = entityBounds.x + entityBounds.width / 2
  const centerY = entityBounds.y + entityBounds.height / 2
  for (let i = pages.length - 1; i >= 0; i -= 1) {
    const page = pages[i]
    const { bounds } = page
    if (
      centerX >= bounds.x &&
      centerX < bounds.x + bounds.width &&
      centerY >= bounds.y &&
      centerY < bounds.y + bounds.height
    ) {
      const pageUrl = canonicalPageUrl(page.url)
      return { pageId: page.id, ...(pageUrl ? { pageUrl } : {}) }
    }
  }
  return null
}
