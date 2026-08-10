import { screen } from 'electron'
import type { WebContents } from 'electron'
import type { WorkspaceBounds } from '../../shared/types'
import type { Page } from './runtime-entities'
import {
  CARD_BORDER_WIDTH,
  LEFT_SIDEBAR_WIDTH,
  TOOLBAR_HEIGHT,
  devtoolsPanelDebug,
} from './runtime-constants'
import {
  pageCustomSizeFromMetadata,
  deviceIdFromMetadata,
  deviceOrientationFromMetadata,
  showDeviceFrameFromMetadata,
} from './runtime-entities'
import { CUSTOM_SHELL_INSETS, shellInsetsForDevice, sizeForOrientation } from '../../shared/device-catalog'
import { win } from './view-refs'
import { layoutCache } from './layout-cache'
import { pages, pan, zoom } from './runtime-context'
import { focusSession } from './focus-session'
import {
  devtoolsOpen as uiDevtoolsOpen,
  devtoolsWidth as uiDevtoolsWidth,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedPageIndex as uiSelectedPageIndex,
} from '../ui-state'
import { viewportPresetForIndex } from './runtime-serialization'

type Bounds = {
  x: number
  y: number
  width: number
  height: number
}

// ---------------------------------------------------------------------------
// Pure utility functions
// ---------------------------------------------------------------------------

export function boundsKey(bounds: Bounds): string {
  return `${bounds.x}:${bounds.y}:${bounds.width}:${bounds.height}`
}

export function boundsOverlap(a: Bounds, b: Bounds): boolean {
  return (
    a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y
  )
}

export function pageContentSize(page: Pick<Page, 'presetIndex' | 'peekWidth' | 'peekHeight' | 'metadata'>): {
  width: number
  height: number
} {
  const vp = viewportPresetForIndex(page.presetIndex)
  const customSize = pageCustomSizeFromMetadata(page.metadata)
  const baseW = page.peekWidth ?? customSize?.width ?? vp.width
  const baseH = page.peekHeight ?? customSize?.height ?? vp.height
  if (customSize || page.peekWidth) return { width: baseW, height: baseH }
  return sizeForOrientation(baseW, baseH, deviceOrientationFromMetadata(page.metadata))
}

export function pageShellInsets(
  page: Pick<Page, 'metadata'>,
): { top: number; right: number; bottom: number; left: number } | null {
  const show = showDeviceFrameFromMetadata(page.metadata)
  if (!show) return null
  const deviceId = deviceIdFromMetadata(page.metadata)
  if (!deviceId) return CUSTOM_SHELL_INSETS
  const orientation = deviceOrientationFromMetadata(page.metadata)
  return shellInsetsForDevice(deviceId, orientation)
}

/**
 * Snap rect = body + device-frame insets, anchored at `canvasY`.
 *
 *   unframed: { x: canvasX,             y: canvasY,             w: bodyW, h: bodyH }
 *   framed:   { x: canvasX,             y: canvasY,             w: bodyW + lr, h: bodyH + tb }
 *
 * This is the rect that alignment guides and grid snap should use. The body
 * sits inside it (see `pageBodyCanvasBounds`) — offset by the bezel insets
 * when framed.
 */
export function pageSnapBounds(
  page: Pick<Page, 'presetIndex' | 'canvasX' | 'canvasY' | 'peekWidth' | 'peekHeight' | 'metadata'>,
): WorkspaceBounds {
  return pageSnapBoundsForContentSize(page, pageContentSize(page))
}

export function pageSnapBoundsForContentSize(
  page: Pick<Page, 'canvasX' | 'canvasY' | 'metadata'>,
  size: { width: number; height: number },
): WorkspaceBounds {
  const insets = pageShellInsets(page)
  if (!insets) {
    return { x: page.canvasX, y: page.canvasY, width: size.width, height: size.height }
  }
  return {
    x: page.canvasX,
    y: page.canvasY,
    width: size.width + insets.left + insets.right,
    height: size.height + insets.top + insets.bottom,
  }
}

/**
 * Body bounds = the webview content area, inside the bezel when framed.
 *
 *   unframed: body == snap rect
 *   framed:   body is offset right/down by (insets.left, insets.top)
 */
export function pageBodyCanvasBounds(
  page: Pick<Page, 'presetIndex' | 'canvasX' | 'canvasY' | 'peekWidth' | 'peekHeight' | 'metadata'>,
): WorkspaceBounds {
  const size = pageContentSize(page)
  const insets = pageShellInsets(page)
  return {
    x: page.canvasX + (insets?.left ?? 0),
    y: page.canvasY + (insets?.top ?? 0),
    width: size.width,
    height: size.height,
  }
}

/**
 * Visual bounds = the snap rect. Used for selection outlines and placement
 * claims. Pages have no chrome band, so visual bounds hug the body.
 */
export function pageVisualBounds(
  page: Pick<Page, 'presetIndex' | 'canvasX' | 'canvasY' | 'peekWidth' | 'peekHeight' | 'metadata'>,
): WorkspaceBounds {
  return pageVisualBoundsForContentSize(page, pageContentSize(page))
}

