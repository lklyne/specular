import { describe, expect, it } from 'vitest'
import {
  canvasToScreenPoint,
  clientYToWindowY,
  isPointerInPageContent,
  pageContentBounds,
  screenPointToCanvasPoint,
  screenRectToCanvasRect,
} from '../../src/shared/coords'
import type { LayoutUpdateData } from '../../src/shared/types'

function layout(partial: Partial<LayoutUpdateData> = {}): LayoutUpdateData {
  return {
    canvasOrigin: { x: 100, y: 50 },
    pan: { x: 20, y: -10 },
    zoom: 1.25,
    ...partial,
  } as LayoutUpdateData
}

describe('coords', () => {
  it('round-trips screen → canvas → screen', () => {
    const L = layout()
    const screen = { x: 742, y: 318 }
    const canvas = screenPointToCanvasPoint(screen.x, screen.y, L)
    const back = canvasToScreenPoint(L, canvas)
    expect(back.x).toBeCloseTo(screen.x, 6)
    expect(back.y).toBeCloseTo(screen.y, 6)
  })

  it('round-trips at zoom extremes', () => {
    for (const zoom of [0.02, 0.5, 1, 2, 10]) {
      const L = layout({ zoom })
      const c = screenPointToCanvasPoint(500, 400, L)
      const s = canvasToScreenPoint(L, c)
      expect(s.x).toBeCloseTo(500, 6)
      expect(s.y).toBeCloseTo(400, 6)
    }
  })

  it('converts screen rect to canvas rect consistently with point conversion', () => {
    const L = layout()
    const rect = { left: 200, top: 150, width: 300, height: 200 }
    const canvasRect = screenRectToCanvasRect(rect, L)
    const tl = screenPointToCanvasPoint(rect.left, rect.top, L)
    expect(canvasRect.x).toBeCloseTo(tl.x, 6)
    expect(canvasRect.y).toBeCloseTo(tl.y, 6)
    expect(canvasRect.width).toBeCloseTo(rect.width / L.zoom, 6)
    expect(canvasRect.height).toBeCloseTo(rect.height / L.zoom, 6)
  })

  it('adds canvasOrigin.y to overlay-relative clientY to land in window space', () => {
    const L = layout()
    expect(clientYToWindowY(200, L)).toBe(200 + L.canvasOrigin.y)
  })

  it('falls back to outer screen bounds when a page has no content* bounds', () => {
    const page = { screenX: 10, screenY: 20, screenWidth: 300, screenHeight: 200 }
    expect(pageContentBounds(page)).toEqual({ left: 10, top: 20, width: 300, height: 200 })
  })

  it('prefers content* bounds over outer screen bounds when present', () => {
    const page = {
      screenX: 0,
      screenY: 0,
      screenWidth: 400,
      screenHeight: 300,
      contentScreenX: 10,
      contentScreenY: 40,
      contentScreenWidth: 380,
      contentScreenHeight: 260,
    }
    expect(pageContentBounds(page)).toEqual({ left: 10, top: 40, width: 380, height: 260 })
  })

  it('isPointerInPageContent matches inclusive membership against the content rect', () => {
    const page = { screenX: 0, screenY: 0, screenWidth: 100, screenHeight: 50 }
    expect(isPointerInPageContent(0, 0, page)).toBe(true)
    expect(isPointerInPageContent(100, 50, page)).toBe(true)
    expect(isPointerInPageContent(101, 25, page)).toBe(false)
    expect(isPointerInPageContent(50, 51, page)).toBe(false)
  })
})
