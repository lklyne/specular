import { describe, expect, it } from 'vitest'
import {
  CAMERA_SPRING_CSS_EASING,
  cameraSpring,
  easeOutCubic,
  interpolateCamera,
  sampledCameraSpringCss,
} from '../../src/shared/camera-transition'

describe('camera transition helpers', () => {
  const start = { zoom: 1, pan: { x: 0, y: 0 } }
  const target = { zoom: 0.5, pan: { x: 100, y: -50 } }

  it('returns the exact start and target at the endpoints', () => {
    expect(interpolateCamera(start, target, 0)).toEqual(start)
    expect(interpolateCamera(start, target, 1)).toEqual(target)
  })

  it('interpolates zoom and rounded pan through the supplied easing', () => {
    const mid = interpolateCamera(start, target, 0.5, (t) => t)

    expect(mid.zoom).toBe(0.75)
    expect(mid.pan).toEqual({ x: 50, y: -25 })
  })

  it('uses the shared spring by default', () => {
    const mid = interpolateCamera(start, target, 0.5)

    expect(mid.zoom).toBeCloseTo(1 + (target.zoom - 1) * cameraSpring(0.5))
    expect(mid.pan.x).toBeGreaterThan(50)
  })

  it('can still interpolate with the legacy cubic ease', () => {
    const mid = interpolateCamera(start, target, 0.5, easeOutCubic)

    expect(mid.zoom).toBeCloseTo(1 + (target.zoom - 1) * easeOutCubic(0.5))
    expect(mid.pan.x).toBeGreaterThan(50)
  })

  it('exports the same sampled spring for renderer animations', () => {
    expect(cameraSpring(0)).toBe(0)
    expect(cameraSpring(1)).toBe(1)
    expect(cameraSpring(0.5)).toBeGreaterThan(0.9)
    expect(CAMERA_SPRING_CSS_EASING).toBe(sampledCameraSpringCss())
    expect(CAMERA_SPRING_CSS_EASING).toMatch(/^linear\(0, /)
  })

  it('clamps invalid progress into the animation range', () => {
    expect(interpolateCamera(start, target, -1)).toEqual(start)
    expect(interpolateCamera(start, target, Number.NaN)).toEqual(target)
    expect(interpolateCamera(start, target, 2)).toEqual(target)
  })
})
