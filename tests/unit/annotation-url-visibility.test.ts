/**
 * Page-anchored annotations hide when their page navigates away from the URL
 * they were created on (comment badges key off the scene page's live `url`).
 *
 * Mutation-verified by: (1) removing the `annotationMatchesPageUrl` skip in
 * `commentBadgesForLayout` — the "hides badges after navigation" cases fail;
 * (2) deleting the `parsed.hash = ''` line in `canonicalAnnotationUrl` — the
 * hash-insensitivity cases fail.
 */

import { describe, expect, it } from 'vitest'
import { commentBadgesForLayout } from '../../src/renderer/above-view/CommentBadgesLayer'
import type { AnnotationLiveBboxLookup } from '../../src/renderer/above-view/annotationMath'
import {
  annotationContextPageId,
  annotationMatchesPageUrl,
  canonicalAnnotationUrl,
} from '../../src/shared/annotation-utils'
import type { Annotation, LayoutUpdateData } from '../../src/shared/types'

const PAGE_URL = 'https://example.com/pricing'

const PAGE = {
  id: 'page-1',
  kind: 'page' as const,
  url: PAGE_URL,
  screenX: 200,
  screenY: 100,
  screenWidth: 400,
  screenHeight: 300,
  width: 400,
  height: 300,
}

function layout(pageUrl: string): LayoutUpdateData {
  return {
    canvasOrigin: { x: 0, y: 50 },
    pan: { x: 0, y: 0 },
    zoom: 1,
    entities: [{ ...PAGE, url: pageUrl }],
  } as LayoutUpdateData
}

function annotation(partial: Partial<Annotation>): Annotation {
  return {
    id: 'ann-1',
    anchor: {
      type: 'element',
      pageId: PAGE.id,
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

const noLiveBboxes: AnnotationLiveBboxLookup = {
  get: () => undefined,
  isStale: () => false,
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
    expect(annotationContextPageId(annotation({}))).toBe(PAGE.id)
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

describe('commentBadgesForLayout page-URL gating', () => {
  it('shows the badge while the page is on the annotation URL', () => {
    const badges = commentBadgesForLayout([annotation({})], layout(PAGE_URL), noLiveBboxes)
    expect(badges).toHaveLength(1)
  })

  it('hides element badges after the page navigates away', () => {
    const badges = commentBadgesForLayout(
      [annotation({})],
      layout('https://example.com/about'),
      noLiveBboxes,
    )
    expect(badges).toHaveLength(0)
  })

  it('hides page-anchored badges after the page navigates away', () => {
    const pageAnchored = annotation({
      anchor: { type: 'page', pageId: PAGE.id, offsetX: 0.5, offsetY: 0.5 },
    })
    expect(
      commentBadgesForLayout([pageAnchored], layout(PAGE_URL), noLiveBboxes),
    ).toHaveLength(1)
    expect(
      commentBadgesForLayout([pageAnchored], layout('https://example.com/about'), noLiveBboxes),
    ).toHaveLength(0)
  })

  it('keeps hash-only navigation and URL-less annotations visible', () => {
    expect(
      commentBadgesForLayout([annotation({})], layout(`${PAGE_URL}#faq`), noLiveBboxes),
    ).toHaveLength(1)
    expect(
      commentBadgesForLayout(
        [annotation({ metadata: undefined })],
        layout('https://example.com/about'),
        noLiveBboxes,
      ),
    ).toHaveLength(1)
  })
})
