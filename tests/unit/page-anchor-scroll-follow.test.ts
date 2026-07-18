import { describe, expect, it } from 'vitest'
import { shouldFastFollowPageScroll } from '../../src/shared/page-anchor'

describe('shouldFastFollowPageScroll', () => {
  it('uses the fast document-scroll follower for ordinary page attachments', () => {
    expect(
      shouldFastFollowPageScroll({
        pageId: 'page-1',
        scrollY: 0,
        element: { selector: '#article', docX: 0, docY: 100 },
      }),
    ).toBe(true)
  })

  it('leaves fixed and sticky attachments to the authoritative element projection', () => {
    expect(
      shouldFastFollowPageScroll({
        pageId: 'page-1',
        scrollY: 0,
        element: {
          selector: '#rail',
          docX: 0,
          docY: 100,
          viewportPositioned: true,
        },
      }),
    ).toBe(false)
  })

  it('heals legacy anchors from the live fixed/sticky classification', () => {
    expect(
      shouldFastFollowPageScroll(
        {
          pageId: 'page-1',
          scrollY: 0,
          element: { selector: '#rail', docX: 0, docY: 100 },
        },
        { viewportPositioned: true },
      ),
    ).toBe(false)
  })
})
