/**
 * Window → page / window → canvas geometry helpers.
 *
 * The renderer dispatches comment-tool clicks in window coordinates (the
 * same space `forwardPointerToPage` already speaks). Main needs to:
 *  1. resolve which page sits under the click (if any), in render order;
 *  2. fall back to a canvas-coord anchor when no page is hit.
 *
 * Pure plumbing on top of `boundScreenBoundsForPage` and the runtime
 * `zoom` / `pan` / `canvasOrigin`. No external state.
 */

import { pages, pan, zoom } from './runtime-context'
import {
  boundCanvasOrigin,
  boundEffectivePageContentSize,
  boundScreenBoundsForPage,
} from './runtime-geometry'

export type PageHit = {
  pageId: string
  /** Page-local x in CSS pixels, suitable for `query-element-at-point`. */
  localX: number
  /** Page-local y in CSS pixels, suitable for `query-element-at-point`. */
  localY: number
}

/**
 * Returns the topmost page whose `page` rect contains the window-coord
 * (windowX, windowY), or null. Iterates in reverse `pages` order so later
 * entries — which paint on top — win.
 *
 * `localX`/`localY` are returned in the page's CSS coordinate space (the
 * space `elementFromPoint` operates on), derived from the rendered host
 * rect via `cssWidth / hostWidth`. At canvas zoom = 1 this collapses to a
 * simple offset; at any other zoom the scale factor is required for
 * element resolution to land on the right node.
 */
export function pageAtWindowPoint(windowX: number, windowY: number): PageHit | null {
  for (let i = pages.length - 1; i >= 0; i--) {
    const page = pages[i]
    if (!page || page.host.webContents.isDestroyed()) continue
    const bounds = boundScreenBoundsForPage(page).page
    if (bounds.width <= 0 || bounds.height <= 0) continue
    if (
      windowX < bounds.x ||
      windowX >= bounds.x + bounds.width ||
      windowY < bounds.y ||
      windowY >= bounds.y + bounds.height
    ) {
      continue
    }
    const cssWidth = boundEffectivePageContentSize(page).width
    const cssScale = cssWidth > 0 ? cssWidth / bounds.width : 1
    return {
      pageId: page.id,
      localX: Math.round((windowX - bounds.x) * cssScale),
      localY: Math.round((windowY - bounds.y) * cssScale),
    }
  }
  return null
}

/** Inverse of `canvasToScreenPoint` — used for canvas-anchor fallback. */
export function windowPointToCanvasPoint(
  windowX: number,
  windowY: number,
): { x: number; y: number } {
  const origin = boundCanvasOrigin()
  return {
    x: (windowX - origin.x - pan.x) / zoom,
    y: (windowY - origin.y - pan.y) / zoom,
  }
}
