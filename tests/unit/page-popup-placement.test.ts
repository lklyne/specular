/**
 * Mutation-verified by dropping the flip-above branch in `placePopup`
 * (always `y = anchor.y + anchor.height`): "flips above" fails.
 */
import { describe, expect, it } from 'vitest'
import { placePopup } from '../../src/renderer/canvas-bg/PageTextureSurface'
import { POPUP_CLOSE_GRACE_MS, popupHasClosed } from '../../src/renderer/canvas-bg/usePageFrames'

const viewport = { width: 1280, height: 800 }

describe('placePopup', () => {
  it('hangs below the anchor, left-aligned', () => {
    expect(placePopup({ x: 24, y: 100, width: 200, height: 30 }, { width: 220, height: 280 }, viewport)).toEqual({
      x: 24,
      y: 130,
    })
  })

  it('flips above when there is no room below', () => {
    expect(placePopup({ x: 24, y: 700, width: 200, height: 30 }, { width: 220, height: 280 }, viewport)).toEqual({
      x: 24,
      y: 420,
    })
  })

  it('slides left to stay inside the viewport', () => {
    expect(placePopup({ x: 1200, y: 100, width: 60, height: 30 }, { width: 220, height: 280 }, viewport)).toEqual({
      x: 1060,
      y: 130,
    })
  })
})

describe('popupHasClosed', () => {
  const popup = (closingSince: number | null) =>
    ({ closingSince }) as Parameters<typeof popupHasClosed>[0]

  it('stays open while the page has not painted after it', () => {
    expect(popupHasClosed(popup(null), 10_000)).toBe(false)
  })

  it('closes once a page paint goes unanswered past the grace window', () => {
    expect(popupHasClosed(popup(1000), 1000 + POPUP_CLOSE_GRACE_MS - 1)).toBe(false)
    expect(popupHasClosed(popup(1000), 1000 + POPUP_CLOSE_GRACE_MS + 1)).toBe(true)
  })
})
