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
 * Anchoring is decided by placement, not by a mode: an entity created with
 * its center inside a page's body anchors to that page; dragging it off
 * clears the anchor; dropping it onto a page sets one. Grouped entities never
 * auto-anchor — group membership already owns their movement.
 *
 * Annotations predate this utility and carry the same idea in
 * `metadata.pageUrl`; both share the URL canonicalization below.
 */

export interface PageAnchor {
  /** The page entity this item is anchored to. */
  pageId: string
  /** Canonical URL (hash stripped) of the document the item was placed on.
   *  Absent when the page had no URL at placement — such anchors always
   *  count as "on the current page". */
  pageUrl?: string
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

/** Whether an anchored item belongs on the page's current document. */
export function pageAnchorOnCurrentUrl(
  anchor: PageAnchor | undefined,
  currentPageUrl: string | undefined | null,
): boolean {
  if (!anchor) return true
  return matchesPageUrl(anchor.pageUrl, currentPageUrl)
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