export function pageVisualBoundsForContentSize(
  page: Pick<Page, 'canvasX' | 'canvasY' | 'metadata'>,
  size: { width: number; height: number },
): WorkspaceBounds {
  return pageSnapBoundsForContentSize(page, size)
}

/**
 * Project a content-space DOM point (from a page iframe) into canvas coordinates.
 * Accounts for shell insets so callers never need to add chromeHeight manually.
 */
export function projectFramePointToCanvas(
  page: Pick<Page, 'presetIndex' | 'canvasX' | 'canvasY' | 'peekWidth' | 'peekHeight' | 'metadata'>,
  point: { x: number; y: number },
): { x: number; y: number } {
  const body = pageBodyCanvasBounds(page)
  return { x: body.x + point.x, y: body.y + point.y }
}

// ---------------------------------------------------------------------------
// Pure computation functions (parameterized — no runtime state)
// ---------------------------------------------------------------------------

export function computeCanvasOrigin(input: {
  toolbarHeight: number
}): { x: number; y: number } {
  return {
    x: 0,
    y: input.toolbarHeight,
  }
}

export function computeAvailableCanvasViewport(input: {
  win: { getBounds(): { width: number; height: number } } | null
  currentDevtoolsOpen: () => boolean
  currentDevtoolsWidth: () => number
  toolbarHeight: number
  leftSidebarWidth: number
}): { width: number; height: number } {
  const viewport = computeAvailableCanvasViewportRect(input)
  return { width: viewport.width, height: viewport.height }
}

export function computeAvailableCanvasViewportRect(input: {
  win: { getBounds(): { width: number; height: number } } | null
  currentDevtoolsOpen: () => boolean
  currentDevtoolsWidth: () => number
  toolbarHeight: number
  leftSidebarWidth: number
}): { x: number; y: number; width: number; height: number } {
  const { width = 0, height = 0 } = input.win?.getBounds() ?? {}
  const leftInset = input.leftSidebarWidth
  const topInset = input.toolbarHeight
  return {
    x: leftInset,
    y: topInset,
    width: Math.max(
      0,
      width - (input.currentDevtoolsOpen() ? input.currentDevtoolsWidth() : 0) - leftInset,
    ),
    height: Math.max(0, height - topInset),
  }
}

export function computeEffectivePageContentSize(input: {
  page: Pick<Page, 'presetIndex' | 'peekWidth' | 'peekHeight' | 'metadata'>
}): { width: number; height: number } {
  return pageContentSize(input.page)
}

export function computeScreenBoundsForPage(input: {
  page: Page
  effectivePageContentSize: (page: Pick<Page, 'presetIndex' | 'peekWidth' | 'peekHeight' | 'metadata'>) => { width: number; height: number }
  zoom: number
  pan: { x: number; y: number }
  toolbarHeight: number
  cardBorderWidth: number
}): {
  frame: { x: number; y: number; width: number; height: number }
  page: { x: number; y: number; width: number; height: number }
  shell: { x: number; y: number; width: number; height: number }
} {
  const { width: w, height: h } = input.effectivePageContentSize(input.page)
  const bw = input.cardBorderWidth
  const displayZoom = input.zoom
  const contentW = Math.round(w * displayZoom)
  const fullPageH = Math.round(h * displayZoom)
  const pageH = fullPageH
  const insets = pageShellInsets(input.page)
  const insetLeft = Math.round((insets?.left ?? 0) * displayZoom)
  const insetTop = Math.round((insets?.top ?? 0) * displayZoom)
  const insetRight = Math.round((insets?.right ?? 0) * displayZoom)
  const insetBottom = Math.round((insets?.bottom ?? 0) * displayZoom)

  // `snapTopScreenY` is the snap-rect top in screen space: the bezel top
  // when framed, body top when not. Body lives at snapTopScreenY + insetTop.
  const snapTopScreenY =
    Math.round(input.page.canvasY * input.zoom + input.pan.y) + input.toolbarHeight
  const snapLeftScreenX = Math.round(input.page.canvasX * input.zoom + input.pan.x)

  const pageX = snapLeftScreenX + insetLeft
  const pageY = snapTopScreenY + insetTop
  // Shell rect (device page bezel) wraps the content rect, offset outward by
  // the bezel insets. Anchoring off the content rect keeps the bezel locked
  // to the page body.
  const shellRect = insets
    ? {
        x: pageX - insetLeft,
        y: pageY - insetTop,
        width: contentW + insetLeft + insetRight,
        height: pageH + insetTop + insetBottom,
      }
    : {
        x: pageX - bw,
        y: pageY - bw,
        width: contentW + 2 * bw,
        height: pageH + 2 * bw,
      }

  return {
    frame: {
      x: pageX - bw,
      y: pageY - bw,
      width: contentW + 2 * bw,
      height: pageH + 2 * bw,
    },
    page: {
      x: pageX,
      y: pageY,
      width: contentW,
      height: pageH,
    },
    shell: shellRect,
  }
}

