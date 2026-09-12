import { describe, expect, it } from 'vitest'
import {
  decidePageDragOutcome,
  pageContentRect,
  pointerOverPageContent,
} from '../../src/shared/page-hit-test'
import type { CanvasScenePageEntity, PageDragPayload } from '../../src/shared/types'

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

describe('decidePageDragOutcome — ADR 0038 drag-out release decision', () => {
  const p = page({ id: 'p1', screenX: 200, screenY: 200, screenWidth: 400, screenHeight: 300 })
  const textPayload: PageDragPayload = { kind: 'text', text: 'hello' }

  it('is "none" when no drag is armed, regardless of release point or page', () => {
    expect(decidePageDragOutcome(null, { x: 300, y: 300 }, p)).toBe('none')
    expect(decidePageDragOutcome(null, { x: 900, y: 900 }, null)).toBe('none')
  })

  it('is "release-in-page" when armed and the release point is inside the page content rect', () => {
    // Verified against the mutation of flipping `pointerOverPageContent`'s
    // rectContains call to strict inequality — this case fails without it.
    expect(decidePageDragOutcome(textPayload, { x: 300, y: 300 }, p)).toBe('release-in-page')
  })

  it('is "drop-on-canvas" when armed and the release point is outside the page content rect', () => {
    expect(decidePageDragOutcome(textPayload, { x: 900, y: 900 }, p)).toBe('drop-on-canvas')
  })

  it('is "drop-on-canvas" when armed and the source page is no longer projected', () => {
    expect(decidePageDragOutcome(textPayload, { x: 300, y: 300 }, null)).toBe('drop-on-canvas')
  })
})
