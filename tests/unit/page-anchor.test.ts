/**
 * Pure page-anchor resolution (shared/page-anchor.ts): an entity anchors to
 * the topmost page whose body contains its center, and anchors record the
 * canonical (hash-stripped) page URL.
 *
 * Mutation-verified by: (1) iterating `pages` forward instead of reverse in
 * `pageAnchorFor` — the topmost-wins case fails; (2) testing bounds overlap
 * instead of center containment — the "straddles the edge" case fails.
 */

import { describe, expect, it } from 'vitest'
import {
  matchesPageUrl,
  pageAnchorFor,
  pageAnchorOnCurrentUrl,
} from '../../src/shared/page-anchor'

const PAGE_A = {
  id: 'page-a',
  url: 'https://example.com/a#hero',
  bounds: { x: 0, y: 0, width: 400, height: 600 },
}
const PAGE_B_OVERLAPPING = {
  id: 'page-b',
  url: 'https://example.com/b',
  bounds: { x: 200, y: 0, width: 400, height: 600 },
}

describe('pageAnchorFor', () => {
  it('anchors to the page containing the entity center, recording the canonical URL', () => {
    const anchor = pageAnchorFor({ x: 50, y: 50, width: 100, height: 100 }, [PAGE_A])
    expect(anchor).toEqual({ pageId: 'page-a', pageUrl: 'https://example.com/a' })
  })

  it('returns null on empty canvas', () => {
    const anchor = pageAnchorFor({ x: 1000, y: 1000, width: 100, height: 100 }, [PAGE_A])
    expect(anchor).toBeNull()
  })

  it('anchors an entity straddling the page edge by its center', () => {
    // Center at (450, 100) — outside page-a (right edge 400) even though the
    // entity's left half overlaps it.
    expect(
      pageAnchorFor({ x: 350, y: 50, width: 200, height: 100 }, [PAGE_A]),
    ).toBeNull()
    // Center at (390, 100) — inside.
    expect(
      pageAnchorFor({ x: 290, y: 50, width: 200, height: 100 }, [PAGE_A])?.pageId,
    ).toBe('page-a')
  })

  it('prefers the topmost page when pages overlap (last in stack order wins)', () => {
    const anchor = pageAnchorFor({ x: 250, y: 50, width: 100, height: 100 }, [
      PAGE_A,
      PAGE_B_OVERLAPPING,
    ])
    expect(anchor?.pageId).toBe('page-b')
  })

  it('omits pageUrl when the page has none', () => {
    const anchor = pageAnchorFor({ x: 50, y: 50, width: 100, height: 100 }, [
      { ...PAGE_A, url: '' },
    ])
    expect(anchor).toEqual({ pageId: 'page-a' })
  })
})

describe('pageAnchorOnCurrentUrl', () => {
  it('matches while the page shows the anchor URL (hash-insensitive), rejects other documents', () => {
    const anchor = { pageId: 'page-a', pageUrl: 'https://example.com/a' }
    expect(pageAnchorOnCurrentUrl(anchor, 'https://example.com/a#pricing')).toBe(true)
    expect(pageAnchorOnCurrentUrl(anchor, 'https://example.com/other')).toBe(false)
    expect(pageAnchorOnCurrentUrl({ pageId: 'page-a' }, 'https://example.com/other')).toBe(true)
    expect(pageAnchorOnCurrentUrl(undefined, 'https://example.com/a')).toBe(true)
  })
})

describe('matchesPageUrl', () => {
  it('treats a missing side as a match and compares canonically otherwise', () => {
    expect(matchesPageUrl(undefined, 'https://example.com/a')).toBe(true)
    expect(matchesPageUrl('https://example.com/a', '')).toBe(true)
    expect(matchesPageUrl('https://example.com/a#x', 'https://example.com/a#y')).toBe(true)
    expect(matchesPageUrl('https://example.com/a?tab=1', 'https://example.com/a?tab=2')).toBe(false)
  })
})
