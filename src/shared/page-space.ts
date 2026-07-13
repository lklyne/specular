import type { LayoutUpdateData } from './types'

/**
 * Page-viewport → screen mapping. A page's DOM reports element rects in page
 * CSS pixels (the web viewport's own coordinate space); the layout broadcast
 * carries where that viewport sits on screen. This module is the one home of
 * the ratio between the two — every overlay that positions chrome over page
 * content (comment badges, pending composers, thread popovers, inspect
 * popovers) maps through here. Scroll tracking (ADR 0029 follow-up) extends
 * this seam, not a call site.
 */

/** Rect in page CSS pixels — e.g. a DOM bounding box. */
export interface PageViewportRect { x: number; y: number; width: number; height: number }

/**
 * The slice of a broadcast scene entity the transform reads. Structural so
 * any scene entity qualifies; only pages (and file entities) carry the
 * optional inner content bounds (the web viewport, excluding a device shell).
 * `width`/`height` are the page-viewport size in page CSS pixels.
 */
export interface PageScreenFrame {
  width: number
  height: number
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  contentScreenX?: number
  contentScreenY?: number
  contentScreenWidth?: number
  contentScreenHeight?: number
}

/**
 * Which screen frame anchors the mapping: `'content'` (default) is the web
 * viewport; `'entity'` is the outer entity bounds, device shell included.
 * Only the annotation thread-popover positioner uses `'entity'` — its output
 * predates the content frame and is pinned by tests. Do not use it for new
 * call sites.
 */
export type PageFrameKind = 'content' | 'entity'

/** Overlay-coordinate rect: left/width in window px, top shifted by -canvasOrigin.y. */
export interface PageScreenRect { left: number; top: number; width: number; height: number }

/**
 * Map a page-viewport rect to where it renders on screen, in overlay
 * coordinates (window-space x; y offset by the canvas origin, matching
 * `toOverlayY`). Callers that work in raw window coordinates pass a zero
 * canvas origin.
 */
export function pageViewportToScreen(
  rect: PageViewportRect,
  page: PageScreenFrame,
  layout: Pick<LayoutUpdateData, 'canvasOrigin'>,
  frame: PageFrameKind = 'content',
): PageScreenRect {
  const useContent = frame === 'content'
  const originX = useContent ? page.contentScreenX ?? page.screenX : page.screenX
  const originY = useContent ? page.contentScreenY ?? page.screenY : page.screenY
  const frameWidth = useContent ? page.contentScreenWidth ?? page.screenWidth : page.screenWidth
  const frameHeight = useContent ? page.contentScreenHeight ?? page.screenHeight : page.screenHeight
  const scaleX = page.width > 0 ? frameWidth / page.width : 1
  const scaleY = page.height > 0 ? frameHeight / page.height : 1
  return {
    left: originX + rect.x * scaleX,
    top: originY + rect.y * scaleY - layout.canvasOrigin.y,
    width: rect.width * scaleX,
    height: rect.height * scaleY,
  }
}
