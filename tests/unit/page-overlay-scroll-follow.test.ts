import { describe, expect, it } from 'vitest'
import {
  scrollFollowTransform,
  type PageScrollOffset,
} from '../../src/renderer/above-view/page-overlay-scroll-follow'

const baseline = (scrollY: number): PageScrollOffset => ({
  pageId: 'page-1',
  scrollX: 0,
  scrollY,
})

describe('page overlay scroll follower', () => {
  it('expresses only the residual between live scroll and the incorporated layout', () => {
    const live = baseline(110)

    expect(scrollFollowTransform(live, baseline(100), { x: 1, y: 1 })).toBe(
      'translate(0px, -10px)',
    )
    expect(scrollFollowTransform(live, baseline(110), { x: 1, y: 1 })).toBe('')
    expect(scrollFollowTransform(baseline(115), baseline(110), { x: 1, y: 1 })).toBe(
      'translate(0px, -5px)',
    )
  })

  it('scales page CSS-pixel offsets into the overlay coordinate system', () => {
    expect(
      scrollFollowTransform(
        { pageId: 'page-1', scrollX: 12, scrollY: 20 },
        { pageId: 'page-1', scrollX: 2, scrollY: 4 },
        { x: 0.5, y: 2 },
      ),
    ).toBe('translate(-5px, -32px)')
  })

  it('ignores a live sample belonging to another page', () => {
    expect(
      scrollFollowTransform(
        { pageId: 'page-2', scrollX: 0, scrollY: 50 },
        baseline(10),
        { x: 1, y: 1 },
      ),
    ).toBe('')
  })
})
