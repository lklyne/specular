import { describe, expect, it } from 'vitest'
import {
  domRectToPageScreenRect,
  placeInspectPopover,
} from '../../src/shared/inspect-popover-position'
import type { Camera } from '../../src/shared/coords'
import type { CanvasScenePageEntity } from '../../src/shared/types'

function page(overrides: Partial<CanvasScenePageEntity> = {}): CanvasScenePageEntity {
  return {
    kind: 'page',
    id: 'page-1',
    label: 'Page',
    url: 'https://example.com',
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    isCustomSize: false,
    canvasX: 0,
    canvasY: 0,
    width: 1000,
    height: 800,
    presetIndex: 0,
    linked: false,
    visualCanvasX: 0,
    visualCanvasY: 0,
    visualWidth: 1000,
    visualHeight: 800,
    ...overrides,
  }
}

// Camera that projects the page's content rect (canvasX 0, canvasY 0, 1000×800)
// to the old fixture's screen rect (300, 120, 500, 400): pan (300,120), zoom 0.5.
const cam: Camera = { pan: { x: 300, y: 120 }, zoom: 0.5, canvasOrigin: { x: 0, y: 0 } }

describe('inspect popover positioning', () => {
  it('projects a page viewport DOM rect into zoomed page screen coordinates', () => {
    const rect = domRectToPageScreenRect({ x: 100, y: 200, width: 50, height: 20 }, page(), cam)

    expect(rect).toEqual({
      left: 350,
      top: 220,
      right: 375,
      bottom: 230,
      width: 25,
      height: 10,
    })
  })

  it('uses device content bounds instead of the outer shell when present', () => {
    // Content rect inset within the shell: projects (via cam) to screen
    // (330, 180, 250, 200) — a quarter of the 1000-wide page → scaleX 0.25.
    const rect = domRectToPageScreenRect(
      { x: 100, y: 200, width: 50, height: 20 },
      page({
        contentCanvasX: 60,
        contentCanvasY: 120,
        contentCanvasWidth: 500,
        contentCanvasHeight: 400,
      }),
      cam,
    )

    expect(rect.left).toBe(355)
    expect(rect.top).toBe(230)
    expect(rect.width).toBe(12.5)
    expect(rect.height).toBe(5)
  })

  it('keeps the popover size independent of canvas zoom', () => {
    // Same page, two zooms: content screen width 250 (zoom 0.25) vs 2000 (zoom 2).
    const camOut: Camera = { pan: { x: 300, y: 120 }, zoom: 0.25, canvasOrigin: { x: 0, y: 0 } }
    const camIn: Camera = { pan: { x: 300, y: 120 }, zoom: 2, canvasOrigin: { x: 0, y: 0 } }
    const zoomedOutTarget = domRectToPageScreenRect(
      { x: 100, y: 100, width: 50, height: 20 },
      page(),
      camOut,
    )
    const zoomedInTarget = domRectToPageScreenRect(
      { x: 100, y: 100, width: 50, height: 20 },
      page(),
      camIn,
    )
    const popover = { width: 260, height: 84 }
    const zoomedOutPosition = placeInspectPopover(zoomedOutTarget, popover, { width: 1200, height: 900 })
    const zoomedInPosition = placeInspectPopover(zoomedInTarget, popover, { width: 1200, height: 900 })

    expect(zoomedOutPosition).toEqual({ left: 325, top: 55 })
    expect(zoomedInPosition).toEqual({ left: 500, top: 230 })
    expect(zoomedOutTarget.top - zoomedOutPosition.top).toBe(popover.height + 6)
    expect(zoomedInTarget.top - zoomedInPosition.top).toBe(popover.height + 6)
  })

  it('places below the target when there is no room above and clamps onscreen', () => {
    expect(placeInspectPopover(
      { left: 1180, top: 20, right: 1190, bottom: 30, width: 10, height: 10 },
      { width: 260, height: 84 },
      { width: 1200, height: 900 },
    )).toEqual({ left: 932, top: 36 })
  })
})
