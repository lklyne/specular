import { describe, expect, it } from 'vitest'
import {
  computeScreenBoundsForPage,
  pageBodyCanvasBounds,
  pageSnapBounds,
  pageVisualBounds,
  pageVisualBoundsForContentSize,
} from '../../src/main/runtime/runtime-geometry'
import type { Page } from '../../src/main/runtime/runtime-entities'

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

  it('visual bounds hug the snap rect (no chrome band above)', () => {
    const page = unframedPage()
    expect(pageVisualBounds(page)).toEqual(pageSnapBounds(page))
  })

  it('visual bounds include the device shell for framed pages', () => {
    const page = framedPage()
    expect(pageVisualBounds(page)).toEqual({
      x: 100,
      y: 200,
      width: 375 + 24,
      height: 667 + 24,
    })
  })

  it('visual bounds can be computed from an effective focus presentation size', () => {
    const page = framedPage()
    expect(pageVisualBoundsForContentSize(page, { width: 500, height: 300 })).toEqual({
      x: 100,
      y: 200,
      width: 500 + 24,
      height: 300 + 24,
    })
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

function computeBounds() {
  return computeScreenBoundsForPage({
    page: screenBoundsPage(),
    effectivePageContentSize: () => ({ width: 375, height: 667 }),
    zoom: 1,
    pan: { x: 0, y: 0 },
    toolbarHeight: 0,
    cardBorderWidth: 1,
  })
}

describe('computeScreenBoundsForPage: device shell tracks the content rect', () => {
  it('canvas mode: shell wraps the content, offset outward by the bezel insets', () => {
    const { shell, page } = computeBounds()
    expect(shell.x).toBe(page.x - SHELL_INSET)
    expect(shell.y).toBe(page.y - SHELL_INSET)
    expect(shell.width).toBe(page.width + 2 * SHELL_INSET)
    expect(shell.height).toBe(page.height + 2 * SHELL_INSET)
  })

  it('does not recenter page bounds for a separate browser presentation', () => {
    const { shell, page } = computeBounds()
    expect(page.x).toBe(100 + SHELL_INSET)
    expect(shell.x).toBe(page.x - SHELL_INSET)
    expect(shell.y).toBe(page.y - SHELL_INSET)
    expect(shell.width).toBe(page.width + 2 * SHELL_INSET)
    expect(shell.height).toBe(page.height + 2 * SHELL_INSET)
  })
})
