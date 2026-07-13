import { describe, expect, it } from 'vitest'
import { commentBadgesForLayout } from '../../src/renderer/above-view/CommentBadgesLayer'
import { annotationScreenPos, type AnnotationLiveBboxLookup } from '../../src/renderer/above-view/annotationMath'
import type { Annotation, LayoutUpdateData } from '../../src/shared/types'

const PAGE = {
  id: 'page-1',
  kind: 'page' as const,
  screenX: 200,
  screenY: 100,
  screenWidth: 400,
  screenHeight: 300,
  width: 400,
  height: 300,
}

function layout(partial: Partial<LayoutUpdateData> = {}): LayoutUpdateData {
  return {
    canvasOrigin: { x: 0, y: 50 },
    pan: { x: 0, y: 0 },
    zoom: 1,
    entities: [PAGE],
    ...partial,
  } as LayoutUpdateData
}

function elementAnnotation(boundingBox: { x: number; y: number; width: number; height: number }): Annotation {
  return {
    id: 'ann-1',
    anchor: {
      type: 'element',
      pageId: PAGE.id,
      selector: 'main > section.hero',
      boundingBox,
    },
    author: 'user',
    text: 'whatever',
    status: 'pending',
    replies: [],
    createdAt: '2026-01-01T00:00:00Z',
  }
}

function lookup(map: Record<string, { x: number; y: number; width: number; height: number }>): AnnotationLiveBboxLookup {
  return {
    get: (id) => map[id],
    isStale: () => false,
  }
}

describe('annotationScreenPos with live bboxes', () => {
  it('uses the stored boundingBox when no live lookup is provided', () => {
    const ann = elementAnnotation({ x: 50, y: 80, width: 100, height: 40 })
    const pos = annotationScreenPos(ann, layout())
    expect(pos).not.toBeNull()
    // page.screenX (200) + (50 + 100) - rightInset(8) = 342
    expect(pos!.x).toBeCloseTo(342, 6)
  })

  it('prefers the live bbox over the stored one when a live lookup hits', () => {
    const ann = elementAnnotation({ x: 50, y: 80, width: 100, height: 40 })
    const pos = annotationScreenPos(
      ann,
      layout(),
      lookup({ 'ann-1': { x: 80, y: 80, width: 100, height: 40 } }),
    )
    expect(pos).not.toBeNull()
    // 200 + (80 + 100) - 8 = 372
    expect(pos!.x).toBeCloseTo(372, 6)
  })

  it('falls back to stored bbox when the live lookup misses', () => {
    const ann = elementAnnotation({ x: 50, y: 80, width: 100, height: 40 })
    const pos = annotationScreenPos(ann, layout(), lookup({}))
    expect(pos!.x).toBeCloseTo(342, 6)
  })

  it('anchors thread popovers to the outer page frame even on device pages', () => {
    // Pins the deliberate divergence in annotationScreenPos: the thread
    // popover maps through the entity's OUTER frame, not the device content
    // viewport. Mutation-verified by switching its pageViewportToScreen call
    // to the default 'content' frame — this test then sees the content-frame
    // x (230 + 150 * 0.5 - 8 = 297) instead of 342.
    const devicePage = {
      ...PAGE,
      contentScreenX: 230,
      contentScreenY: 140,
      contentScreenWidth: 200,
      contentScreenHeight: 150,
    }
    const ann = elementAnnotation({ x: 50, y: 80, width: 100, height: 40 })
    const pos = annotationScreenPos(ann, layout({ entities: [devicePage] }))
    expect(pos).not.toBeNull()
    // page.screenX (200) + (50 + 100) * (400/400) - rightInset(8) = 342
    expect(pos!.x).toBeCloseTo(342, 6)
    // toOverlayY(page.screenY (100) + 80 * (300/300)) + topInset(8) = 138
    expect(pos!.y).toBeCloseTo(138, 6)
  })

  it('still positions canvas-anchored annotations the same way', () => {
    const ann: Annotation = {
      id: 'canv-1',
      anchor: { type: 'canvas', canvasX: 50, canvasY: 50 },
      author: 'user',
      text: '',
      status: 'pending',
      replies: [],
      createdAt: '2026-01-01T00:00:00Z',
    }
    const pos = annotationScreenPos(ann, layout(), lookup({ 'canv-1': { x: 0, y: 0, width: 1, height: 1 } }))
    expect(pos).not.toBeNull()
  })
})

describe('commentBadgesForLayout', () => {
  it('positions element badges in the page content viewport above device frames', () => {
    const page = {
      ...PAGE,
      contentScreenX: 230,
      contentScreenY: 140,
      contentScreenWidth: 200,
      contentScreenHeight: 150,
    }
    const ann = elementAnnotation({ x: 50, y: 40, width: 100, height: 20 })
    const badges = commentBadgesForLayout(
      [ann],
      layout({ entities: [page] }),
      lookup({}),
    )

    expect(badges).toHaveLength(1)
    expect(badges[0]).toMatchObject({
      annotationId: ann.id,
      count: 1,
      x: 297,
      y: 118,
      transform: 'translate(-100%, 0)',
      highlightRect: {
        left: 255,
        top: 110,
        width: 50,
        height: 10,
      },
    })
  })
})
