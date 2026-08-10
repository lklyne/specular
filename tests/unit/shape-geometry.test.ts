import { describe, expect, it } from 'vitest'
import { CORNER_RADIUS, shapeDef, shapeRender } from '../../src/shared/shapes'

// The corner radius a path's arcs were drawn with — every `A r,r` in the path
// carries it, and they must all agree (corners scale evenly or not at all).
function arcRadius(d: string): number {
  const radii = [...d.matchAll(/A(-?[\d.]+),(-?[\d.]+)/g)].map((m) => Number(m[1]))
  expect(radii.length).toBeGreaterThan(0)
  expect(new Set(radii).size).toBe(1)
  return radii[0]
}

describe('rounded rectangle geometry', () => {
  const rounded = shapeDef('rounded')

  it('keeps the corner radius fixed while the flat edges take the resize', () => {
    expect(arcRadius(shapeRender(rounded, 160, 160).d)).toBe(CORNER_RADIUS)
    expect(arcRadius(shapeRender(rounded, 600, 120).d)).toBe(CORNER_RADIUS)
    expect(arcRadius(shapeRender(rounded, 90, 800).d)).toBe(CORNER_RADIUS)
  })

  it('scales the radius evenly once the box has no flat edge left', () => {
    expect(arcRadius(shapeRender(rounded, 30, 400).d)).toBe(15)
    expect(arcRadius(shapeRender(rounded, 400, 24).d)).toBe(12)
  })

  it('draws at true size, so the viewBox matches the box and nothing stretches', () => {
    expect(shapeRender(rounded, 600, 120).viewBox).toBe('0 0 600 120')
  })

  it('caps the pill at half its short side, at every aspect ratio', () => {
    const pill = shapeDef('pill')
    expect(arcRadius(shapeRender(pill, 400, 100).d)).toBe(50)
    expect(arcRadius(shapeRender(pill, 100, 400).d)).toBe(50)
  })

  it('stretches kinds without a geometry builder from the normalized box', () => {
    const diamond = shapeDef('diamond')
    const render = shapeRender(diamond, 600, 120)
    expect(render.viewBox).toBe('0 0 100 100')
    expect(render.d).toBe(diamond.path)
  })
})
