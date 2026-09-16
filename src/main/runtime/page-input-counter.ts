/**
 * How much input main has forwarded into each page, as a monotonic count
 * stamped onto every frame that page paints.
 *
 * Electron reports no close event for a popup widget, so canvas-bg infers one
 * from a page paint that arrives after the popup's last paint. A page that
 * animates paints on its own, though, which would read as every popup closing
 * a moment after it opened. The count is what separates the two: a popup closes
 * because of something the user did, so only a paint that follows new input is
 * evidence of one.
 *
 * Pointer moves are deliberately not counted — hovering over a page does not
 * dismiss its picker, and counting moves would make a hovered page look
 * permanently "just interacted with".
 */

const counts = new Map<string, number>()

/** Record one input event that could dismiss a popup (press, key, wheel). */
export function noteInputToPage(pageId: string): void {
  counts.set(pageId, (counts.get(pageId) ?? 0) + 1)
}

export function inputCountForPage(pageId: string): number {
  return counts.get(pageId) ?? 0
}

export function forgetInputCountForPage(pageId: string): void {
  counts.delete(pageId)
}
