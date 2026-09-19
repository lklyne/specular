/**
 * The idle policy quiets page hosts through their frame rate and painting,
 * and hands both back in full on wake. The regression that matters: a page
 * that was idled once must animate again afterwards — the lifecycle freeze
 * this replaced left offscreen pages stuck (ADR 0035, offscreen postmortem).
 *
 * Mutation-verified by:
 * - replacing `this.isPainting && !this.isIdle` with `this.isPainting` in
 *   `applyPainting` (page-host.ts) — the idle case and the culled case fail;
 * - restoring `IDLE_FRAME_RATE` instead of `activeFrameRate` in `setIdle` —
 *   the wake case fails;
 * - dropping the `loadingPageIds` check from `shouldIdle`
 *   (page-idle-throttle.ts) — the loading case fails.
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

  it('leaves a culled page unpainted after wake — idle and culling compose', () => {
    const { host, contents } = createPageHost()
    host.setPainting(false)
    expect(contents.painting).toBe(false)

    blurPastGrace()
    setWindowFocused(true)
    expect(host.idle).toBe(false)
    expect(contents.frameRate).toBe(60)
    expect(contents.painting).toBe(false)

    host.setPainting(true)
    expect(contents.painting).toBe(true)
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
