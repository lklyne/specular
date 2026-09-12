/**
 * Page content-bounds hit-test for aboveView input forwarding.
 *
 * A page's web content can render inset from its entity body (device frame,
 * chrome), so its content bounds carry their own screen-space fields. When
 * those are absent the content falls back to the body bounds.
 *
 * Pure: no Electron, no DOM. Lives in src/shared so the coord math is
 * single-sourced (interaction-layer.md §6, I9) across the wheel router and the
 * pointer-forwarding hook that both decide whether a pointer is over a page.
 */

import type { ProjectedPageEntity } from './scene-projection'
import { rectContains, type Point, type Rect } from './hit-regions'
import type { PageDragPayload } from './types'

/** Screen-space content rect of a page, falling back to the body bounds. */
export function pageContentRect(page: ProjectedPageEntity): Rect {
  return {
    x: page.contentScreenX ?? page.screenX,
    y: page.contentScreenY ?? page.screenY,
    width: page.contentScreenWidth ?? page.screenWidth,
    height: page.contentScreenHeight ?? page.screenHeight,
  }
}

/**
 * Whether a screen-space point lies within a page's content bounds. The point
 * is window-relative (y already offset by `canvasOrigin.y`), matching
 * `entity.screenX/screenY`. Edges are inclusive.
 */
export function pointerOverPageContent(page: ProjectedPageEntity, point: Point): boolean {
  return rectContains(pageContentRect(page), point)
}

export type PageDragDropOutcome = 'drop-on-canvas' | 'release-in-page' | 'none'

/**
 * Where a page drag-out gesture (ADR 0038) lands on release: still inside
 * the source page's own content — its native drag-and-drop semantics would
 * apply there, which is lost once the page is an offscreen texture — or past
 * it onto the canvas, where the armed payload becomes a new entity.
 */
export function decidePageDragOutcome(
  armedPayload: PageDragPayload | null,
  releasePoint: Point,
  page: ProjectedPageEntity | null,
): PageDragDropOutcome {
  if (!armedPayload) return 'none'
  if (page && pointerOverPageContent(page, releasePoint)) return 'release-in-page'
  return 'drop-on-canvas'
}
