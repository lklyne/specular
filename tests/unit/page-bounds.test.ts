import { describe, expect, it } from 'vitest'
import {
  computeScreenBoundsForPage,
  pageBodyCanvasBounds,
  pageSnapBounds,
  pageVisualBounds,
} from '../../src/main/runtime/runtime-geometry'
import type { Page } from '../../src/main/runtime/runtime-entities'
import { CHROME_HEADER_HEIGHT } from '../../src/shared/entity-chrome-slots'

type PageStub = Parameters<typeof pageSnapBounds>[0]

function unframedPage(overrides: Partial<PageStub> = {}): PageStub {
  // presetIndex 0 = iPhone SE viewport 375×667 in the catalog.
  return {
    presetIndex: 0,
    canvasX: 100,
    canvasY: 200,
    peekWidth: 375,
    peekHeight: 667,
    metadata: undefined,
    ...overrides,
  }
}

function framedPage(overrides: Partial<PageStub> = {}): PageStub {
  // Force the custom-shell path (CUSTOM_SHELL_INSETS = 12px all around) by
  // turning on the frame metadata without a real deviceId.
  return unframedPage({
    metadata: { showDeviceFrame: true },
    ...overrides,
  })
}

describe('page bounds (Path A semantics)', () => {
  it('unframed page: snap rect == body rect, anchored at canvasY', () => {
    const page = unframedPage()
    expect(pageSnapBounds(page)).toEqual({ x: 100, y: 200, width: 375, height: 667 })
    expect(pageBodyCanvasBounds(page)).toEqual({ x: 100, y: 200, width: 375, height: 667 })
  })

  it('framed page: snap rect grows by insets; body is offset inward', () => {
    const page = framedPage()
    // CUSTOM_SHELL_INSETS = { top: 12, right: 12, bottom: 12, left: 12 }
    expect(pageSnapBounds(page)).toEqual({
      x: 100,
      y: 200,
      width: 375 + 24,
      height: 667 + 24,
    })
    expect(pageBodyCanvasBounds(page)).toEqual({
      x: 100 + 12,
      y: 200 + 12,
      width: 375,
      height: 667,
    })
  })

  it('chrome lives above the snap rect (visual bounds extend upward)', () => {
    const page = unframedPage()
    const visual = pageVisualBounds(page)
    expect(visual.y).toBe(200 - CHROME_HEADER_HEIGHT)
    expect(visual.height).toBe(667 + CHROME_HEADER_HEIGHT)
    expect(visual.x).toBe(100)
    expect(visual.width).toBe(375)
  })

  it('toggling a frame on keeps canvasY stable and pushes body down', () => {
    const unframed = unframedPage()
    const framed = framedPage()
    // Snap-rect top is anchored.
    expect(pageSnapBounds(framed).y).toBe(pageSnapBounds(unframed).y)
    // Body moves down to make room for the bezel.
    expect(pageBodyCanvasBounds(framed).y).toBeGreaterThan(pageBodyCanvasBounds(unframed).y)
    expect(pageBodyCanvasBounds(framed).y - pageBodyCanvasBounds(unframed).y).toBe(12)
  })
})

// CUSTOM_SHELL_INSETS = 12px all around (framed page without a real deviceId).
const SHELL_INSET = 12

function screenBoundsPage(): Page {
  return {
    id: 'page_test',
    canvasX: 100,
    canvasY: 200,
    presetIndex: 0,
    peekWidth: 375,
    peekHeight: 667,
    metadata: { showDeviceFrame: true },
  } as unknown as Page
}

function computeBounds(opts: { viewMode: 'canvas' | 'browser' }) {
  return computeScreenBoundsForPage({
    page: screenBoundsPage(),
    effectivePageContentSize: () => ({ width: 375, height: 667 }),
    availableCanvasViewportRect: () => ({ x: 0, y: 0, width: 1000, height: 1000 }),
    currentViewMode: () => opts.viewMode,
    // In browser mode the test page is the selected/active page.
    selectedPageId: () => (opts.viewMode === 'browser' ? 'page_test' : null),
    isFillBrowserPage: () => false,
    zoom: 1,
    pan: { x: 0, y: 0 },
    toolbarHeight: 0,
    browserHeaderHeight: 0,
    chromePageGap: 0,
    cardBorderWidth: 1,
  })
}

describe('computeScreenBoundsForPage: device shell tracks the content rect', () => {
  it('canvas mode: shell wraps the content, offset outward by the bezel insets', () => {
    const { shell, page } = computeBounds({ viewMode: 'canvas' })
    expect(shell.x).toBe(page.x - SHELL_INSET)
    expect(shell.y).toBe(page.y - SHELL_INSET)
    expect(shell.width).toBe(page.width + 2 * SHELL_INSET)
    expect(shell.height).toBe(page.height + 2 * SHELL_INSET)
  })

  it('browser mode (non-fill): shell follows the centered content, not the canvas position', () => {
    const { shell, page } = computeBounds({ viewMode: 'browser' })
    // Content is browser-centered: x = (1000 - 375) / 2 = 312.5 -> 313.
    expect(page.x).toBe(313)
    // Regression: the bezel must wrap the *centered* content. Before the fix the
    // shell was anchored at snapLeftScreenX/snapTopScreenY (the canvas position,
    // x = 100), leaving the grey card stranded away from the live page.
    expect(shell.x).toBe(page.x - SHELL_INSET)
    expect(shell.y).toBe(page.y - SHELL_INSET)
    expect(shell.width).toBe(page.width + 2 * SHELL_INSET)
    expect(shell.height).toBe(page.height + 2 * SHELL_INSET)
  })
})
