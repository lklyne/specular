/**
 * Answers one question: does `View.setVisible(false)` actually make Chromium
 * treat a page as hidden, so the default background throttling engages?
 *
 * Viewport culling collapses off-screen pages to 0×0 bounds, which stops them
 * compositing but leaves them attached and *visible* as far as Chromium is
 * concerned — rAF, timers, and media keep running at full rate. The probe
 * measures a culled page as-is, flips `setVisible(false)`, measures again, and
 * restores it. A drop to zero frames and ~1 timer tick per second is throttling
 * engaging; unchanged counts mean the lever is somewhere else.
 *
 * Only culled pages are probed — they are already off-screen, so toggling their
 * visibility is imperceptible. Running this costs two renderer wake-ups per
 * page, which is why it is on-demand and never part of the metrics sampler.
 *
 * Visibility is requested through `page-visibility` and applied by the layout
 * pass (invariant I1), which is also the seam a real off-screen lifecycle
 * policy would use — so what this measures is what shipping it would do.
 */

import type { WebContentsView } from 'electron'
import type {
  VisibilityProbePageResult,
  VisibilityProbeResult,
  VisibilityProbeSample,
} from '../shared/process-metrics'
import { pages } from './runtime/runtime-context'
import { requestLayout } from './runtime/layout-engine'
import { setPageVisibilityOverride } from './runtime/page-visibility'
import { pageLabel, presentationOf } from './process-metrics'

const DEFAULT_WINDOW_MS = 1500
/** Bounds the renderer wake-ups a single probe run costs. */
const MAX_PAGES = 16
/** requestLayout debounces onto a 16ms timer; wait past it before measuring. */
const LAYOUT_SETTLE_MS = 50

/** Starts a rAF loop and a 100ms interval, counting both into a page global. */
const INSTALL_SCRIPT = `(() => {
  const probe = { frames: 0, timers: 0, start: Date.now(), raf: 0, interval: 0 }
  const tick = () => { probe.frames += 1; probe.raf = requestAnimationFrame(tick) }
  probe.raf = requestAnimationFrame(tick)
  probe.interval = setInterval(() => { probe.timers += 1 }, 100)
  window.__specularVisibilityProbe = probe
  return true
})()`

/** Reads the counters and resets them for the next window. */
const READ_SCRIPT = `(() => {
  const probe = window.__specularVisibilityProbe
  if (!probe) return null
  const sample = {
    visibilityState: document.visibilityState,
    frames: probe.frames,
    timerTicks: probe.timers,
    elapsedMs: Date.now() - probe.start,
  }
  probe.frames = 0
  probe.timers = 0
  probe.start = Date.now()
  return sample
})()`

const CLEANUP_SCRIPT = `(() => {
  const probe = window.__specularVisibilityProbe
  if (!probe) return false
  cancelAnimationFrame(probe.raf)
  clearInterval(probe.interval)
  delete window.__specularVisibilityProbe
  return true
})()`

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

async function readSample(view: WebContentsView): Promise<VisibilityProbeSample | null> {
  const raw = (await view.webContents.executeJavaScript(READ_SCRIPT)) as unknown
  if (!raw || typeof raw !== 'object') return null
  const sample = raw as Partial<VisibilityProbeSample>
  if (typeof sample.frames !== 'number') return null
  return {
    visibilityState: String(sample.visibilityState ?? 'unknown'),
    frames: sample.frames,
    timerTicks: Number(sample.timerTicks ?? 0),
    elapsedMs: Number(sample.elapsedMs ?? 0),
  }
}

async function probePage(
  pageId: string,
  view: WebContentsView,
  url: string,
  windowMs: number,
): Promise<VisibilityProbePageResult> {
  const result: VisibilityProbePageResult = {
    pageId,
    label: pageLabel(pageId),
    url,
    presentation: 'culled',
    before: null,
    after: null,
  }

  if (typeof view.setVisible !== 'function') {
    result.error = 'View.setVisible is unavailable in this Electron build.'
    return result
  }

  let overridden = false
  try {
    await view.webContents.executeJavaScript(INSTALL_SCRIPT)
    await sleep(windowMs)
    result.before = await readSample(view)

    // The user may have panned this page back into view while we waited.
    // Hiding it now would be visible, so stop rather than flicker the canvas.
    if (presentationOf(view) !== 'culled') {
      result.error = 'Page returned to the viewport mid-probe; skipped.'
      return result
    }

    setPageVisibilityOverride(pageId, false)
    overridden = true
    requestLayout()
    await sleep(LAYOUT_SETTLE_MS)
    await sleep(windowMs)
    result.after = await readSample(view)
  } catch (error) {
    result.error = error instanceof Error ? error.message : String(error)
  } finally {
    if (overridden) {
      setPageVisibilityOverride(pageId, null)
      requestLayout()
    }
    try {
      if (!view.webContents.isDestroyed()) {
        await view.webContents.executeJavaScript(CLEANUP_SCRIPT)
      }
    } catch {
      // The page navigated or closed; its probe globals went with it.
    }
  }

  return result
}

export async function runVisibilityProbe(
  options: { windowMs?: number } = {},
): Promise<VisibilityProbeResult> {
  const windowMs = Math.max(250, Math.min(10_000, options.windowMs ?? DEFAULT_WINDOW_MS))
  const culled = pages.filter(
    (page) =>
      !page.pageView.webContents.isDestroyed() &&
      presentationOf(page.pageView) === 'culled',
  )

  if (culled.length === 0) {
    return {
      probedAt: Date.now(),
      windowMs,
      pages: [],
      note: 'No culled pages to probe. Pan the canvas so at least one page is fully off-screen, then run again.',
    }
  }

  const targets = culled.slice(0, MAX_PAGES)
  const results = await Promise.all(
    targets.map((page) => probePage(page.id, page.pageView, page.url, windowMs)),
  )

  return {
    probedAt: Date.now(),
    windowMs,
    pages: results,
    note:
      culled.length > targets.length
        ? `Probed ${targets.length} of ${culled.length} culled pages (capped at ${MAX_PAGES}).`
        : undefined,
  }
}
