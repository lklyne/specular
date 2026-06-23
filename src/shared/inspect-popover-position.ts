import type {
  CanvasScenePageEntity,
  DevtoolsPanelDomRect,
  InspectNodeDetail,
} from './types'

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
  page: CanvasScenePageEntity,
): InspectOverlayRect | null {
  const box = detail.boundingBox
  if (!box) return null
  return domRectToPageScreenRect(box, page)
}

export function domRectToPageScreenRect(
  rect: DevtoolsPanelDomRect,
  page: CanvasScenePageEntity,
): InspectOverlayRect {
  const contentX = page.contentScreenX ?? page.screenX
  const contentY = page.contentScreenY ?? page.screenY
  const contentWidth = page.contentScreenWidth ?? page.screenWidth
  const contentHeight = page.contentScreenHeight ?? page.screenHeight
  const scaleX = page.width > 0 ? contentWidth / page.width : 1
  const scaleY = page.height > 0 ? contentHeight / page.height : 1
  const left = contentX + rect.x * scaleX
  const top = contentY + rect.y * scaleY
  const width = rect.width * scaleX
  const height = rect.height * scaleY
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
