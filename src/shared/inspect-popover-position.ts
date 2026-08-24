import type { ProjectedPageEntity } from './scene-projection'
import { pageViewportToScreen } from './page-space'
import type { DevtoolsPanelDomRect, InspectNodeDetail } from './types'

export interface InspectOverlayRect {
  left: number
  top: number
  right: number
  bottom: number
  width: number
  height: number
}

export interface InspectPopoverSize {
  width: number
  height: number
}

export interface InspectPopoverViewport {
  width: number
  height: number
}

const VIEWPORT_PADDING = 8
const TARGET_GAP = 6

export function inspectTargetScreenRect(
  detail: Pick<InspectNodeDetail, 'boundingBox'>,
  page: ProjectedPageEntity,
): InspectOverlayRect | null {
  const box = detail.boundingBox
  if (!box) return null
  return domRectToPageScreenRect(box, page)
}

// The inspect popover works in raw window coordinates — its caller applies
// the overlay offset itself — so the transform runs with a zero canvas origin.
const WINDOW_ORIGIN = { canvasOrigin: { x: 0, y: 0 } }

export function domRectToPageScreenRect(
  rect: DevtoolsPanelDomRect,
  page: ProjectedPageEntity,
): InspectOverlayRect {
  const { left, top, width, height } = pageViewportToScreen(rect, page, WINDOW_ORIGIN)
  return {
    left,
    top,
    right: left + width,
    bottom: top + height,
    width,
    height,
  }
}

export function placeInspectPopover(
  target: InspectOverlayRect,
  popover: InspectPopoverSize,
  viewport: InspectPopoverViewport,
): { left: number; top: number } {
  const maxLeft = Math.max(
    VIEWPORT_PADDING,
    viewport.width - popover.width - VIEWPORT_PADDING,
  )
  const left = clamp(Math.round(target.left), VIEWPORT_PADDING, maxLeft)

  const aboveTop = Math.round(target.top) - popover.height - TARGET_GAP
  const belowTop = Math.round(target.bottom) + TARGET_GAP
  const maxTop = Math.max(
    VIEWPORT_PADDING,
    viewport.height - popover.height - VIEWPORT_PADDING,
  )
  const top = aboveTop >= VIEWPORT_PADDING
    ? aboveTop
    : clamp(belowTop, VIEWPORT_PADDING, maxTop)

  return { left, top }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}
