import { describe, expect, it } from 'vitest'
import { anchoredSlotRect } from '../../src/renderer/above-view/useAnchoredPosition'
import { CHROME_HEADER_HEIGHT } from '../../src/shared/entity-chrome-slots'
import type { CanvasScenePageEntity, CanvasSceneTextEntity, LayoutUpdateData } from '../../src/shared/types'

function pageEntity(): CanvasScenePageEntity {
  return {
    kind: 'page',
    id: 'f1',
    label: 'page',
    url: 'https://example.com',
    canGoBack: false,
    canGoForward: false,
    isLoading: false,
    isCustomSize: false,
    canvasX: 0,
    canvasY: 0,
    width: 400,
    height: 300,
    presetIndex: 0,
    linked: false,
    screenX: 200,
    screenY: 250,
    screenWidth: 400,
    screenHeight: 300,
  }
}

function textEntity(): CanvasSceneTextEntity {
  return {
    kind: 'text',
    id: 't1',
    text: 'hi',
    color: '#000',
    canvasX: 0,
    canvasY: 0,
    width: 100,
    height: 40,
    screenX: 50,
    screenY: 80,
    screenWidth: 100,
    screenHeight: 40,
  }
}

function makeLayout(entities: LayoutUpdateData['entities'], originY = 60): LayoutUpdateData {
  return { entities, canvasOrigin: { x: 0, y: originY } } as unknown as LayoutUpdateData
}

describe('anchoredSlotRect', () => {
  it('returns header rect above page body in overlay-local coords', () => {
    const layout = makeLayout([pageEntity()])
    const rect = anchoredSlotRect(layout, 'f1', 'header')
    expect(rect).toEqual({
      x: 200,
      y: 250 - CHROME_HEADER_HEIGHT - 60,
      width: 400,
      height: CHROME_HEADER_HEIGHT,
    })
  })

  it('returns body rect adjusted for overlay origin', () => {
    const layout = makeLayout([pageEntity()])
    const rect = anchoredSlotRect(layout, 'f1', 'body')
    expect(rect).toEqual({ x: 200, y: 250 - 60, width: 400, height: 300 })
  })

  it('returns null for unknown entity', () => {
    expect(anchoredSlotRect(makeLayout([]), 'nope', 'header')).toBeNull()
  })

  it('returns null when chromeless kind queried for header', () => {
    const layout = makeLayout([textEntity()])
    expect(anchoredSlotRect(layout, 't1', 'header')).toBeNull()
  })

  it('chromeless kind: body equals entity rect (overlay-local)', () => {
    const layout = makeLayout([textEntity()])
    expect(anchoredSlotRect(layout, 't1', 'body')).toEqual({
      x: 50,
      y: 80 - 60,
      width: 100,
      height: 40,
    })
  })
})
