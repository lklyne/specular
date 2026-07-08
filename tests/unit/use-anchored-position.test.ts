import { describe, expect, it } from 'vitest'
import { anchoredRect } from '../../src/renderer/above-view/useAnchoredPosition'
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
    synced: false,
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

describe('anchoredRect', () => {
  it('returns the page body rect adjusted for overlay origin', () => {
    const layout = makeLayout([pageEntity()])
    const rect = anchoredRect(layout, 'f1')
    expect(rect).toEqual({ x: 200, y: 250 - 60, width: 400, height: 300 })
  })

  it('returns null for unknown entity', () => {
    expect(anchoredRect(makeLayout([]), 'nope')).toBeNull()
  })

  it('body equals entity rect (overlay-local) for chromeless kinds', () => {
    const layout = makeLayout([textEntity()])
    expect(anchoredRect(layout, 't1')).toEqual({
      x: 50,
      y: 80 - 60,
      width: 100,
      height: 40,
    })
  })
})
