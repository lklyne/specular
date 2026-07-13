/**
 * Page-bound annotations hide when their page navigates away from the URL
 * they were created on. The gate is main-side (`annotationHiddenByPageDocument`
 * in document-binding.ts): hidden annotations are dropped from the layout
 * broadcast, so renderers never re-derive visibility from URLs.
 *
 * Mutation-verified by: (1) making `offPageDocument` return false
 * unconditionally (document-binding.ts) — the navigation-hiding cases fail;
 * (2) deleting the `parsed.hash = ''` line in `canonicalPageUrl`
 * (page-anchor.ts) — the hash-insensitivity cases fail.
 */

import { afterEach, describe, expect, it } from 'vitest'
import {
  annotationHiddenByPageDocument,
  entityHiddenByPageAnchor,
} from '../../src/main/runtime/document-binding'
import { pages } from '../../src/main/runtime/runtime-context'
import type { Page } from '../../src/main/runtime/runtime-entities'
import {
  annotationContextPageId,
  annotationMatchesPageUrl,
  canonicalAnnotationUrl,
} from '../../src/shared/annotation-utils'
import type { Annotation } from '../../src/shared/types'

const PAGE_ID = 'page-1'
const PAGE_URL = 'https://example.com/pricing'

/** Point the runtime's page registry at a single page showing `url`. */
function showPage(url: string): void {
  pages.length = 0
  pages.push({ id: PAGE_ID, url } as unknown as Page)
}

function annotation(partial: Partial<Annotation>): Annotation {
  return {
    id: 'ann-1',
    anchor: {
      type: 'element',
      pageId: PAGE_ID,
      selector: 'main > section.hero',
      boundingBox: { x: 50, y: 40, width: 100, height: 20 },
    },
    author: 'user',
    text: 'tighten this up',
    status: 'pending',
    replies: [],
    createdAt: '2026-01-01T00:00:00Z',
    metadata: { pageUrl: PAGE_URL },
    ...partial,
  }
}

describe('canonicalAnnotationUrl', () => {
  it('strips the hash but keeps the query', () => {
    expect(canonicalAnnotationUrl('https://example.com/a?tab=2#section')).toBe(
      'https://example.com/a?tab=2',
    )
  })

  it('passes non-URL strings through unchanged', () => {
    expect(canonicalAnnotationUrl('not a url')).toBe('not a url')
  })

  it('returns undefined for empty and whitespace values', () => {
    expect(canonicalAnnotationUrl('')).toBeUndefined()
    expect(canonicalAnnotationUrl('   ')).toBeUndefined()
    expect(canonicalAnnotationUrl(null)).toBeUndefined()
  })
})

describe('annotationMatchesPageUrl', () => {
  it('matches when only the hash differs', () => {
    expect(annotationMatchesPageUrl(annotation({}), `${PAGE_URL}#faq`)).toBe(true)
  })

  it('rejects a different path on the same origin', () => {
    expect(annotationMatchesPageUrl(annotation({}), 'https://example.com/about')).toBe(false)
  })

  it('matches when the annotation has no recorded pageUrl', () => {
    expect(annotationMatchesPageUrl(annotation({ metadata: undefined }), PAGE_URL)).toBe(true)
  })

  it('matches when the page has no URL yet (loading / about:blank-less)', () => {
    expect(annotationMatchesPageUrl(annotation({}), '')).toBe(true)
  })
})

describe('annotationContextPageId', () => {
  it('reads the pageId off element and page anchors', () => {
    expect(annotationContextPageId(annotation({}))).toBe(PAGE_ID)
    expect(
      annotationContextPageId(
        annotation({ anchor: { type: 'page', pageId: 'page-9', offsetX: 0.5, offsetY: 0.5 } }),
      ),
    ).toBe('page-9')
  })

  it('reads the primary page off region metadata and returns null for canvas anchors', () => {
    expect(
      annotationContextPageId(
        annotation({
          anchor: { type: 'region', canvasRect: { x: 0, y: 0, width: 10, height: 10 } },
          metadata: {
            pageUrl: PAGE_URL,
            regionComponents: [{ pageId: 'page-7', pageName: 'Page 7', components: [] }],
          },
        }),
      ),
    ).toBe('page-7')
    expect(
      annotationContextPageId(annotation({ anchor: { type: 'canvas', canvasX: 1, canvasY: 2 } })),
    ).toBeNull()
  })
})

describe('annotationHiddenByPageDocument (main-side layout gate)', () => {
  afterEach(() => {
    pages.length = 0
  })

  it('shows the annotation while the page is on its URL, hides it after navigation', () => {
    showPage(PAGE_URL)
    expect(annotationHiddenByPageDocument(annotation({}))).toBe(false)

    showPage('https://example.com/about')
    expect(annotationHiddenByPageDocument(annotation({}))).toBe(true)
  })

  it('gates page-anchored and region annotations through their context page', () => {
    showPage('https://example.com/about')
    expect(
      annotationHiddenByPageDocument(
        annotation({ anchor: { type: 'page', pageId: PAGE_ID, offsetX: 0.5, offsetY: 0.5 } }),
      ),
    ).toBe(true)
    expect(
      annotationHiddenByPageDocument(
        annotation({
          anchor: { type: 'region', canvasRect: { x: 0, y: 0, width: 10, height: 10 } },
          metadata: {
            pageUrl: PAGE_URL,
            regionComponents: [{ pageId: PAGE_ID, pageName: 'Page 1', components: [] }],
          },
        }),
      ),
    ).toBe(true)
  })

  it('never hides hash-only navigation, URL-less annotations, canvas anchors, or missing pages', () => {
    showPage(`${PAGE_URL}#faq`)
    expect(annotationHiddenByPageDocument(annotation({}))).toBe(false)

    showPage('https://example.com/about')
    expect(annotationHiddenByPageDocument(annotation({ metadata: undefined }))).toBe(false)
    expect(
      annotationHiddenByPageDocument(
        annotation({ anchor: { type: 'canvas', canvasX: 1, canvasY: 2 } }),
      ),
    ).toBe(false)

    pages.length = 0
    expect(annotationHiddenByPageDocument(annotation({}))).toBe(false)
  })

  it('applies the same gate to page-anchored entities', () => {
    const entity = { id: 'sticky-1', pageAnchor: { pageId: PAGE_ID, pageUrl: PAGE_URL } }
    showPage(PAGE_URL)
    expect(entityHiddenByPageAnchor(entity)).toBe(false)
    showPage('https://example.com/about')
    expect(entityHiddenByPageAnchor(entity)).toBe(true)
    expect(entityHiddenByPageAnchor({ id: 'sticky-1' })).toBe(false)
  })
})
