/**
 * Pure math behind the offscreen-rendering lab. Mutation-verified by flipping
 * the sign of the anchor correction in `zoomCameraAt` (the anchor test fails)
 * and by returning the first hit instead of the last in `hitTestPages` (the
 * overlap test fails).
 */
import { describe, expect, it } from 'vitest'
import {
  boundsOf,
  canvasToScreen,
  electronKeyCodeFor,
  fitCamera,
  hitTestPages,
  layoutPages,
  screenToCanvas,
  summarizeFrameIntervals,
  visiblePageIds,
  windowsVirtualKeyCodeFor,
  zoomCameraAt,
} from '../../src/shared/osr-lab'

describe('camera', () => {
  it('round-trips screen and canvas points', () => {
    const camera = { x: 120, y: -40, zoom: 0.37 }
    const canvas = screenToCanvas(camera, 500, 300)
    const screen = canvasToScreen(camera, canvas.x, canvas.y)
    expect(screen.x).toBeCloseTo(500, 6)
    expect(screen.y).toBeCloseTo(300, 6)
  })

  it('zooms about the pointer so the canvas point under it stays put', () => {
    const camera = { x: 60, y: 60, zoom: 0.5 }
    const before = screenToCanvas(camera, 640, 400)
    const zoomed = zoomCameraAt(camera, -100, 640, 400)
    expect(zoomed.zoom).toBeGreaterThan(camera.zoom)
    const after = screenToCanvas(zoomed, 640, 400)
    expect(after.x).toBeCloseTo(before.x, 6)
    expect(after.y).toBeCloseTo(before.y, 6)
  })

  it('fits bounds inside the viewport with the margin on every side', () => {
    const bounds = { x: 100, y: 200, width: 4000, height: 2000 }
    const camera = fitCamera(bounds, { width: 1000, height: 800 }, 40)
    const topLeft = canvasToScreen(camera, bounds.x, bounds.y)
    const bottomRight = canvasToScreen(camera, bounds.x + bounds.width, bounds.y + bounds.height)
    expect(topLeft.x).toBeGreaterThanOrEqual(40 - 1e-6)
    expect(topLeft.y).toBeGreaterThanOrEqual(40 - 1e-6)
    expect(bottomRight.x).toBeLessThanOrEqual(960 + 1e-6)
    expect(bottomRight.y).toBeLessThanOrEqual(760 + 1e-6)
    expect(bottomRight.x - topLeft.x).toBeCloseTo(920, 6)
  })
})

describe('layout and hit testing', () => {
  it('lays pages out in a near-square grid with no overlap', () => {
    const rects = layoutPages(9, 1280, 800)
    expect(rects).toHaveLength(9)
    expect(rects[3].x).toBe(0)
    expect(rects[3].y).toBeGreaterThan(rects[0].y + 800)
    const bounds = boundsOf(rects)
    expect(bounds.width).toBeCloseTo(3 * 1280 + 2 * 80)
  })

  it('returns the topmost (last) page when pages overlap', () => {
    const camera = { x: 0, y: 0, zoom: 1 }
    const rects = [
      { x: 0, y: 0, width: 100, height: 100 },
      { x: 50, y: 50, width: 100, height: 100 },
    ]
    expect(hitTestPages(camera, rects, 75, 75)).toBe(1)
    expect(hitTestPages(camera, rects, 10, 10)).toBe(0)
    expect(hitTestPages(camera, rects, 300, 300)).toBe(-1)
  })

  it('reports only pages whose projection overlaps the viewport', () => {
    const camera = { x: 0, y: 0, zoom: 0.5 }
    const pages = [
      { id: 'a', rect: { x: 0, y: 0, width: 400, height: 400 } },
      { id: 'b', rect: { x: 3000, y: 0, width: 400, height: 400 } },
    ]
    expect(visiblePageIds(camera, pages, { width: 800, height: 600 })).toEqual(['a'])
  })
})

describe('frame interval summary', () => {
  it('counts frames over 1.5× the refresh interval as long', () => {
    const summary = summarizeFrameIntervals([8, 8, 8, 30, 8], 8.33)
    expect(summary.draws).toBe(5)
    expect(summary.longFrames).toBe(1)
    expect(summary.maxFrameMs).toBe(30)
    expect(summary.drawFps).toBeCloseTo(5 / 0.062, 1)
  })
})

describe('key mapping', () => {
  it('maps DOM keys to Electron accelerator names and Windows virtual keys', () => {
    expect(electronKeyCodeFor('a')).toBe('a')
    expect(electronKeyCodeFor('ArrowLeft')).toBe('Left')
    expect(electronKeyCodeFor('F13')).toBeNull()
    expect(windowsVirtualKeyCodeFor('Enter')).toBe(13)
    expect(windowsVirtualKeyCodeFor('a')).toBe(65)
    expect(windowsVirtualKeyCodeFor('é')).toBeNull()
  })
})
