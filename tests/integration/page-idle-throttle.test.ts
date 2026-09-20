/**
 * The idle policy quiets page hosts through their frame rate and painting,
 * and hands both back in full on wake. The regression that matters: a page
 * that was idled once must animate again afterwards — the lifecycle freeze
 * this replaced left offscreen pages stuck (ADR 0035, offscreen postmortem).
 *
 * Mutation-verified by:
 * - replacing `this.isPainting && !this.isIdle` with `this.isPainting` in
 *   `applyPainting` (page-host.ts) — the idle case and the culled case fail;
 * - dropping the `setFrameRate` call from `applyPainting` — the culled
 *   frame-rate cases fail (a culled page kept compositing at 60fps for
 *   nobody; ~110% GPU-process CPU with every page off-screen);
 * - dropping the `loadingPageIds` check from `shouldIdle`
 *   (page-idle-throttle.ts) — the loading case fails;
 * - restoring a fixed 60 instead of `tierFrameRate` in `applyPainting`
 *   (page-host.ts) — the LOD wake and culled-tier cases fail.
 */

import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { findPageById } from '../../src/main/runtime/runtime-context'
import { setWindowFocused } from '../../src/main/runtime/page-idle-throttle'

let harness: WorkspaceHarness

const BLUR_GRACE_MS = 5_000

interface StubContents {
  painting: boolean
  frameRate: number
  emit(event: string): boolean
}

function createPageHost() {
  const pageId = applyCanvasPatch({
    entities: [{ kind: 'page', url: 'https://example.com/', canvasX: 0, canvasY: 0, presetIndex: 2 }],
  }).created[0]
  const page = findPageById(pageId)
  if (!page) throw new Error('page not created')
  return { host: page.host, contents: page.host.webContents as unknown as StubContents }
}

function blurPastGrace(): void {
  setWindowFocused(false)
  vi.advanceTimersByTime(BLUR_GRACE_MS + 1)
}

describe('page idle throttle', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    vi.useFakeTimers()
  })

  afterEach(() => {
    setWindowFocused(true)
    vi.useRealTimers()
  })

  afterAll(() => harness?.dispose())

  it('drops the frame rate and stops painting once the blur grace elapses, and restores both on focus', () => {
    const { host, contents } = createPageHost()
    expect(host.idle).toBe(false)
    expect(contents.frameRate).toBe(60)

    setWindowFocused(false)
    vi.advanceTimersByTime(BLUR_GRACE_MS - 1)
    expect(host.idle).toBe(false)
    expect(contents.painting).toBe(true)

    vi.advanceTimersByTime(2)
    expect(host.idle).toBe(true)
    expect(contents.frameRate).toBe(1)
    expect(contents.painting).toBe(false)

    setWindowFocused(true)
    expect(host.idle).toBe(false)
    expect(contents.frameRate).toBe(60)
    expect(contents.painting).toBe(true)
  })

  it('quiets a culled page fully — frame rate drops with painting, not just on idle', () => {
    const { host, contents } = createPageHost()
    expect(contents.frameRate).toBe(60)

    host.setPainting(false)
    expect(contents.painting).toBe(false)
    // Delivery off but compositor still at 60fps would leave an animating
    // page rendering full frames for nobody — the rate is part of culling.
    expect(contents.frameRate).toBe(1)

    host.setPainting(true)
    expect(contents.painting).toBe(true)
    expect(contents.frameRate).toBe(60)
  })

  it('leaves a culled page quiet after wake — idle and culling compose', () => {
    const { host, contents } = createPageHost()
    host.setPainting(false)
    expect(contents.painting).toBe(false)

    blurPastGrace()
    setWindowFocused(true)
    expect(host.idle).toBe(false)
    expect(contents.frameRate).toBe(1)
    expect(contents.painting).toBe(false)

    host.setPainting(true)
    expect(contents.painting).toBe(true)
    expect(contents.frameRate).toBe(60)
  })

  it('restores the LOD tier on wake, not full rate', () => {
    const { host, contents } = createPageHost()
    host.setDisplayScale(0.1)
    expect(contents.frameRate).toBe(15)
    expect(contents.painting).toBe(true)

    blurPastGrace()
    expect(contents.frameRate).toBe(1)

    setWindowFocused(true)
    // A thumbnail-sized page waking at 60fps would undo the LOD every blur.
    expect(contents.frameRate).toBe(15)

    host.setDisplayScale(1)
    expect(contents.frameRate).toBe(60)
  })

  it('applies a tier change made while culled only once painting resumes', () => {
    const { host, contents } = createPageHost()
    host.setPainting(false)
    expect(contents.frameRate).toBe(1)

    host.setDisplayScale(0.1)
    expect(contents.frameRate).toBe(1)

    host.setPainting(true)
    expect(contents.frameRate).toBe(15)
  })

  it('keeps a loading page at full rate until the load settles', () => {
    const { host, contents } = createPageHost()
    blurPastGrace()
    expect(host.idle).toBe(true)

    contents.emit('did-start-loading')
    expect(host.idle).toBe(false)
    expect(contents.frameRate).toBe(60)

    contents.emit('did-stop-loading')
    expect(host.idle).toBe(true)
    expect(contents.frameRate).toBe(1)
  })
})
