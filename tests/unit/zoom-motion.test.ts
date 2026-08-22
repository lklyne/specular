import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  isZoomInMotion,
  markZoomMotion,
  quantizeZoomForEmulation,
} from '../../src/main/runtime/zoom-motion'

afterEach(() => {
  vi.useRealTimers()
})

describe('quantizeZoomForEmulation', () => {
  it('returns a positive number for positive input', () => {
    expect(quantizeZoomForEmulation(0.37)).toBeGreaterThan(0)
    expect(quantizeZoomForEmulation(4.2)).toBeGreaterThan(0)
  })

  it('is monotonic non-decreasing across a sweep', () => {
    let prev = -Infinity
    for (let zoom = 0.1; zoom <= 10; zoom += 0.05) {
      const quantized = quantizeZoomForEmulation(zoom)
      expect(quantized).toBeGreaterThanOrEqual(prev)
      prev = quantized
    }
  })

  it('snaps nearby inputs onto a shared coarse grid', () => {
    const inputs: number[] = []
    for (let zoom = 0.9; zoom <= 1.1; zoom += 0.002) inputs.push(zoom)
    const distinct = new Set(inputs.map(quantizeZoomForEmulation))
    expect(distinct.size).toBeLessThan(inputs.length / 10)
  })

  it('passes zoom <= 0 through unchanged', () => {
    expect(quantizeZoomForEmulation(0)).toBe(0)
    expect(quantizeZoomForEmulation(-1)).toBe(-1)
  })
})

describe('zoom motion lifecycle', () => {
  it('does not settle inside a fast trackpad gesture burst gap', () => {
    vi.useFakeTimers()
    const onSettle = vi.fn()

    markZoomMotion(onSettle)
    vi.advanceTimersByTime(250)

    expect(isZoomInMotion()).toBe(true)
    expect(onSettle).not.toHaveBeenCalled()

    markZoomMotion(onSettle)
    vi.advanceTimersByTime(299)
    expect(onSettle).not.toHaveBeenCalled()

    vi.advanceTimersByTime(1)
    expect(isZoomInMotion()).toBe(false)
    expect(onSettle).toHaveBeenCalledOnce()
  })
})
