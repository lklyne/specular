// Protects the single page-viewport→screen transform every page-content
// overlay (comment badges, pending composers, thread popovers, inspect
// popovers) maps through, and the page-document→screen transform that
// composes with it for stored (document-space) anchors.
//
// Mutation-verified by doubling the scale denominator in pageViewportToScreen
// (`frameWidth / page.width` → `frameWidth / (page.width * 2)`): this file,
// tests/unit/annotation-live-bbox.test.ts, and
// tests/unit/inspect-popover-position.test.ts all fail. Separately,
// mutation-verified for pageDocumentToScreen by dropping the
// `- (page.scrollX ?? 0)` / `- (page.scrollY ?? 0)` terms: the
// `pageDocumentToScreen` describe block below fails.
import { describe, expect, it } from 'vitest'
import { pageDocumentToScreen, pageViewportToScreen, type PageScreenFrame } from '../../src/shared/page-space'

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

describe('pageDocumentToScreen', () => {
  // page() is 1:1 (viewport size equals screen size), so scale is 1 and a
  // scroll offset of N maps to exactly N screen pixels — no scale factor to
  // account for.
  const docRect = { x: 50, y: 80, width: 100, height: 40 }

  it('renders N pixels higher than the unscrolled case when scrolled down by N', () => {
    const unscrolled = pageDocumentToScreen(docRect, page({ scrollY: 0 }), ORIGIN)
    const scrolled = pageDocumentToScreen(docRect, page({ scrollY: 25 }), ORIGIN)
    expect(scrolled.top).toBe(unscrolled.top - 25)
    expect(scrolled.left).toBe(unscrolled.left)
  })

  it('renders N pixels to the left of the unscrolled case when scrolled right by N', () => {
    const unscrolled = pageDocumentToScreen(docRect, page({ scrollX: 0 }), ORIGIN)
    const scrolled = pageDocumentToScreen(docRect, page({ scrollX: 15 }), ORIGIN)
    expect(scrolled.left).toBe(unscrolled.left - 15)
    expect(scrolled.top).toBe(unscrolled.top)
  })

  it('matches pageViewportToScreen exactly when scroll is zero (or absent)', () => {
    const viaViewport = pageViewportToScreen(docRect, page(), ORIGIN)
    const viaDocumentZero = pageDocumentToScreen(docRect, page({ scrollX: 0, scrollY: 0 }), ORIGIN)
    const viaDocumentAbsent = pageDocumentToScreen(docRect, page(), ORIGIN)
    expect(viaDocumentZero).toEqual(viaViewport)
    expect(viaDocumentAbsent).toEqual(viaViewport)
  })

  it('passes the frame kind through to the underlying content/entity frame selection', () => {
    const devicePage = page({
      contentScreenX: 230,
      contentScreenY: 140,
      contentScreenWidth: 200,
      contentScreenHeight: 150,
      scrollY: 10,
    })
    const rect = pageDocumentToScreen(
      { x: 50, y: 40, width: 100, height: 20 },
      devicePage,
      ORIGIN,
      'entity',
    )
    // 'entity' ignores content bounds; scale is outer screen / page size = 1,
    // and y is shifted up by scrollY (10) versus the plain viewport mapping.
    const viewportEntity = pageViewportToScreen(
      { x: 50, y: 40, width: 100, height: 20 },
      devicePage,
      ORIGIN,
      'entity',
    )
    expect(rect.top).toBe(viewportEntity.top - 10)
    expect(rect.left).toBe(viewportEntity.left)
  })
})
