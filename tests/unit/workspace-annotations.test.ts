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
  findPageById: vi.fn(),
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

  it('filters by pageId for page-anchored annotations', () => {
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'page', pageId: 'f1', pageX: 0, pageY: 0 },
      }),
    )
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'page', pageId: 'f2', pageX: 0, pageY: 0 },
      }),
    )
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'canvas', canvasX: 0, canvasY: 0 },
      }),
    )

    const result = getAnnotations({ pageId: 'f1' })
    expect(result).toHaveLength(1)
    expect(result[0].anchor.type === 'page' && result[0].anchor.pageId).toBe('f1')
  })

  it('excludes canvas-anchored annotations when filtering by pageId', () => {
    mockAnnotations.push(
      makeAnnotation({
        anchor: { type: 'canvas', canvasX: 0, canvasY: 0 },
      }),
    )
    expect(getAnnotations({ pageId: 'f1' })).toHaveLength(0)
  })

  it('combines status and pageId filters', () => {
    mockAnnotations.push(
      makeAnnotation({
        status: 'pending',
        anchor: { type: 'page', pageId: 'f1', pageX: 0, pageY: 0 },
      }),
    )
    mockAnnotations.push(
      makeAnnotation({
        status: 'resolved',
        anchor: { type: 'page', pageId: 'f1', pageX: 0, pageY: 0 },
      }),
    )

    expect(getAnnotations({ status: 'unresolved', pageId: 'f1' })).toHaveLength(1)
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
