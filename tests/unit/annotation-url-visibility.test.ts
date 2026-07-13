/**
 * Page-bound items hide when their page navigates away from the URL they
 * were placed on. The binding is `pageAnchor { pageId, pageUrl }` — one
 * field for anchored entities and annotations alike — and the gate is
 * main-side (`hiddenByPageAnchor` in document-binding.ts): hidden items are
 * dropped from the layout broadcast, so renderers never re-derive
 * visibility from URLs. Annotations without a `pageAnchor` (legacy records,
 * canvas points, grab-less regions) are canvas-bound and never hide.
 *
 * Mutation-verified by: (1) making `offPageDocument` return false
 * unconditionally (document-binding.ts) — the navigation-hiding cases fail;
 * (2) deleting the `parsed.hash = ''` line in `canonicalPageUrl`
 * (page-anchor.ts) — the hash-insensitivity cases fail.
 */

import { afterEach, describe, expect, it } from 'vitest'
import { hiddenByPageAnchor } from '../../src/main/runtime/document-binding'
import { pages } from '../../src/main/runtime/runtime-context'
import type { Page } from '../../src/main/runtime/runtime-entities'
import { canonicalAnnotationUrl } from '../../src/shared/annotation-utils'
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
    pageAnchor: { pageId: PAGE_ID, pageUrl: PAGE_URL },
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

describe('hiddenByPageAnchor (main-side layout gate)', () => {
  afterEach(() => {
    pages.length = 0
  })

  it('shows the annotation while the page is on its URL, hides it after navigation', () => {
    showPage(PAGE_URL)
    expect(hiddenByPageAnchor(annotation({}))).toBe(false)

    showPage('https://example.com/about')
    expect(hiddenByPageAnchor(annotation({}))).toBe(true)
  })

  it('gates region annotations through their pageAnchor, not anchor internals', () => {
    showPage('https://example.com/about')
    expect(
      hiddenByPageAnchor(
        annotation({
          anchor: { type: 'region', canvasRect: { x: 0, y: 0, width: 10, height: 10 } },
        }),
      ),
    ).toBe(true)
  })

  it('never hides hash-only navigation, URL-less anchors, or missing pages', () => {
    showPage(`${PAGE_URL}#faq`)
    expect(hiddenByPageAnchor(annotation({}))).toBe(false)

    showPage('https://example.com/about')
    expect(
      hiddenByPageAnchor(annotation({ pageAnchor: { pageId: PAGE_ID } })),
    ).toBe(false)

    pages.length = 0
    expect(hiddenByPageAnchor(annotation({}))).toBe(false)
  })

  it('treats an annotation without a pageAnchor as canvas-bound — legacy metadata.pageUrl is not a binding', () => {
    showPage('https://example.com/about')
    // Legacy-shaped record: pre-pageAnchor files carry the URL in metadata.
    // That read is retired — the record loads and never hides.
    expect(
      hiddenByPageAnchor(
        annotation({ pageAnchor: undefined, metadata: { pageUrl: PAGE_URL } }),
      ),
    ).toBe(false)
    expect(
      hiddenByPageAnchor(
        annotation({
          pageAnchor: undefined,
          anchor: { type: 'canvas', canvasX: 1, canvasY: 2 },
        }),
      ),
    ).toBe(false)
  })

  it('applies the same gate to page-anchored entities', () => {
    const entity = { id: 'sticky-1', pageAnchor: { pageId: PAGE_ID, pageUrl: PAGE_URL } }
    showPage(PAGE_URL)
    expect(hiddenByPageAnchor(entity)).toBe(false)
    showPage('https://example.com/about')
    expect(hiddenByPageAnchor(entity)).toBe(true)
    expect(hiddenByPageAnchor({ id: 'sticky-1' })).toBe(false)
  })
})
