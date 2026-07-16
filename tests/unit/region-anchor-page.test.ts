/**
 * A region binds to a page geometrically: the page body covering the largest
 * share of the region's area, provided that share is a majority. This is the
 * signal region select passes as `anchorPageId` — it replaces the old proxy
 * of "grabbed an interactive element", which left regions drawn over static
 * page content canvas-anchored.
 *
 * Mutation-verified by: changing the `> 0.5` threshold to `> 0` in
 * regionAnchorPageId — the edge-brush case fails; hard-coding the first
 * intersecting page — the largest-overlap-wins case fails.
 */

import { describe, expect, it } from 'vitest'
import { regionAnchorPageId } from '../../src/main/runtime/region-select'
import type { Page } from '../../src/main/runtime/runtime-entities'
import type { WorkspaceBounds } from '../../src/shared/types'

function page(id: string, bounds: WorkspaceBounds): { page: Page; bounds: WorkspaceBounds } {
  return { page: { id } as unknown as Page, bounds }
}

function anchorFor(
  rect: WorkspaceBounds,
  entries: { page: Page; bounds: WorkspaceBounds }[],
): string | undefined {
  const boundsById = new Map(entries.map((e) => [e.page.id, e.bounds]))
  return regionAnchorPageId(
    rect,
    entries.map((e) => e.page),
    (p) => boundsById.get(p.id)!,
  )
}

describe('regionAnchorPageId', () => {
  const host = page('host', { x: 0, y: 0, width: 1000, height: 800 })

  it('binds a region drawn mostly over a page body', () => {
    expect(anchorFor({ x: 100, y: 100, width: 200, height: 200 }, [host])).toBe('host')
  })

  it('binds a region hanging partially off the page while the majority overlaps', () => {
    // 60% of the region's width is over the page.
    expect(anchorFor({ x: 880, y: 100, width: 200, height: 200 }, [host])).toBe('host')
  })

  it('does not bind a region that merely brushes a page edge', () => {
    // Only 10% of the region overlaps the page.
    expect(anchorFor({ x: 980, y: 100, width: 200, height: 200 }, [host])).toBeUndefined()
  })

  it('binds to the page with the largest overlap when two intersect', () => {
    const left = page('left', { x: 0, y: 0, width: 500, height: 800 })
    const right = page('right', { x: 500, y: 0, width: 500, height: 800 })
    // Region straddles the seam, 75% over the right page.
    expect(
      anchorFor({ x: 450, y: 100, width: 200, height: 200 }, [left, right]),
    ).toBe('right')
  })

  it('never binds a degenerate (zero-area) region', () => {
    expect(anchorFor({ x: 100, y: 100, width: 0, height: 0 }, [host])).toBeUndefined()
  })
})
