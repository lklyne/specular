import { describe, expect, it } from 'vitest'
import {
  domRectToPageScreenRect,
  placeInspectPopover,
} from '../../src/shared/inspect-popover-position'
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
    canvasX: 0,
    canvasY: 0,
    width: 1000,
    height: 800,
    presetIndex: 0,
    synced: false,
    screenX: 300,
    screenY: 120,
    screenWidth: 500,
    screenHeight: 400,
    ...overrides,
  }
}

describe('inspect popover positioning', () => {
  it('projects a page viewport DOM rect into zoomed page screen coordinates', () => {
    const rect = domRectToPageScreenRect(
      { x: 100, y: 200, width: 50, height: 20 },
      page(),
    )

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
    const rect = domRectToPageScreenRect(
      { x: 100, y: 200, width: 50, height: 20 },
      page({
        contentScreenX: 330,
        contentScreenY: 180,
        contentScreenWidth: 250,
        contentScreenHeight: 200,
      }),
    )

    expect(rect.left).toBe(355)
    expect(rect.top).toBe(230)
    expect(rect.width).toBe(12.5)
    expect(rect.height).toBe(5)
  })

  it('keeps the popover size independent of canvas zoom', () => {
    const zoomedOutTarget = domRectToPageScreenRect(
      { x: 100, y: 100, width: 50, height: 20 },
      page({ screenWidth: 250, screenHeight: 200 }),
    )
    const zoomedInTarget = domRectToPageScreenRect(
      { x: 100, y: 100, width: 50, height: 20 },
      page({ screenWidth: 2000, screenHeight: 1600 }),
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
