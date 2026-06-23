import { screen } from 'electron'
import type { WebContents } from 'electron'
import type { WorkspaceBounds } from '../../shared/types'
import type { Page } from './runtime-entities'
import {
  BROWSER_HEADER_HEIGHT,
  CARD_BORDER_WIDTH,
  CHROME_HEADER_HEIGHT,
  CHROME_PAGE_GAP,
  LEFT_SIDEBAR_WIDTH,
  devtoolsPanelDebug,
} from './runtime-constants'
import {
  pageCustomSizeFromMetadata,
  pageBrowserSizeModeFromMetadata,
  deviceIdFromMetadata,
  deviceOrientationFromMetadata,
  showDeviceFrameFromMetadata,
} from './runtime-entities'
import { CUSTOM_SHELL_INSETS, shellInsetsForDevice, sizeForOrientation } from '../../shared/device-catalog'
import { win } from './view-refs'
import { layoutCache } from './layout-cache'
import { pages, pan, zoom } from './runtime-context'
import {
  devtoolsOpen as uiDevtoolsOpen,
  devtoolsWidth as uiDevtoolsWidth,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedPageIndex as uiSelectedPageIndex,
  workspaceViewMode as uiWorkspaceViewMode,
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
 * This is the rect that alignment guides and grid snap should use. Chrome
 * lives above it (see `pageVisualBounds`); the body sits inside it (see
 * `pageBodyCanvasBounds`) — offset by the bezel insets when framed.
 */
export function pageSnapBounds(
  page: Pick<Page, 'presetIndex' | 'canvasX' | 'canvasY' | 'peekWidth' | 'peekHeight' | 'metadata'>,
): WorkspaceBounds {
  const size = pageContentSize(page)
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
 * Visual bounds = snap rect extended upward by the chrome strip. Used for
 * selection outlines that should wrap chrome and for placement claims.
 */
export function pageVisualBounds(
  page: Pick<Page, 'presetIndex' | 'canvasX' | 'canvasY' | 'peekWidth' | 'peekHeight' | 'metadata'>,
): WorkspaceBounds {
  const snap = pageSnapBounds(page)
  return {
    x: snap.x,
    y: snap.y - CHROME_HEADER_HEIGHT,
    width: snap.width,
    height: snap.height + CHROME_HEADER_HEIGHT,
  }
}

/** Chrome band that floats above the snap rect. */
export function frameChromeCanvasBounds(
  page: Pick<Page, 'presetIndex' | 'canvasX' | 'canvasY' | 'peekWidth' | 'peekHeight' | 'metadata'>,
): WorkspaceBounds {
  const snap = pageSnapBounds(page)
  return {
    x: snap.x,
    y: snap.y - CHROME_HEADER_HEIGHT,
    width: snap.width,
    height: CHROME_HEADER_HEIGHT,
  }
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
  currentViewMode: () => string
  toolbarHeight: number
  browserHeaderHeight: number
}): { x: number; y: number } {
  const viewMode = input.currentViewMode()
  return {
    x: 0,
    y: input.toolbarHeight + (viewMode === 'browser' ? input.browserHeaderHeight : 0),
  }
}

export function computeAvailableCanvasViewport(input: {
  win: { getBounds(): { width: number; height: number } } | null
  currentViewMode: () => string
  currentDevtoolsOpen: () => boolean
  currentDevtoolsWidth: () => number
  toolbarHeight: number
  browserHeaderHeight: number
  leftSidebarWidth: number
}): { width: number; height: number } {
  const viewport = computeAvailableCanvasViewportRect(input)
  return { width: viewport.width, height: viewport.height }
}

export function computeAvailableCanvasViewportRect(input: {
  win: { getBounds(): { width: number; height: number } } | null
  currentViewMode: () => string
  currentDevtoolsOpen: () => boolean
  currentDevtoolsWidth: () => number
  toolbarHeight: number
  browserHeaderHeight: number
  leftSidebarWidth: number
}): { x: number; y: number; width: number; height: number } {
  const { width = 0, height = 0 } = input.win?.getBounds() ?? {}
  const leftInset = input.leftSidebarWidth
  const topInset =
    input.toolbarHeight + (input.currentViewMode() === 'browser' ? input.browserHeaderHeight : 0)
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

function computeFillBrowserViewportSize(input: {
  availableCanvasViewport: () => { width: number; height: number }
}): { width: number; height: number } {
  const viewport = input.availableCanvasViewport()
  return {
    width: Math.max(0, Math.round(viewport.width)),
    height: Math.max(0, Math.round(viewport.height)),
  }
}

export function computeIsFillBrowserPage(input: {
  page: Pick<Page, 'id' | 'metadata'>
  currentViewMode: () => string
  selectedPageId: () => string | null
}): boolean {
  return (
    input.currentViewMode() === 'browser' &&
    input.selectedPageId() === input.page.id &&
    pageBrowserSizeModeFromMetadata(input.page.metadata) === 'fill'
  )
}

export function computeEffectivePageContentSize(input: {
  page: Pick<Page, 'id' | 'presetIndex' | 'peekWidth' | 'peekHeight' | 'metadata'>
  isFillBrowserPage: (page: Pick<Page, 'id' | 'metadata'>) => boolean
  fillBrowserViewportSize: () => { width: number; height: number }
}): { width: number; height: number } {
  if (input.isFillBrowserPage(input.page)) {
    return input.fillBrowserViewportSize()
  }
  return pageContentSize(input.page)
}

export function computeScreenBoundsForPage(input: {
  page: Page
  effectivePageContentSize: (page: Pick<Page, 'id' | 'presetIndex' | 'peekWidth' | 'peekHeight' | 'metadata'>) => { width: number; height: number }
  availableCanvasViewportRect: () => { x: number; y: number; width: number; height: number }
  currentViewMode: () => string
  selectedPageId: () => string | null
  isFillBrowserPage: (page: Pick<Page, 'id' | 'metadata'>) => boolean
  zoom: number
  pan: { x: number; y: number }
  toolbarHeight: number
  browserHeaderHeight: number
  chromePageGap: number
  cardBorderWidth: number
}): {
  frame: { x: number; y: number; width: number; height: number }
  chrome: { x: number; y: number; width: number; height: number }
  page: { x: number; y: number; width: number; height: number }
  shell: { x: number; y: number; width: number; height: number }
} {
  const { width: w, height: h } = input.effectivePageContentSize(input.page)
  const bw = input.cardBorderWidth
  const isBrowserActive =
    input.currentViewMode() === 'browser' && input.selectedPageId() === input.page.id
  const isFillBrowserActive = input.isFillBrowserPage(input.page)
  const displayZoom = isFillBrowserActive ? 1 : input.zoom
  const chromeH = Math.round(CHROME_HEADER_HEIGHT * input.zoom)
  const contentW = Math.round(w * displayZoom)
  const fullPageH = Math.round(h * displayZoom)
  const viewport = input.availableCanvasViewportRect()
  const browserViewportTop = input.toolbarHeight + input.browserHeaderHeight
  const browserViewportHeight = viewport.height
  const maxBrowserPageH = Math.max(0, browserViewportHeight)
  const pageH = isFillBrowserActive
    ? maxBrowserPageH
    : isBrowserActive
      ? Math.min(fullPageH, maxBrowserPageH)
      : fullPageH
  const insets = pageShellInsets(input.page)
  const insetLeft = Math.round((insets?.left ?? 0) * displayZoom)
  const insetTop = Math.round((insets?.top ?? 0) * displayZoom)
  const insetRight = Math.round((insets?.right ?? 0) * displayZoom)
  const insetBottom = Math.round((insets?.bottom ?? 0) * displayZoom)

  // `snapTopScreenY` is the snap-rect top in screen space: the bezel top
  // when framed, body top when not. Body lives at snapTopScreenY + insetTop,
  // chrome floats above at snapTopScreenY - chromeH.
  const snapTopScreenY =
    Math.round(input.page.canvasY * input.zoom + input.pan.y) + input.toolbarHeight
  const snapLeftScreenX = Math.round(input.page.canvasX * input.zoom + input.pan.x)

  const rawChromeX = isBrowserActive
    ? isFillBrowserActive
      ? viewport.x
      : Math.round(viewport.x + (viewport.width - w * input.zoom) / 2)
    : snapLeftScreenX + insetLeft
  const browserMinPageY = browserViewportTop
  const centeredBrowserPageY = Math.round(
    browserViewportTop + (browserViewportHeight - pageH) / 2,
  )
  const pageY = isBrowserActive
    ? fullPageH >= maxBrowserPageH
      ? browserMinPageY
      : Math.max(browserMinPageY, centeredBrowserPageY)
    : snapTopScreenY + insetTop
  const chromeY = isBrowserActive ? browserViewportTop : snapTopScreenY - chromeH
  // Shell rect (device page bezel) wraps the content rect, offset outward by
  // the bezel insets — skip in fill-browser mode. Anchoring off the content
  // rect (rawChromeX/pageY) rather than the snap rect keeps the bezel locked
  // to the page in every mode: in canvas mode rawChromeX === snapLeftScreenX +
  // insetLeft and pageY === snapTopScreenY + insetTop, so this is identical to
  // the snap-anchored value; in browser mode the content is centered/fitted
  // (rawChromeX/pageY follow the browser viewport), and the bezel must follow
  // it instead of staying at the canvas position.
  const shellRect = insets && !isFillBrowserActive
    ? {
        x: rawChromeX - insetLeft,
        y: pageY - insetTop,
        width: contentW + insetLeft + insetRight,
        height: pageH + insetTop + insetBottom,
      }
    : {
        x: rawChromeX - bw,
        y: pageY - bw,
        width: contentW + 2 * bw,
        height: pageH + 2 * bw,
      }

  return {
    frame: {
      x: rawChromeX - bw,
      y: pageY - bw,
      width: contentW + 2 * bw,
      height: pageH + 2 * bw,
    },
    chrome: {
      x: rawChromeX,
      y: chromeY,
      width: contentW,
      height: chromeH,
    },
    page: {
      x: rawChromeX,
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
  isFillBrowserPage: (page: Pick<Page, 'id' | 'metadata'>) => boolean
  viewportPresetForIndex: (presetIndex: number) => { width: number; height: number }
}): void {
  const start = Date.now()
  const vp = input.viewportPresetForIndex(input.presetIndex)
  const nativeScale = screen.getPrimaryDisplay().scaleFactor
  const fillScale = input.page && input.isFillBrowserPage(input.page) ? 1 : input.zoom
  const size = input.page
    ? input.effectivePageContentSize(input.page)
    : { width: vp.width, height: vp.height }
  input.webContents.enableDeviceEmulation({
    screenPosition: 'desktop',
    screenSize: { width: size.width, height: size.height },
    viewSize: { width: size.width, height: size.height },
    viewPosition: { x: 0, y: 0 },
    deviceScaleFactor: nativeScale,
    scale: fillScale,
  })
  devtoolsPanelDebug('geometry:enable-device-emulation', {
    pageId: input.page?.id ?? null,
    durationMs: Date.now() - start,
    width: size.width,
    height: size.height,
    fillScale,
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

export function boundIsFillBrowserPage(page: Pick<Page, 'id' | 'metadata'>): boolean {
  return computeIsFillBrowserPage({
    page,
    currentViewMode: uiWorkspaceViewMode,
    selectedPageId: boundSelectedPageId,
  })
}

export function boundAvailableCanvasViewport(): { width: number; height: number } {
  return computeAvailableCanvasViewport({
    win,
    currentViewMode: uiWorkspaceViewMode,
    currentDevtoolsOpen: uiDevtoolsOpen,
    currentDevtoolsWidth: uiDevtoolsWidth,
    toolbarHeight: layoutCache.toolbarHeight,
    browserHeaderHeight: BROWSER_HEADER_HEIGHT,
    leftSidebarWidth: uiLeftSidebarOpen() ? LEFT_SIDEBAR_WIDTH : 0,
  })
}

export function boundAvailableCanvasViewportRect(): { x: number; y: number; width: number; height: number } {
  return computeAvailableCanvasViewportRect({
    win,
    currentViewMode: uiWorkspaceViewMode,
    currentDevtoolsOpen: uiDevtoolsOpen,
    currentDevtoolsWidth: uiDevtoolsWidth,
    toolbarHeight: layoutCache.toolbarHeight,
    browserHeaderHeight: BROWSER_HEADER_HEIGHT,
    leftSidebarWidth: uiLeftSidebarOpen() ? LEFT_SIDEBAR_WIDTH : 0,
  })
}

export function boundFillBrowserViewportSize(): { width: number; height: number } {
  return computeFillBrowserViewportSize({
    availableCanvasViewport: boundAvailableCanvasViewport,
  })
}

export function boundEffectivePageContentSize(
  page: Pick<Page, 'id' | 'presetIndex' | 'peekWidth' | 'peekHeight' | 'metadata'>,
): { width: number; height: number } {
  return computeEffectivePageContentSize({
    page,
    isFillBrowserPage: boundIsFillBrowserPage,
    fillBrowserViewportSize: boundFillBrowserViewportSize,
  })
}

export function boundCanvasOrigin(): { x: number; y: number } {
  return computeCanvasOrigin({
    currentViewMode: uiWorkspaceViewMode,
    toolbarHeight: layoutCache.toolbarHeight,
    browserHeaderHeight: BROWSER_HEADER_HEIGHT,
  })
}

export function boundCanvasOriginX(): number {
  return boundCanvasOrigin().x
}

export function boundScreenBoundsForPage(page: Page) {
  return computeScreenBoundsForPage({
    page,
    effectivePageContentSize: boundEffectivePageContentSize,
    availableCanvasViewportRect: boundAvailableCanvasViewportRect,
    currentViewMode: uiWorkspaceViewMode,
    selectedPageId: boundSelectedPageId,
    isFillBrowserPage: boundIsFillBrowserPage,
    zoom,
    pan,
    toolbarHeight: layoutCache.toolbarHeight,
    browserHeaderHeight: BROWSER_HEADER_HEIGHT,
    chromePageGap: CHROME_PAGE_GAP,
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
    isFillBrowserPage: boundIsFillBrowserPage,
    viewportPresetForIndex,
  })
}
