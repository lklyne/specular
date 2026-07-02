import { describe, expect, it } from 'vitest'
import {
  computeLayoutMetrics,
  computeLayoutPositions,
  computeRowReflow,
  type LayoutBox,
} from '../../src/main/layout-math'
import {
  resolveSpacing,
  SPACING_TOKEN_PIXELS,
  validateLayoutDirective,
} from '../../src/shared/layout-directive'

const ORIGIN = { x: 0, y: 0 }

describe('resolveSpacing', () => {
  it('maps tokens to pixels', () => {
    expect(resolveSpacing('xs', 0)).toBe(SPACING_TOKEN_PIXELS.xs)
    expect(resolveSpacing('m', 0)).toBe(SPACING_TOKEN_PIXELS.m)
    expect(resolveSpacing('xl', 0)).toBe(SPACING_TOKEN_PIXELS.xl)
  })
  it('passes through numbers unchanged', () => {
    expect(resolveSpacing(37, 0)).toBe(37)
  })
  it('falls back when undefined', () => {
    expect(resolveSpacing(undefined, 24)).toBe(24)
  })
})

describe('validateLayoutDirective', () => {
  it('accepts a minimal valid directive', () => {
    expect(validateLayoutDirective({ kind: 'row' })).toBeNull()
  })
  it('accepts the full surface', () => {
    expect(validateLayoutDirective({
      kind: 'grid', gap: 'm', rowGap: 24, colGap: 'l', cols: 3,
      originX: 100, originY: 200, near: 'page_x',
    })).toBeNull()
  })
  it('rejects bad kind', () => {
    expect(validateLayoutDirective({ kind: 'flex' })).toMatch(/layout\.kind/)
  })
  it('rejects bad spacing tokens', () => {
    expect(validateLayoutDirective({ kind: 'row', gap: 'huge' })).toMatch(/layout\.gap/)
  })
  it('rejects non-positive cols', () => {
    expect(validateLayoutDirective({ kind: 'grid', cols: 0 })).toMatch(/layout\.cols/)
    expect(validateLayoutDirective({ kind: 'grid', cols: 1.5 })).toMatch(/layout\.cols/)
  })
  it('requires originX and originY together', () => {
    expect(validateLayoutDirective({ kind: 'row', originX: 100 })).toMatch(/originX and originY/)
    expect(validateLayoutDirective({ kind: 'row', originY: 100 })).toMatch(/originX and originY/)
  })
  it('rejects non-string near', () => {
    expect(validateLayoutDirective({ kind: 'row', near: 42 })).toMatch(/layout\.near/)
  })
  it('rejects non-object', () => {
    expect(validateLayoutDirective(null)).toMatch(/expected an object/)
    expect(validateLayoutDirective('row')).toMatch(/expected an object/)
  })
})

describe('computeLayoutPositions — row', () => {
  it('places homogeneous items left-to-right with gap', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 80 },
      { width: 100, height: 80 },
      { width: 100, height: 80 },
    ]
    const positions = computeLayoutPositions(items, 'row', 16, 16, ORIGIN)
    expect(positions).toEqual([
      { canvasX: 0, canvasY: 0 },
      { canvasX: 116, canvasY: 0 },
      { canvasX: 232, canvasY: 0 },
    ])
  })

  it('flows heterogeneous widths by item width (not max)', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 80 },
      { width: 200, height: 80 },
      { width: 50, height: 80 },
    ]
    const positions = computeLayoutPositions(items, 'row', 10, 10, ORIGIN)
    expect(positions.map((p) => p.canvasX)).toEqual([0, 110, 320])
  })

  it('respects origin offset', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 80 },
      { width: 100, height: 80 },
    ]
    const positions = computeLayoutPositions(items, 'row', 16, 16, { x: 200, y: 50 })
    expect(positions).toEqual([
      { canvasX: 200, canvasY: 50 },
      { canvasX: 316, canvasY: 50 },
    ])
  })

  it('handles single item without gap', () => {
    const items: LayoutBox[] = [{ width: 300, height: 200 }]
    const positions = computeLayoutPositions(items, 'row', 24, 24, { x: 10, y: 20 })
    expect(positions).toEqual([{ canvasX: 10, canvasY: 20 }])
  })

  it('returns empty for empty items', () => {
    expect(computeLayoutPositions([], 'row', 16, 16, ORIGIN)).toEqual([])
  })
})

describe('computeLayoutPositions — column', () => {
  it('flows top-to-bottom by item height', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 80 },
      { width: 100, height: 120 },
      { width: 100, height: 60 },
    ]
    const positions = computeLayoutPositions(items, 'column', 10, 10, ORIGIN)
    expect(positions.map((p) => p.canvasY)).toEqual([0, 90, 220])
    expect(positions.every((p) => p.canvasX === 0)).toBe(true)
  })

  it('uses rowGap distinct from colGap', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 100 },
      { width: 100, height: 100 },
    ]
    const positions = computeLayoutPositions(items, 'column', 999, 50, ORIGIN)
    expect(positions[1].canvasY).toBe(150)
  })
})

