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
import { fileEntities } from './file-entity-state'
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
    if (!page || page.pageView.webContents.isDestroyed()) continue
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

/**
 * Returns the id of the topmost file entity whose content rect contains
 * (windowX, windowY), or null. Iterates in reverse array order so later
 * entries — painted on top — win. Uses content bounds, ignoring shell insets.
 */
export function fileAtWindowPoint(windowX: number, windowY: number): string | null {
  const origin = boundCanvasOrigin()
  for (let i = fileEntities.length - 1; i >= 0; i--) {
    const entity = fileEntities[i]
    const contentScreenX = origin.x + entity.canvasX * zoom + pan.x
    const contentScreenY = origin.y + entity.canvasY * zoom + pan.y
    const contentScreenW = entity.width * zoom
    const contentScreenH = entity.height * zoom
    if (
      windowX >= contentScreenX &&
      windowX <= contentScreenX + contentScreenW &&
      windowY >= contentScreenY &&
      windowY <= contentScreenY + contentScreenH
    ) {
      return entity.id
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
