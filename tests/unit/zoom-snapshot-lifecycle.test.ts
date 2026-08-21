import { describe, expect, it } from 'vitest'
import {
  frameMeetsTarget,
  pickBetterFrame,
  snapshotCaptureStillValid,
  snapshotTargetScale,
} from '../../src/shared/zoom-snapshot-lifecycle'

describe('zoom snapshot capture lifecycle', () => {
  it('accepts a capture when neither the gesture lease nor page state changed', () => {
    expect(
      snapshotCaptureStillValid({
        captureLeaseAtStart: 4,
        currentCaptureLease: 4,
        signatureAtStart: 'page:bounds:scroll-0',
        currentSignature: 'page:bounds:scroll-0',
      }),
    ).toBe(true)
  })

  it('discards a capture when a zoom begins while capturePage is in flight', () => {
    expect(
      snapshotCaptureStillValid({
        captureLeaseAtStart: 4,
        currentCaptureLease: 5,
        signatureAtStart: 'page:bounds:scroll-0',
        currentSignature: 'page:bounds:scroll-0',
      }),
    ).toBe(false)
  })

  it('discards a capture when native page scroll changes in flight', () => {
    expect(
      snapshotCaptureStillValid({
        captureLeaseAtStart: 4,
        currentCaptureLease: 4,
        signatureAtStart: 'page:bounds:scroll-0',
        currentSignature: 'page:bounds:scroll-400',
      }),
    ).toBe(false)
  })
})

describe('snapshotTargetScale', () => {
  it('puts the long edge at the pixel cap, bounded by max zoom', () => {
    expect(snapshotTargetScale({ zoom: 0.1, cssWidth: 1024, cssHeight: 640, devicePixelRatio: 2, maxZoom: 3 })).toBe(1)
    expect(snapshotTargetScale({ zoom: 0.1, cssWidth: 393, cssHeight: 852, devicePixelRatio: 2, maxZoom: 3 })).toBeCloseTo(1.2019, 3)
    expect(snapshotTargetScale({ zoom: 0.1, cssWidth: 100, cssHeight: 100, devicePixelRatio: 2, maxZoom: 3 })).toBe(3)
  })

  it('never asks for less than the live zoom', () => {
    expect(snapshotTargetScale({ zoom: 2.5, cssWidth: 1440, cssHeight: 900, devicePixelRatio: 2, maxZoom: 3 })).toBe(2.5)
  })
})

describe('pickBetterFrame', () => {
  const big = { contentKey: 'a', capturedWidth: 2000, capturedHeight: 1000 }
  const small = { contentKey: 'a', capturedWidth: 200, capturedHeight: 100 }

  it('keeps the higher-resolution frame of the same content', () => {
    expect(pickBetterFrame(big, small)).toBe(big)
    expect(pickBetterFrame(small, big)).toBe(big)
  })

  it('always takes the incoming frame when the content changed', () => {
    const changed = { ...small, contentKey: 'b' }
    expect(pickBetterFrame(big, changed)).toBe(changed)
    expect(pickBetterFrame(undefined, small)).toBe(small)
  })
})

describe('frameMeetsTarget', () => {
  const target = { contentKey: 'a', cssWidth: 1000, devicePixelRatio: 2, targetScale: 1 }
  it('is true only for matching content with at least the target width', () => {
    expect(frameMeetsTarget({ contentKey: 'a', capturedWidth: 2000, capturedHeight: 1 }, target)).toBe(true)
    expect(frameMeetsTarget({ contentKey: 'a', capturedWidth: 1999, capturedHeight: 1 }, target)).toBe(false)
    expect(frameMeetsTarget({ contentKey: 'b', capturedWidth: 4000, capturedHeight: 1 }, target)).toBe(false)
    expect(frameMeetsTarget(undefined, target)).toBe(false)
  })
})
