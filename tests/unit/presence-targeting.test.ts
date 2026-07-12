import { describe, it, expect } from 'vitest'
import {
  resolvePresencePagePoint,
  pagePointMatchesTargetRect,
  shouldSkipReposition,
} from '../../src/shared/presence-targeting'

describe('resolvePresencePagePoint', () => {
  it('prefers explicit page coordinates', () => {
    const result = resolvePresencePagePoint({
      pageX: 100,
      pageY: 200,
      targetRect: { x: 10, y: 20, width: 50, height: 50 },
      fallbackX: 0,
      fallbackY: 0,
    })
    expect(result).toEqual({ x: 100, y: 200 })
  })

  it('falls back to target rect center when no page coordinates', () => {
    const result = resolvePresencePagePoint({
      pageX: null,
      pageY: null,
      targetRect: { x: 10, y: 20, width: 50, height: 60 },
      fallbackX: 0,
      fallbackY: 0,
    })
    expect(result).toEqual({ x: 35, y: 50 })
  })

  it('mixes partial page coordinates with fallback values', () => {
    const result = resolvePresencePagePoint({
      pageX: null,
      pageY: 20,
      targetRect: null,
      fallbackX: 500,
      fallbackY: 300,
    })
    expect(result).toEqual({ x: 500, y: 20 })
  })

  it('falls back to fallback coordinates when nothing else available', () => {
    const result = resolvePresencePagePoint({
      pageX: null,
      pageY: null,
      targetRect: null,
      fallbackX: 500,
      fallbackY: 300,
    })
    expect(result).toEqual({ x: 500, y: 300 })
  })
})

describe('pagePointMatchesTargetRect', () => {
  const rect = { x: 10, y: 20, width: 100, height: 50 }

  it('returns true when point is inside rect', () => {
    expect(pagePointMatchesTargetRect(50, 40, rect)).toBe(true)
  })

  it('returns true at rect edges within tolerance', () => {
    expect(pagePointMatchesTargetRect(10, 20, rect)).toBe(true)
    expect(pagePointMatchesTargetRect(110, 70, rect)).toBe(true)
  })

  it('returns false when point is outside rect and tolerance', () => {
    expect(pagePointMatchesTargetRect(5, 15, rect)).toBe(false)
    expect(pagePointMatchesTargetRect(200, 200, rect)).toBe(false)
  })

  it('returns true when pageX or pageY is null', () => {
    expect(pagePointMatchesTargetRect(null, 40, rect)).toBe(true)
    expect(pagePointMatchesTargetRect(50, null, rect)).toBe(true)
  })

  it('returns true when targetRect is null', () => {
    expect(pagePointMatchesTargetRect(50, 40, null)).toBe(true)
  })

  it('respects custom tolerance', () => {
    expect(pagePointMatchesTargetRect(5, 20, rect, 10)).toBe(true)
    expect(pagePointMatchesTargetRect(5, 20, rect, 1)).toBe(false)
  })
})

// shouldSkipReposition is the "already on the target, no need to re-travel"
// check the CDP proxy inlined twice (box-model pre-move and the mousePressed
// skip check) before it was extracted here (ADR 0029). Skip is within-rect
// only: a canvas-space distance heuristic suppressed travel more the further
// the canvas was zoomed out, so every hop to a distinct element now animates
// (#319).
describe('shouldSkipReposition', () => {
  const rect = { x: 10, y: 20, width: 100, height: 50 }

  it('skips when the click point is inside the target rect', () => {
    expect(shouldSkipReposition({ clickPoint: { x: 50, y: 40 }, targetRect: rect })).toBe(true)
  })

  it('skips when the click point sits exactly on the rect edge', () => {
    expect(
      shouldSkipReposition({ clickPoint: { x: rect.x, y: rect.y }, targetRect: rect }),
    ).toBe(true)
  })

  it('does not skip when the click point is outside the rect', () => {
    expect(shouldSkipReposition({ clickPoint: { x: 500, y: 500 }, targetRect: rect })).toBe(false)
  })

  it('does not skip a null targetRect — a rect-less target always travels', () => {
    expect(shouldSkipReposition({ clickPoint: { x: 50, y: 40 }, targetRect: null })).toBe(false)
  })
})
