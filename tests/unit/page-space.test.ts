// Protects the single page-viewport→screen transform every page-content
// overlay (comment badges, pending composers, thread popovers, inspect
// popovers) maps through.
//
// Mutation-verified by doubling the scale denominator in pageViewportToScreen
// (`frameWidth / page.width` → `frameWidth / (page.width * 2)`): this file,
// tests/unit/annotation-live-bbox.test.ts, and
// tests/unit/inspect-popover-position.test.ts all fail.
import { describe, expect, it } from 'vitest'
import { pageViewportToScreen, type PageScreenFrame } from '../../src/shared/page-space'

const ORIGIN = { canvasOrigin: { x: 0, y: 50 } }

function page(overrides: Partial<PageScreenFrame> = {}): PageScreenFrame {
  return {
    width: 400,
    height: 300,
    screenX: 200,
    screenY: 100,
    screenWidth: 400,
    screenHeight: 300,
    ...overrides,
  }
}

describe('pageViewportToScreen', () => {
  it('falls back to the outer screen bounds when content bounds are absent', () => {
    const rect = pageViewportToScreen({ x: 50, y: 80, width: 100, height: 40 }, page(), ORIGIN)
    expect(rect).toEqual({ left: 250, top: 130, width: 100, height: 40 })
  })

  it('maps through the inner content frame (scaled) when present', () => {
    const devicePage = page({
      contentScreenX: 230,
      contentScreenY: 140,
      contentScreenWidth: 200,
      contentScreenHeight: 150,
    })
    // scale is content/viewport = 0.5 on both axes
    const rect = pageViewportToScreen({ x: 50, y: 40, width: 100, height: 20 }, devicePage, ORIGIN)
    expect(rect).toEqual({ left: 255, top: 110, width: 50, height: 10 })
  })

  it("the 'entity' frame ignores content bounds and maps through the outer bounds", () => {
    const devicePage = page({
      contentScreenX: 230,
      contentScreenY: 140,
      contentScreenWidth: 200,
      contentScreenHeight: 150,
    })
    const rect = pageViewportToScreen(
      { x: 50, y: 40, width: 100, height: 20 },
      devicePage,
      ORIGIN,
      'entity',
    )
    expect(rect).toEqual({ left: 250, top: 90, width: 100, height: 20 })
  })

  it('shifts only the y axis by the canvas origin (overlay coordinates)', () => {
    const base = pageViewportToScreen(
      { x: 50, y: 80, width: 100, height: 40 },
      page(),
      { canvasOrigin: { x: 0, y: 0 } },
    )
    const offset = pageViewportToScreen(
      { x: 50, y: 80, width: 100, height: 40 },
      page(),
      { canvasOrigin: { x: 123, y: 30 } },
    )
    expect(offset.left).toBe(base.left)
    expect(offset.top).toBe(base.top - 30)
    expect(offset.width).toBe(base.width)
    expect(offset.height).toBe(base.height)
  })

  it('treats a degenerate zero-size page viewport as unscaled', () => {
    const rect = pageViewportToScreen(
      { x: 10, y: 20, width: 30, height: 40 },
      page({ width: 0, height: 0 }),
      ORIGIN,
    )
    expect(rect).toEqual({ left: 210, top: 70, width: 30, height: 40 })
  })
})
