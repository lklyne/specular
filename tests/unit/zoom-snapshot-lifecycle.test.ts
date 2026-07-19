import { describe, expect, it } from 'vitest'
import { snapshotCaptureStillValid } from '../../src/shared/zoom-snapshot-lifecycle'

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
