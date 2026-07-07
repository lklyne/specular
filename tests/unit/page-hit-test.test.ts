import { describe, expect, it } from 'vitest'
import { pageContentRect, pointerOverPageContent } from '../../src/shared/page-hit-test'
import type { CanvasScenePageEntity } from '../../src/shared/types'

function page(overrides: Partial<CanvasScenePageEntity> & { id: string }): CanvasScenePageEntity {
  const screenX = overrides.screenX ?? 200
  const screenY = overrides.screenY ?? 200
  const screenWidth = overrides.screenWidth ?? 400
  const screenHeight = overrides.screenHeight ?? 300
  return {
    kind: 'page',
    id: overrides.id,
    label: 'page',
    url: 'https://example.com',
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    isCustomSize: false,
    canvasX: 0,
    canvasY: 0,
    width: screenWidth,
    height: screenHeight,
    presetIndex: 0,
    synced: false,
    screenX,
    screenY,
    screenWidth,
    screenHeight,
    ...overrides,
  }
}

describe('pageContentRect — content-rect fallback chain', () => {
  it('uses content screen fields when present', () => {
    const p = page({
      id: 'p1',
      screenX: 200,
      screenY: 200,
      screenWidth: 400,
      screenHeight: 300,
      contentScreenX: 210,
      contentScreenY: 236,
      contentScreenWidth: 380,
      contentScreenHeight: 260,
    })
    expect(pageContentRect(p)).toEqual({ x: 210, y: 236, width: 380, height: 260 })
  })

  it('falls back to body bounds when content fields are absent', () => {
    const p = page({ id: 'p1', screenX: 200, screenY: 200, screenWidth: 400, screenHeight: 300 })
    expect(pageContentRect(p)).toEqual({ x: 200, y: 200, width: 400, height: 300 })
  })

  it('falls back per-field when only some content fields are set', () => {
    const p = page({
      id: 'p1',
      screenX: 200,
      screenY: 200,
      screenWidth: 400,
      screenHeight: 300,
      contentScreenY: 236,
      contentScreenHeight: 260,
    })
    expect(pageContentRect(p)).toEqual({ x: 200, y: 236, width: 400, height: 260 })
  })
})

describe('pointerOverPageContent', () => {
  const p = page({
    id: 'p1',
    screenX: 200,
    screenY: 200,
    screenWidth: 400,
    screenHeight: 300,
    contentScreenX: 210,
    contentScreenY: 236,
    contentScreenWidth: 380,
    contentScreenHeight: 260,
  })

  it('is true for a point inside the content rect', () => {
    expect(pointerOverPageContent(p, { x: 300, y: 400 })).toBe(true)
  })

  it('is true on the inclusive edges of the content rect', () => {
    expect(pointerOverPageContent(p, { x: 210, y: 236 })).toBe(true)
    expect(pointerOverPageContent(p, { x: 590, y: 496 })).toBe(true)
  })

  it('is false outside the content rect but inside the body inset', () => {
    // Point sits in the chrome/frame inset: inside the body (x=205) but left of
    // the content-left edge (210).
    expect(pointerOverPageContent(p, { x: 205, y: 400 })).toBe(false)
    expect(pointerOverPageContent(p, { x: 300, y: 220 })).toBe(false)
  })

  it('tests against the body bounds when content fields are absent', () => {
    const bodyOnly = page({ id: 'p2', screenX: 200, screenY: 200, screenWidth: 400, screenHeight: 300 })
    expect(pointerOverPageContent(bodyOnly, { x: 205, y: 220 })).toBe(true)
    expect(pointerOverPageContent(bodyOnly, { x: 199, y: 220 })).toBe(false)
  })
})
