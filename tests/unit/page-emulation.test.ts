import { describe, expect, it } from 'vitest'
import type { WebContents } from 'electron'
import {
  applyPageMetrics,
  clearPageMetrics,
  withCaptureMetrics,
} from '../../src/main/runtime/page-emulation'

/**
 * The owner's contract: the metrics on record are what a capture restores,
 * and a layout pass that changes them mid-capture wins over the capture's
 * starting point. Breaking either reintroduces the "page snaps to the wrong
 * scale after a hi-res capture" bug.
 */
function fakeContents() {
  const commands: Array<{ method: string; params?: Record<string, unknown> }> = []
  const wc = {
    isDestroyed: () => false,
    enableDeviceEmulation: (params: Record<string, unknown>) => {
      commands.push({ method: 'enableDeviceEmulation', params })
    },
    disableDeviceEmulation: () => {
      commands.push({ method: 'disableDeviceEmulation' })
    },
  } as unknown as WebContents
  return { wc, commands }
}

const base = { width: 1194, height: 834, deviceScaleFactor: 2, scale: 0.5 }

describe('page-emulation', () => {
  it('sends metrics once and skips an identical re-apply', () => {
    const { wc, commands } = fakeContents()
    applyPageMetrics(wc, base)
    applyPageMetrics(wc, { ...base })
    expect(commands).toHaveLength(1)
    expect(commands[0].method).toBe('enableDeviceEmulation')
    expect(commands[0].params).toMatchObject({
      viewSize: { width: 1194, height: 834 },
      deviceScaleFactor: 2,
      scale: 0.5,
    })
  })

  it('restores the metrics on record after a capture, not the capture density', async () => {
    const { wc, commands } = fakeContents()
    applyPageMetrics(wc, base)
    await withCaptureMetrics(wc, 3, async (capture) => {
      expect(capture.deviceScaleFactor).toBe(6)
      expect(capture.scale).toBe(0.5)
      expect(capture.width).toBe(1194)
    })
    const last = commands.at(-1)!
    expect(last.method).toBe('enableDeviceEmulation')
    expect(last.params).toMatchObject({ deviceScaleFactor: 2, scale: 0.5 })
  })

  it('a layout pass mid-capture wins the restore', async () => {
    const { wc, commands } = fakeContents()
    applyPageMetrics(wc, base)
    await withCaptureMetrics(wc, 3, async () => {
      applyPageMetrics(wc, { ...base, scale: 1 })
    })
    expect(commands.at(-1)!.params).toMatchObject({ deviceScaleFactor: 2, scale: 1 })
  })

  it('restores to native when the page went native mid-capture', async () => {
    const { wc, commands } = fakeContents()
    applyPageMetrics(wc, base)
    await withCaptureMetrics(wc, 3, async () => {
      clearPageMetrics(wc)
    })
    expect(commands.at(-1)!.method).toBe('disableDeviceEmulation')
  })

  it('returns null without touching a page that has no metrics on record', async () => {
    const { wc, commands } = fakeContents()
    const result = await withCaptureMetrics(wc, 3, async () => 'captured')
    expect(result).toBeNull()
    expect(commands).toHaveLength(0)
  })
})
