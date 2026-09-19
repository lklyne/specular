// Protects the gap between a navigation commit and the new document's first
// paint. `did-stop-loading` fires at the load event, which a client-rendered
// app reaches with an empty body — a hot reload that gets frozen or snapshotted
// in that gap leaves a hole where the site was, and page views are transparent,
// so the hole is the canvas showing through.
//
// Mutation-verified by settling the wait on 'did-stop-loading' instead of after
// the post-load frame, and confirming the "stays unpainted through the load
// event" case below fails.

import { EventEmitter } from 'events'
import { describe, expect, it, vi } from 'vitest'
import {
  pageAwaitingPaint,
  registerPagePresentation,
} from '../../src/main/runtime/page-presentation'
import type { Page } from '../../src/main/runtime/runtime-entities'

class FakeWebContents extends EventEmitter {
  isDestroyed = (): boolean => false
  /** Resolves when the test says the page has run two animation frames. */
  paint: () => void = () => {}
  executeJavaScript = vi.fn(
    () => new Promise<boolean>((resolve) => { this.paint = () => resolve(true) }),
  )
}

function fakePage(id: string): { page: Page; wc: FakeWebContents } {
  const wc = new FakeWebContents()
  return { page: { id, pageView: { webContents: wc } } as unknown as Page, wc }
}

/** Lets the promise chain inside the presentation wait run to completion. */
const flush = async (): Promise<void> => {
  await Promise.resolve()
  await Promise.resolve()
  await Promise.resolve()
}

describe('page presentation', () => {
  it('treats a page that has never loaded as painted', () => {
    const { page } = fakePage('page_never_loaded')
    expect(pageAwaitingPaint(page.id)).toBe(false)
  })

  it('stays unpainted through the load event, until the page presents', async () => {
    const { page, wc } = fakePage('page_reload')
    const onPresented = vi.fn()
    registerPagePresentation(page, onPresented)

    wc.emit('did-start-loading')
    expect(pageAwaitingPaint(page.id)).toBe(true)

    wc.emit('did-stop-loading')
    await flush()
    expect(pageAwaitingPaint(page.id)).toBe(true)
    expect(onPresented).not.toHaveBeenCalled()

    wc.paint()
    await flush()
    expect(pageAwaitingPaint(page.id)).toBe(false)
    expect(onPresented).toHaveBeenCalledTimes(1)
  })

  it('retires a pending wait when a new load starts', async () => {
    const { page, wc } = fakePage('page_second_reload')
    const onPresented = vi.fn()
    registerPagePresentation(page, onPresented)

    wc.emit('did-start-loading')
    wc.emit('did-stop-loading')
    const stalePaint = wc.paint

    wc.emit('did-start-loading')
    stalePaint()
    await flush()
    expect(pageAwaitingPaint(page.id)).toBe(true)
    expect(onPresented).not.toHaveBeenCalled()
  })

  it('releases a page whose renderer died — it will paint nothing', async () => {
    const { page, wc } = fakePage('page_crashed')
    registerPagePresentation(page, () => {})

    wc.emit('did-start-loading')
    wc.emit('render-process-gone')
    expect(pageAwaitingPaint(page.id)).toBe(false)
  })

  it('keeps a recreated page loading when the old one under its id is torn down', async () => {
    const old = fakePage('page_reloaded_app')
    registerPagePresentation(old.page, () => {})
    const next = fakePage('page_reloaded_app')
    const onPresented = vi.fn()
    registerPagePresentation(next.page, onPresented)

    next.wc.emit('did-start-loading')
    old.wc.emit('destroyed')
    expect(pageAwaitingPaint(next.page.id)).toBe(true)

    next.wc.emit('did-stop-loading')
    next.wc.paint()
    await flush()
    expect(pageAwaitingPaint(next.page.id)).toBe(false)
    expect(onPresented).toHaveBeenCalledTimes(1)
  })
})
