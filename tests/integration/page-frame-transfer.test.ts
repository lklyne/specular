/**
 * Texture transfer at the page-host seam (page-host.ts): what a host does
 * when the canvas falls behind on the textures it is sent.
 *
 * Mutation-verified by:
 * - dropping `finish()` from the `sendSharedTexture` failure path — the
 *   failed-transfer case fails (a canvas swamped by sixty pages waking at
 *   once timed transfers out; six left counted and the host dropped every
 *   later frame, a page stuck on its last texture);
 * - checking pool pressure before `textureScaleInFlight` in `deliver` — the
 *   transition case fails (the frame that ends a transition was dropped with
 *   the rest, and the host never delivered again).
 */

import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { WebContentsView, sharedTexture } from 'electron'
import * as pageHosts from '../../src/main/runtime/page-host'

const POOL_ALLOWANCE = 6
const SHRINK_SETTLE_MS = 600

const hosts: pageHosts.PageHost[] = []

function createHost(id: string): pageHosts.PageHost {
  pageHosts.setPageFrameTarget(new WebContentsView().webContents)
  const host = pageHosts.createPageHost({ id, width: 800, height: 600 })
  hosts.push(host)
  return host
}

function paint(host: pageHosts.PageHost, codedWidth: number): void {
  const texture = {
    textureInfo: { widgetType: 'frame', codedSize: { width: codedWidth, height: 600 } },
    release: vi.fn(),
  }
  ;(host.webContents as unknown as NodeJS.EventEmitter).emit('paint', { texture })
}

const statsFor = (id: string) => pageHosts.pageHostStats().find((s) => s.pageId === id)!

beforeEach(() => vi.useFakeTimers())

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
  hosts.splice(0).forEach((host) => host.destroy())
  pageHosts.setPageFrameTarget(null)
})

it('does not count a transfer the canvas never acknowledged against the pool', async () => {
  const host = createHost('timed-out')
  const send = vi
    .spyOn(sharedTexture, 'sendSharedTexture')
    .mockRejectedValue(new Error('transfer shared texture timed out after 1000ms'))
  vi.spyOn(console, 'error').mockImplementation(() => {})

  for (let i = 0; i < POOL_ALLOWANCE; i++) paint(host, 800)
  await vi.advanceTimersByTimeAsync(0)
  expect(statsFor('timed-out').sendFailures).toBe(POOL_ALLOWANCE)
  expect(statsFor('timed-out').outstandingTextures).toBe(0)

  send.mockResolvedValue(undefined)
  paint(host, 800)
  expect(send).toHaveBeenCalledTimes(POOL_ALLOWANCE + 1)
  expect(statsFor('timed-out').framesDroppedForPoolPressure).toBe(0)
})

it('ends a texture transition even with its pool allowance spent', async () => {
  const host = createHost('full-pool')
  // The stub never reports references released, so each send stays counted.
  for (let i = 0; i < POOL_ALLOWANCE; i++) paint(host, 800)
  expect(statsFor('full-pool').outstandingTextures).toBe(POOL_ALLOWANCE)

  host.setDisplayScale(0.1)
  vi.advanceTimersByTime(SHRINK_SETTLE_MS)
  await vi.advanceTimersByTimeAsync(0)
  expect(statsFor('full-pool').textureTransitionInFlight).toBe(true)

  paint(host, 800 * 0.25)
  expect(statsFor('full-pool').textureTransitionInFlight).toBe(false)
})
