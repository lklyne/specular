/**
 * Annotation store: creation-time `pageAnchor` derivation and the
 * status/page/url query filters.
 *
 * Mutation-verified by:
 * - removing the `...(pageAnchor ? { pageAnchor } : {})` spread in
 *   `createAnnotationInternal` (workspace-annotations.ts) — every
 *   "writes a pageAnchor" case fails;
 * - hard-coding the region branch of `annotationAnchorPageId` to a page id
 *   (anchoring regions regardless of grab) — the grab-less region case fails;
 * - reading the structural `anchor.pageId` instead of `pageAnchor` in the
 *   `getAnnotations` pageId filter — the legacy-record filter case fails.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { Annotation } from '../../src/shared/types'

// ---------------------------------------------------------------------------
// Mocks — stub out main-process dependencies so we can test pure filter logic
// ---------------------------------------------------------------------------

const { mockAnnotations } = vi.hoisted(() => {
  const mockAnnotations: Annotation[] = []
  return { mockAnnotations }
})

vi.mock('../../src/main/runtime/workspace-model', () => ({
  workspaceAnnotations: mockAnnotations,
}))

vi.mock('../../src/main/runtime/page-runtime', () => ({
  findPageById: vi.fn((pageId: string) =>
    pageId === 'page-live'
      ? { id: 'page-live', url: 'https://example.com/pricing#hero', presetIndex: 0 }
      : undefined,
  ),
  getComponentAncestryByNodeId: vi.fn(() => []),
  getComponentSourceLocationByNodeId: vi.fn(),
}))

vi.mock('../../src/main/runtime/layout-dirty', () => ({
  markDirty: vi.fn(),
}))

vi.mock('../../src/main/runtime/viewport-control', () => ({
  requestLayout: vi.fn(),
}))

vi.mock('../../src/main/runtime/workspace-autosave', () => ({
  scheduleWorkspaceAutosave: vi.fn(),
}))

vi.mock('../../src/main/workspace-utils', () => ({
  makeId: vi.fn(() => 'test-id'),
}))

import { createAnnotation, getAnnotations } from '../../src/main/workspace-annotations'

function makeAnnotation(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: `ann-${Math.random().toString(36).slice(2, 8)}`,
    anchor: { type: 'canvas', canvasX: 0, canvasY: 0 },
    author: 'user',
    text: 'test',
    status: 'pending',
    replies: [],
    createdAt: new Date().toISOString(),
    ...overrides,
  }
}

describe('getAnnotations', () => {
  beforeEach(() => {
    mockAnnotations.length = 0
  })

  it('returns all annotations when no filters provided', () => {
    mockAnnotations.push(makeAnnotation({ status: 'pending' }))
    mockAnnotations.push(makeAnnotation({ status: 'resolved' }))
    expect(getAnnotations()).toHaveLength(2)
  })

  it('filters by exact status', () => {
    mockAnnotations.push(makeAnnotation({ status: 'pending' }))
    mockAnnotations.push(makeAnnotation({ status: 'resolved' }))
    mockAnnotations.push(makeAnnotation({ status: 'acknowledged' }))

    expect(getAnnotations({ status: 'pending' })).toHaveLength(1)
    expect(getAnnotations({ status: 'resolved' })).toHaveLength(1)
    expect(getAnnotations({ status: 'acknowledged' })).toHaveLength(1)
  })

  it('"unresolved" matches pending and acknowledged', () => {
    mockAnnotations.push(makeAnnotation({ status: 'pending' }))
    mockAnnotations.push(makeAnnotation({ status: 'acknowledged' }))
    mockAnnotations.push(makeAnnotation({ status: 'resolved' }))
    mockAnnotations.push(makeAnnotation({ status: 'dismissed' }))

    const result = getAnnotations({ status: 'unresolved' })
    expect(result).toHaveLength(2)
    expect(result.every((a) => a.status === 'pending' || a.status === 'acknowledged')).toBe(true)
  })

  it('"all" returns every annotation regardless of status', () => {
    mockAnnotations.push(makeAnnotation({ status: 'pending' }))
    mockAnnotations.push(makeAnnotation({ status: 'resolved' }))
    mockAnnotations.push(makeAnnotation({ status: 'dismissed' }))
    mockAnnotations.push(makeAnnotation({ status: 'acknowledged' }))

    expect(getAnnotations({ status: 'all' })).toHaveLength(4)
  })

  it('filters by pageId through the pageAnchor binding', () => {
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'page', pageId: 'f1', offsetX: 0, offsetY: 0 },
        pageAnchor: { pageId: 'f1' },
      }),
    )
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'page', pageId: 'f2', offsetX: 0, offsetY: 0 },
        pageAnchor: { pageId: 'f2' },
      }),
    )
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'canvas', canvasX: 0, canvasY: 0 },
      }),
    )

    const result = getAnnotations({ pageId: 'f1' })
    expect(result).toHaveLength(1)
    expect(result[0].pageAnchor?.pageId).toBe('f1')
  })

  it('excludes canvas-bound annotations (no pageAnchor) from pageId and url filters', () => {
    // Legacy-shaped record: structural page anchor but no pageAnchor field.
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'page', pageId: 'f1', offsetX: 0, offsetY: 0 },
        metadata: { pageUrl: 'https://example.com/pricing' },
      }),
    )
    expect(getAnnotations({ pageId: 'f1' })).toHaveLength(0)
    expect(getAnnotations({ url: 'https://example.com/pricing' })).toHaveLength(0)
  })

  it('filters by url through the pageAnchor binding, hash-insensitively', () => {
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'page', pageId: 'f1', offsetX: 0, offsetY: 0 },
        pageAnchor: { pageId: 'f1', pageUrl: 'https://example.com/pricing' },
      }),
    )
    expect(getAnnotations({ url: 'https://example.com/pricing#faq' })).toHaveLength(1)
    expect(getAnnotations({ url: 'https://example.com/about' })).toHaveLength(0)
  })

  it('combines status and pageId filters', () => {
    mockAnnotations.push(
      makeAnnotation({
        status: 'pending',
        anchor: { type: 'page', pageId: 'f1', offsetX: 0, offsetY: 0 },
        pageAnchor: { pageId: 'f1' },
      }),
    )
    mockAnnotations.push(
      makeAnnotation({
        status: 'resolved',
        anchor: { type: 'page', pageId: 'f1', offsetX: 0, offsetY: 0 },
        pageAnchor: { pageId: 'f1' },
      }),
    )

    expect(getAnnotations({ status: 'unresolved', pageId: 'f1' })).toHaveLength(1)
  })
})

describe('createAnnotation pageAnchor derivation', () => {
  beforeEach(() => {
    mockAnnotations.length = 0
  })

  it('binds element and page anchors to their page with its canonical URL', () => {
    const created = createAnnotation({
      anchor: { type: 'page', pageId: 'page-live', offsetX: 0.5, offsetY: 0.5 },
      text: 'page note',
    })
    // Hash stripped: the page mock shows .../pricing#hero.
    expect(created.pageAnchor).toEqual({
      pageId: 'page-live',
      pageUrl: 'https://example.com/pricing',
    })
    expect(created.metadata?.pageUrl).toBeUndefined()
  })

  it('binds a region iff the marquee grabbed page content (first group wins)', () => {
    const grabbed = createAnnotation({
      anchor: { type: 'region', canvasRect: { x: 0, y: 0, width: 10, height: 10 } },
      text: 'grabbed region',
      metadata: {
        regionComponents: [
          { pageId: 'page-live', pageName: 'Live', components: [] },
          { pageId: 'page-other', pageName: 'Other', components: [] },
        ],
      },
    })
    expect(grabbed.pageAnchor).toEqual({
      pageId: 'page-live',
      pageUrl: 'https://example.com/pricing',
    })

    const grabless = createAnnotation({
      anchor: { type: 'region', canvasRect: { x: 0, y: 0, width: 10, height: 10 } },
      text: 'canvas region',
      metadata: { regionComponents: [], regionElements: [] },
    })
    expect(grabless.pageAnchor).toBeUndefined()
  })

  it('never binds canvas points, and skips the anchor when the page is gone', () => {
    const canvasPoint = createAnnotation({
      anchor: { type: 'canvas', canvasX: 5, canvasY: 5 },
      text: 'free note',
    })
    expect(canvasPoint.pageAnchor).toBeUndefined()

    const orphan = createAnnotation({
      anchor: { type: 'page', pageId: 'page-gone', offsetX: 0, offsetY: 0 },
      text: 'orphan note',
    })
    expect(orphan.pageAnchor).toBeUndefined()
  })
})

describe('createAnnotation elementName (ADR 0013 §6)', () => {
  beforeEach(() => {
    mockAnnotations.length = 0
  })

  it('stores elementName on element-anchored annotations', () => {
    const created = createAnnotation({
      anchor: {
        type: 'element',
        pageId: 'p1',
        selector: '#submit',
        elementPath: 'body > button#submit',
      },
      text: 'tighten copy',
      elementName: 'Submit button',
    })
    expect(created.elementName).toBe('Submit button')
  })

  it('trims and ignores empty elementName', () => {
    const created = createAnnotation({
      anchor: {
        type: 'element',
        pageId: 'p1',
        selector: '#x',
        elementPath: 'body > div#x',
      },
      text: 'note',
      elementName: '   ',
    })
    expect(created.elementName).toBeUndefined()
  })

  it('does not attach elementName to canvas-point annotations', () => {
    const created = createAnnotation({
      anchor: { type: 'canvas', canvasX: 0, canvasY: 0 },
      text: 'free note',
      elementName: 'should be ignored',
    })
    expect(created.elementName).toBeUndefined()
  })

  it('does not attach elementName to region annotations', () => {
    const created = createAnnotation({
      anchor: { type: 'region', canvasRect: { x: 0, y: 0, width: 10, height: 10 } },
      text: 'region note',
      elementName: 'should be ignored',
    })
    expect(created.elementName).toBeUndefined()
  })
})
