/**
 * Texture-resolution LOD at the page-host seam (page-host.ts): a page's view
 * shrinks with its on-screen size once the camera rests, the document never
 * faces the small view, and everything that speaks to the page in pixels
 * follows the scale.
 *
 * Mutation-verified by:
 * - applying the wanted scale at once in `reconcileTextureScale` (dropping
 *   the settle timer) — the waits-for-the-camera case fails;
 * - dropping `|| this.lastDisplayScale >= 1` there — the pinned-page case
 *   fails (an agent's click within the settle wait would land at view px);
 * - dropping `this.reconcileTextureScale(true)` from `setPainting` — the
 *   waking-page case fails (a zoom-out woke sixty pages at full size for the
 *   length of the settle wait; the canvas could not copy their textures in
 *   time, transfers timed out, and the late copies showed as blank pages);
 * - calling `resizeView()` before `sendCommand` in `applyTextureScale` — the
 *   override-before-resize case fails (a document that saw the small view
 *   first would lay out at a fraction of its width; measured on reload);
 * - dropping `* target.viewScale` from `toPagePoint`
 *   (page-input-forwarding.ts) — the input case fails (a click on a
 *   quarter-scale page landed at 4× its coordinates);
 * - returning `contents.capturePage()` unconditionally in
 *   `captureFullResolution` — the capture case fails.
 */

import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { findPageById } from '../../src/main/runtime/runtime-context'
import { forwardPointerToPage } from '../../src/main/runtime/page-input-forwarding'

let harness: WorkspaceHarness

const GROW_SETTLE_MS = 120
const SHRINK_SETTLE_MS = 600

interface StubContents {
  debuggerCommands: Array<{ method: string; params: Record<string, unknown> }>
  contentSizes: Array<{ width: number; height: number; afterCommands: number }>
  inputEvents: Array<{ type: string; x: number; y: number }>
}

function createPageHost() {
  const pageId = applyCanvasPatch({
    entities: [{ kind: 'page', url: 'https://example.com/', canvasX: 0, canvasY: 0, presetIndex: 2 }],
  }).created[0]
  const page = findPageById(pageId)
  if (!page) throw new Error('page not created')
  return { pageId, host: page.host, contents: page.host.webContents as unknown as StubContents }
}

const overrides = (contents: StubContents) =>
  contents.debuggerCommands.filter((c) => c.method === 'Emulation.setDeviceMetricsOverride')

/** Let the override's ack (a resolved promise in the stub) run its `then`. */
const flushAck = () => vi.advanceTimersByTimeAsync(0)

describe('page texture LOD', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    vi.useFakeTimers()
  })

  afterEach(() => vi.useRealTimers())

  afterAll(() => harness?.dispose())

  it('shrinks only once the camera has rested, and a moving camera restarts the wait', async () => {
    const { host, contents } = createPageHost()
    host.setDisplayScale(0.1)
    vi.advanceTimersByTime(SHRINK_SETTLE_MS - 1)
    expect(host.textureScale).toBe(1)

    // Still zooming: the wait starts over.
    host.setDisplayScale(0.12)
    vi.advanceTimersByTime(SHRINK_SETTLE_MS - 1)
    expect(host.textureScale).toBe(1)
    expect(overrides(contents)).toHaveLength(0)

    vi.advanceTimersByTime(1)
    expect(host.textureScale).toBe(0.25)

    // The layout pass re-reporting the same scale is not camera movement.
    host.setDisplayScale(0.12)
    expect(host.textureScale).toBe(0.25)
    await flushAck()
  })

  it('lays the document out at full size before the view shrinks under it', async () => {
    const { host, contents } = createPageHost()
    const sizesBefore = contents.contentSizes.length
    host.setDisplayScale(0.1)
    vi.advanceTimersByTime(SHRINK_SETTLE_MS)

    const [override] = overrides(contents)
    expect(override.params).toMatchObject({ scale: 0.25, mobile: false, dontSetVisibleSize: true })
    const cssWidth = override.params.width as number
    // Not resized until the override is acknowledged.
    expect(contents.contentSizes).toHaveLength(sizesBefore)

    await flushAck()
    const resized = contents.contentSizes.at(-1)
    expect(resized?.width).toBe(Math.round(cssWidth * 0.25))
    expect(resized?.afterCommands).toBeGreaterThanOrEqual(
      contents.debuggerCommands.indexOf(override) + 1,
    )
  })

  it('grows sooner than it shrinks, and at once for a page pinned to full scale', async () => {
    const { host } = createPageHost()
    host.setDisplayScale(0.1)
    vi.advanceTimersByTime(SHRINK_SETTLE_MS)
    await flushAck()
    expect(host.textureScale).toBe(0.25)

    host.setDisplayScale(0.4)
    vi.advanceTimersByTime(GROW_SETTLE_MS - 1)
    expect(host.textureScale).toBe(0.25)
    vi.advanceTimersByTime(1)
    expect(host.textureScale).toBe(0.5)

    // Agent-driven and focus-session pages report scale 1: no settle wait.
    host.setDisplayScale(1)
    expect(host.textureScale).toBe(1)
    await flushAck()
  })

  it('wakes a culled page at the scale it is owed, without the settle wait', async () => {
    const { host } = createPageHost()
    host.setPainting(false)
    host.setDisplayScale(0.1)
    expect(host.textureScale).toBe(1)

    host.setPainting(true)
    expect(host.textureScale).toBe(0.25)
    await flushAck()
  })

  it('sends pointer input in view px, so a shrunken page is hit where it was clicked', async () => {
    const { pageId, host, contents } = createPageHost()
    const click = () =>
      forwardPointerToPage(pageId, {
        kind: 'down',
        windowX: 1_000_000,
        windowY: 1_000_000,
        button: 'left',
        shiftKey: false,
        ctrlKey: false,
        altKey: false,
        metaKey: false,
      } as Parameters<typeof forwardPointerToPage>[1])

    expect(click()).toBe(true)
    const full = contents.inputEvents.at(-1)!

    host.setDisplayScale(0.1)
    vi.advanceTimersByTime(SHRINK_SETTLE_MS)
    await flushAck()
    expect(click()).toBe(true)
    const shrunk = contents.inputEvents.at(-1)!
    expect(shrunk.x).toBe(Math.round(full.x * 0.25))
    expect(shrunk.y).toBe(Math.round(full.y * 0.25))
  })

  it('captures at full resolution from a shrunken page, then lets it shrink again', async () => {
    const { host, contents } = createPageHost()
    host.setDisplayScale(0.1)
    vi.advanceTimersByTime(SHRINK_SETTLE_MS)
    await flushAck()
    expect(host.textureScale).toBe(0.25)

    const capture = host.captureFullResolution()
    // Full size is owed to the capture at once, not on camera settle.
    expect(host.textureScale).toBe(1)
    // The first capture still comes back at the old width; the poll retries.
    await vi.advanceTimersByTimeAsync(100)
    await capture

    await vi.advanceTimersByTimeAsync(SHRINK_SETTLE_MS)
    expect(host.textureScale).toBe(0.25)
    expect(overrides(contents).map((c) => c.params.scale)).toEqual([0.25, 1, 0.25])
  })
})