export function computeApplyEmulation(input: {
  webContents: WebContents
  presetIndex: number
  page?: Page
  zoom: number
  effectivePageContentSize: (page: Pick<Page, 'id' | 'presetIndex' | 'peekWidth' | 'peekHeight' | 'metadata'>) => { width: number; height: number }
  viewportPresetForIndex: (presetIndex: number) => { width: number; height: number }
}): void {
  const start = Date.now()
  const vp = input.viewportPresetForIndex(input.presetIndex)
  const nativeScale = screen.getPrimaryDisplay().scaleFactor
  const pageScale = input.zoom
  const size = input.page
    ? input.effectivePageContentSize(input.page)
    : { width: vp.width, height: vp.height }
  input.webContents.enableDeviceEmulation({
    screenPosition: 'desktop',
    screenSize: { width: size.width, height: size.height },
    viewSize: { width: size.width, height: size.height },
    viewPosition: { x: 0, y: 0 },
    deviceScaleFactor: nativeScale,
    scale: pageScale,
  })
  devtoolsPanelDebug('geometry:enable-device-emulation', {
    pageId: input.page?.id ?? null,
    durationMs: Date.now() - start,
    width: size.width,
    height: size.height,
    pageScale,
  })
}

// ---------------------------------------------------------------------------
// Bound convenience functions (close over runtime state)
// ---------------------------------------------------------------------------

export function boundSelectedPage(): Page | null {
  const selectedIndex = uiSelectedPageIndex(pages.map((p) => p.id))
  if (selectedIndex === null || selectedIndex < 0 || selectedIndex >= pages.length) {
    return null
  }
  return pages[selectedIndex]
}

export function boundSelectedPageId(): string | null {
  const page = boundSelectedPage()
  return page?.id ?? null
}

export function boundAvailableCanvasViewport(): { width: number; height: number } {
  return computeAvailableCanvasViewport({
    win,
    currentDevtoolsOpen: uiDevtoolsOpen,
    currentDevtoolsWidth: uiDevtoolsWidth,
    toolbarHeight: layoutCache.toolbarHeight,
    leftSidebarWidth: uiLeftSidebarOpen() ? LEFT_SIDEBAR_WIDTH : 0,
  })
}

export function boundAvailableCanvasViewportRect(): { x: number; y: number; width: number; height: number } {
  return computeAvailableCanvasViewportRect({
    win,
    currentDevtoolsOpen: uiDevtoolsOpen,
    currentDevtoolsWidth: uiDevtoolsWidth,
    toolbarHeight: layoutCache.toolbarHeight,
    leftSidebarWidth: uiLeftSidebarOpen() ? LEFT_SIDEBAR_WIDTH : 0,
  })
}

/**
 * The 'fill' focus region: the canvas viewport below the flush focus chrome
 * bar (TOOLBAR_HEIGHT tall, pinned to the canvas-area top). Single source of
 * truth for both the fill page-view bounds and its native viewport size, so
 * the two can't drift.
 */
export function focusFillRegion(): { x: number; y: number; width: number; height: number } {
  const rect = boundAvailableCanvasViewportRect()
  return {
    x: rect.x,
    y: rect.y + TOOLBAR_HEIGHT,
    width: Math.round(rect.width),
    height: Math.max(1, Math.round(rect.height - TOOLBAR_HEIGHT)),
  }
}

export function boundEffectivePageContentSize(
  page: Pick<Page, 'presetIndex' | 'peekWidth' | 'peekHeight' | 'metadata'> & { id?: string },
): { width: number; height: number } {
  const focus = focusSession()
  if (page.id && focus?.target.kind === 'page' && focus.target.id === page.id) {
    const mode = focus.mode
    if (mode === 'fill') {
      const region = focusFillRegion()
      return { width: region.width, height: region.height }
    }
    if (mode === 'fit') {
      const viewport = boundAvailableCanvasViewportRect()
      return {
        width: Math.max(320, Math.round(viewport.width - 128)),
        height: Math.max(200, Math.round(viewport.height - 128)),
      }
    }
  }
  return computeEffectivePageContentSize({
    page,
  })
}

export function boundCanvasOrigin(): { x: number; y: number } {
  return computeCanvasOrigin({
    toolbarHeight: layoutCache.toolbarHeight,
  })
}

export function boundCanvasOriginX(): number {
  return boundCanvasOrigin().x
}

export function boundScreenBoundsForPage(page: Page) {
  return computeScreenBoundsForPage({
    page,
    effectivePageContentSize: boundEffectivePageContentSize,
    zoom,
    pan,
    toolbarHeight: layoutCache.toolbarHeight,
    cardBorderWidth: CARD_BORDER_WIDTH,
  })
}

export function boundApplyEmulation(webContents: WebContents, presetIndex: number, page?: Page): void {
  computeApplyEmulation({
    webContents,
    presetIndex,
    page,
    zoom,
    effectivePageContentSize: boundEffectivePageContentSize,
    viewportPresetForIndex,
  })
}
