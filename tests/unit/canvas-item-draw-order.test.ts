// Guards CanvasItemSurface's draw list. Each page carries its own chrome and
// texture in scene order, so a page stacked above another covers that page's
// bezel as well as its content; the focused page draws last; focus sessions
// drop hidden context and strip the fill page's chrome.
// Mutation-verified by (1) dropping the "move focused page last" step in
// orderCanvasItemDraws and confirming "draws the focused page last" fails, and
// (2) appending device-framed files after every page instead of in scene order
// and confirming "paints each item whole, in scene order" fails.

import { describe, expect, it } from 'vitest'
import { orderCanvasItemDraws, type CanvasItemDraw } from '../../src/renderer/canvas-bg/canvasItemDrawOrder'
import type { PagePresentationFocus } from '../../src/shared/page-presentation'
import type { ProjectedSceneEntity } from '../../src/shared/scene-projection'

function page(id: string, showDeviceFrame = false): ProjectedSceneEntity {
  return { kind: 'page', id, showDeviceFrame } as unknown as ProjectedSceneEntity
}

function file(id: string, showDeviceFrame: boolean): ProjectedSceneEntity {
  return { kind: 'file', id, showDeviceFrame } as unknown as ProjectedSceneEntity
}

function presentation(focus: Partial<PagePresentationFocus>): PagePresentationFocus {
  const active = focus.active ?? false
  return { active, pageId: null, mode: null, showsContext: !active, ...focus }
}

function summary(draws: CanvasItemDraw[]) {
  return draws.map((draw) => [draw.item.id, draw.chrome, draw.pageId])
}

describe('orderCanvasItemDraws', () => {
  it('paints each item whole, in scene order', () => {
    const entities = [page('a', true), file('framed', true), file('plain', false), page('b')]
    expect(summary(orderCanvasItemDraws(entities, presentation({})))).toEqual([
      ['a', true, 'a'],
      ['framed', true, null],
      ['b', true, 'b'],
    ])
  })

  it('draws the focused page last', () => {
    const entities = [page('a'), page('b'), page('c')]
    const draws = orderCanvasItemDraws(
      entities,
      presentation({ active: true, pageId: 'b', mode: 'fit', showsContext: true }),
    )
    expect(draws.map((draw) => draw.item.id)).toEqual(['a', 'c', 'b'])
  })

  it('drops other pages and files while a focus session hides context', () => {
    const entities = [page('a'), file('framed', true), page('b')]
    const draws = orderCanvasItemDraws(entities, presentation({ active: true, pageId: 'b', mode: 'fit' }))
    expect(summary(draws)).toEqual([['b', true, 'b']])
  })

  it('draws a fill page edge to edge, with no border or bezel', () => {
    const entities = [page('a', true), page('b', true)]
    const draws = orderCanvasItemDraws(entities, presentation({ active: true, pageId: 'b', mode: 'fill' }))
    expect(summary(draws)).toEqual([['b', false, 'b']])
    expect(draws[0].item.showDeviceFrame).toBe(false)
  })
})
