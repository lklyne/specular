/**
 * Has a page drawn a frame of the document it is currently showing?
 *
 * Between a navigation commit and the new document's first paint a page has no
 * surface: the old frame is gone and nothing has replaced it. `did-stop-loading`
 * does not close that window — it fires at the load event, which a
 * client-rendered app reaches with an empty body, whole frames before React (or
 * anything else) puts pixels on screen. Page views are transparent, so a page
 * caught there is a hole in the canvas, not a white rectangle.
 *
 * Everything that treats a page's surface as that page's content has to wait
 * past it. Freezing the page holds the hole until something thaws it; a zoom
 * snapshot taken there pictures the hole and then caches it as this document's
 * frame, so every later gesture shows it too.
 *
 * The signal is two animation frames after the load settles: Blink has laid out
 * and committed the new document, so the frame on its way is a picture of it.
 * Bounded by a timeout, because a page that never answers — hung, culled to
 * zero bounds, mid-teardown — must not read as loading forever.
 */

import type { WebContents } from 'electron'
import type { Page } from './runtime-entities'

const DOUBLE_RAF =
  'new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r(true))))'

/** Upper bound on the wait for a page's post-load frame. */
const PRESENT_TIMEOUT_MS = 2_000

const awaitingPaint = new Set<string>()

/** Whether `pageId` has a load in flight or a loaded document it has not
 *  painted yet. False for a page that has never loaded — it has no surface to
 *  protect. */
export function pageAwaitingPaint(pageId: string): boolean {
  return awaitingPaint.has(pageId)
}

async function waitForPaint(wc: WebContents): Promise<void> {
  await Promise.race([
    wc.executeJavaScript(DOUBLE_RAF).catch(() => undefined),
    new Promise((resolve) => setTimeout(resolve, PRESENT_TIMEOUT_MS)),
  ])
}

/**
 * Tracks `page`'s paint state and calls `onPresented` at the edge where it
 * gains a surface again, so callers that stood down during the gap can act.
 */
export function registerPagePresentation(page: Page, onPresented: () => void): void {
  const wc = page.pageView.webContents
  // Each load owns the wait it started; a load that begins while an earlier
  // wait is still pending retires that one rather than letting it settle a
  // document it is not a picture of.
  let generation = 0

  const settle = (): void => {
    if (!awaitingPaint.delete(page.id)) return
    onPresented()
  }

  wc.on('did-start-loading', () => {
    generation += 1
    awaitingPaint.add(page.id)
  })
  wc.on('did-stop-loading', () => {
    const gen = ++generation
    void waitForPaint(wc).then(() => {
      if (gen !== generation || wc.isDestroyed()) return
      settle()
    })
  })
  // A dead renderer paints nothing and will not answer the wait; the page is
  // as presented as it is going to get until it loads again.
  wc.on('render-process-gone', () => {
    generation += 1
    settle()
  })
  wc.once('destroyed', () => {
    generation += 1
    awaitingPaint.delete(page.id)
  })
}