describe('computeLayoutPositions — grid', () => {
  it('uses uniform tracks (max width × max height) for clean alignment', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 100 },
      { width: 200, height: 100 },
      { width: 100, height: 200 },
      { width: 100, height: 100 },
    ]
    const positions = computeLayoutPositions(items, 'grid', 10, 10, ORIGIN, 2)
    // Tracks: maxW=200, maxH=200. Cells start at (0,0), (210,0), (0,210), (210,210).
    expect(positions).toEqual([
      { canvasX: 0, canvasY: 0 },
      { canvasX: 210, canvasY: 0 },
      { canvasX: 0, canvasY: 210 },
      { canvasX: 210, canvasY: 210 },
    ])
  })

  it('fills row-major', () => {
    const items: LayoutBox[] = Array.from({ length: 6 }, () => ({ width: 100, height: 100 }))
    const positions = computeLayoutPositions(items, 'grid', 10, 10, ORIGIN, 3)
    // Indices 0,1,2 on row 0; 3,4,5 on row 1.
    expect(positions[0]).toEqual({ canvasX: 0, canvasY: 0 })
    expect(positions[2]).toEqual({ canvasX: 220, canvasY: 0 })
    expect(positions[3]).toEqual({ canvasX: 0, canvasY: 110 })
    expect(positions[5]).toEqual({ canvasX: 220, canvasY: 110 })
  })

  it('defaults cols to ceil(sqrt(n))', () => {
    const items: LayoutBox[] = Array.from({ length: 5 }, () => ({ width: 100, height: 100 }))
    const positions = computeLayoutPositions(items, 'grid', 10, 10, ORIGIN)
    // ceil(sqrt(5)) = 3 cols → indices 0,1,2 on row 0; 3,4 on row 1.
    expect(positions[2]).toEqual({ canvasX: 220, canvasY: 0 })
    expect(positions[3]).toEqual({ canvasX: 0, canvasY: 110 })
  })

  it('respects origin offset', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 100 },
      { width: 100, height: 100 },
    ]
    const positions = computeLayoutPositions(items, 'grid', 10, 10, { x: 50, y: 50 }, 2)
    expect(positions[0]).toEqual({ canvasX: 50, canvasY: 50 })
    expect(positions[1]).toEqual({ canvasX: 160, canvasY: 50 })
  })
})

describe('computeRowReflow', () => {
  it('returns empty for no children', () => {
    expect(computeRowReflow([], 80, 0, 0)).toEqual([])
  })

  it('places a single child at the origin', () => {
    expect(computeRowReflow([{ width: 300, height: 200 }], 80, 10, 20)).toEqual([
      { canvasX: 10, canvasY: 20 },
    ])
  })

  it('packs two children left-to-right separated by gap', () => {
    const children: LayoutBox[] = [
      { width: 100, height: 80 },
      { width: 150, height: 80 },
    ]
    expect(computeRowReflow(children, 80, 0, 0)).toEqual([
      { canvasX: 0, canvasY: 0 },
      { canvasX: 180, canvasY: 0 },
    ])
  })

  it('flows N heterogeneous widths by own width, sharing originY', () => {
    const children: LayoutBox[] = [
      { width: 100, height: 80 },
      { width: 200, height: 120 },
      { width: 50, height: 60 },
    ]
    const positions = computeRowReflow(children, 10, 5, 50)
    expect(positions.map((p) => p.canvasX)).toEqual([5, 115, 325])
    expect(positions.every((p) => p.canvasY === 50)).toBe(true)
  })

  it('treats a zero-width child as occupying only the gap', () => {
    const children: LayoutBox[] = [
      { width: 0, height: 0 },
      { width: 100, height: 80 },
    ]
    expect(computeRowReflow(children, 20, 0, 0).map((p) => p.canvasX)).toEqual([0, 20])
  })
})

describe('computeLayoutMetrics', () => {
  it('returns bbox for row', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 80 },
      { width: 100, height: 80 },
      { width: 100, height: 80 },
    ]
    const m = computeLayoutMetrics(items, 'row', 16, 16)
    expect(m.bbWidth).toBe(332)
    expect(m.bbHeight).toBe(80)
  })

  it('returns bbox for grid using uniform tracks', () => {
    const items: LayoutBox[] = [
      { width: 100, height: 100 },
      { width: 200, height: 100 },
      { width: 100, height: 200 },
      { width: 100, height: 100 },
    ]
    const m = computeLayoutMetrics(items, 'grid', 10, 10, 2)
    // 2 cols × 200 + 1 × 10 = 410; 2 rows × 200 + 1 × 10 = 410
    expect(m.bbWidth).toBe(410)
    expect(m.bbHeight).toBe(410)
    expect(m.cols).toBe(2)
  })

  it('handles empty', () => {
    const m = computeLayoutMetrics([], 'row', 16, 16)
    expect(m).toEqual({ cols: 0, maxW: 0, maxH: 0, bbWidth: 0, bbHeight: 0 })
  })
})
