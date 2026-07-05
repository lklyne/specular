import { describe, expect, it } from 'vitest'
import {
  canvasToScreenPoint,
  clientYToWindowY,
  screenPointToCanvasPoint,
  screenRectToCanvasRect,
  toOverlayY,
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

  it('converts clientY to window Y by adding the overlay origin offset', () => {
    const L = layout()
    expect(clientYToWindowY(300, L)).toBe(300 + L.canvasOrigin.y)
  })

  it('is the inverse of toOverlayY', () => {
    const L = layout()
    const clientY = 412
    const windowY = clientYToWindowY(clientY, L)
    expect(toOverlayY(L, windowY)).toBe(clientY)
  })
})
