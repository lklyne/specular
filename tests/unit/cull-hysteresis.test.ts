import { describe, expect, it } from 'vitest'
import { pageVisibleWithHysteresis } from '../../src/main/runtime/runtime-geometry'

// Viewport at origin, 1000×800. Show margin 48, keep margin 256 — the dead
// band is the 208px between them where an edge-hovering page must NOT toggle.
const VIEWPORT = { x: 0, y: 0, width: 1000, height: 800 }
const SHOW = 48
const KEEP = 256

function pageAt(x: number): { x: number; y: number; width: number; height: number } {
  return { x, y: 0, width: 100, height: 100 }
}

describe('pageVisibleWithHysteresis', () => {
  it('shows a page overlapping the viewport regardless of prior state', () => {
    expect(pageVisibleWithHysteresis(pageAt(500), VIEWPORT, false, SHOW, KEEP)).toBe(true)
    expect(pageVisibleWithHysteresis(pageAt(500), VIEWPORT, true, SHOW, KEEP)).toBe(true)
  })

  it('keeps a visible page alive out to the keep margin', () => {
    // Right edge of viewport is x=1000. A 100px page at x=1100 sits 100px past
    // it — inside the 256px keep band, so a visible page stays visible.
    expect(pageVisibleWithHysteresis(pageAt(1100), VIEWPORT, false, SHOW, KEEP)).toBe(true)
    // At x=1300 it is 300px past — beyond keep — so it culls.
    expect(pageVisibleWithHysteresis(pageAt(1300), VIEWPORT, false, SHOW, KEEP)).toBe(false)
  })

  it('reveals a culled page only inside the smaller show margin', () => {
    // Same 100px-past-edge page: a culled page stays culled (beyond show=48)…
    expect(pageVisibleWithHysteresis(pageAt(1100), VIEWPORT, true, SHOW, KEEP)).toBe(false)
    // …and only reappears once within the show margin of the viewport.
    expect(pageVisibleWithHysteresis(pageAt(1020), VIEWPORT, true, SHOW, KEEP)).toBe(true)
  })

  it('does not toggle in the dead band (prior state persists)', () => {
    const deadBandPage = pageAt(1100) // 100px past edge: > show, < keep
    // Visible stays visible; culled stays culled — no per-tick flip.
    expect(pageVisibleWithHysteresis(deadBandPage, VIEWPORT, false, SHOW, KEEP)).toBe(true)
    expect(pageVisibleWithHysteresis(deadBandPage, VIEWPORT, true, SHOW, KEEP)).toBe(false)
  })
})
