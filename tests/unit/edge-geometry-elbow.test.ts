/**
 * Elbow routing and per-endpoint side resolution.
 *
 * Mutation-verified by (a) hardcoding the split ratio to 0.5 in
 * buildElbowPoints — the stored-split cases then fail; (b) dropping the
 * `split.axis === adjustable` guard — the degenerate-L case then bends;
 * (c) removing the half-shorter-segment clamp in elbowPathFromPoints — the
 * corner-radius case then rounds past the segment midpoint.
 */
import { describe, expect, it } from 'vitest'
import {
  autoSide,
  buildElbowPoints,
  elbowPathFromPoints,
  resolveEdgeSides,
} from '../../src/shared/edge-geometry'
import type { AnchorPoint } from '../../src/shared/edge-geometry'
import type { CanvasSceneEntity, EdgeSide } from '../../src/shared/types'

function anchor(x: number, y: number, side: EdgeSide): AnchorPoint {
  return { x, y, side }
}

function box(id: string, x: number, y: number): CanvasSceneEntity {
  return {
    id,
    kind: 'page',
    canvasX: x,
    canvasY: y,
    width: 100,
    height: 100,
    screenX: x,
    screenY: y,
    screenWidth: 100,
    screenHeight: 100,
  } as CanvasSceneEntity
}

describe('buildElbowPoints', () => {
  it('splits at the midpoint for two facing sides (3-segment route)', () => {
    const points = buildElbowPoints(anchor(0, 0, 'right'), anchor(200, 100, 'left'))
    expect(points).toEqual([
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 100, y: 100 },
      { x: 200, y: 100 },
    ])
  })

  it('routes perpendicular sides as a 2-segment L', () => {
    const points = buildElbowPoints(anchor(0, 0, 'right'), anchor(200, 100, 'top'))
    expect(points).toEqual([
      { x: 0, y: 0 },
      { x: 200, y: 0 },
      { x: 200, y: 100 },
    ])
  })

  it('routes sides facing away from each other as a 5-segment S', () => {
    const points = buildElbowPoints(anchor(0, 0, 'left'), anchor(200, 100, 'right'), 1)
    expect(points).toEqual([
      { x: 0, y: 0 },
      { x: -24, y: 0 },
      { x: -24, y: 50 },
      { x: 224, y: 50 },
      { x: 224, y: 100 },
      { x: 200, y: 100 },
    ])
  })

  it('honours a stored split on the facing axis', () => {
    const points = buildElbowPoints(anchor(0, 0, 'right'), anchor(200, 100, 'left'), 1, {
      value: 0.85,
      axis: 'x',
    })
    expect(points[1]).toEqual({ x: 170, y: 0 })
    expect(points[2]).toEqual({ x: 170, y: 100 })
  })

  it('holds the split on its own axis when the far end resolves perpendicular', () => {
    // Crossbar dragged to 85% across while the pair was side by side; the target
    // then moves below, so its auto end rederives to `top`. The offset stays on
    // x and the route becomes 4-segment: out, cross, along, in.
    const points = buildElbowPoints(anchor(0, 0, 'right'), anchor(200, 400, 'top'), 1, {
      value: 0.85,
      axis: 'x',
    })
    expect(points).toEqual([
      { x: 0, y: 0 },
      { x: 170, y: 0 },
      { x: 170, y: 376 },
      { x: 200, y: 376 },
      { x: 200, y: 400 },
    ])
  })

  it('ignores a split stored on the other axis, leaving a plain L', () => {
    const split = { value: 0.85, axis: 'y' } as const
    const points = buildElbowPoints(anchor(0, 0, 'right'), anchor(200, 100, 'top'), 1, split)
    expect(points).toEqual([
      { x: 0, y: 0 },
      { x: 200, y: 0 },
      { x: 200, y: 100 },
    ])
    // The builder is pure: the caller's stored split is untouched, so it applies
    // again if the entities move back.
    expect(split).toEqual({ value: 0.85, axis: 'y' })
  })
})

describe('elbowPathFromPoints', () => {
  it('clamps the corner radius to half the shorter adjacent segment', () => {
    // Second segment is 10px long, so the corner may only round 5px.
    const d = elbowPathFromPoints([
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 100, y: 10 },
      { x: 200, y: 10 },
    ])
    expect(d).toBe(
      'M 0 0 L 95 0 Q 100 0 100 5 L 100 5 Q 100 10 105 10 L 200 10',
    )
  })
})

describe('per-endpoint side resolution', () => {
  it('keeps a pinned side while the other end rederives', () => {
    const from = box('a', 0, 0)
    const to = box('b', 400, 0)
    expect(resolveEdgeSides(from, to, { fromSide: 'top' })).toEqual({
      fromSide: 'top',
      toSide: 'left',
    })
    expect(resolveEdgeSides(from, to)).toEqual({ fromSide: 'right', toSide: 'left' })
  })

  it('resolves one end against a bare point', () => {
    expect(autoSide(box('a', 0, 0), { x: 50, y: 500 })).toBe('bottom')
    expect(autoSide(box('a', 0, 0), { x: -500, y: 50 })).toBe('left')
  })
})
