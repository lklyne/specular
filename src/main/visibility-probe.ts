/**
 * Answers one question: does a page that has stopped painting actually stand
 * down, or do rAF and timers keep running at full rate behind the stopped
 * frame stream?
 *
 * Viewport culling stops an off-screen page's offscreen window from painting,
 * which stops frame production — but the renderer is still attached and, with
 * `backgroundThrottling: false`, Chromium has no reason to consider it hidden.
 * The probe measures a culled page in that steady state, resumes painting,
 * measures again, and stops it. A `before` sample already at zero frames and
 * ~1 timer tick per second is the page standing down on its own; counts that
 * only climb once painting resumes mean the lever is somewhere else.
 *
 * Only culled pages are probed — they are already off-screen, so resuming
 * their painting for a moment is imperceptible. Running this costs two
 * renderer wake-ups per page, which is why it is on-demand and never part of
 * the metrics sampler.
 */

import type {
  VisibilityProbePageResult,
  VisibilityProbeResult,
  VisibilityProbeSample,
} from '../shared/process-metrics'
import { pages } from './runtime/runtime-context'
import type { Page } from './runtime/runtime-entities'
import { pageLabel, pagePresentationOf } from './process-metrics'
import { requestLayout } from './runtime/viewport-control'

const DEFAULT_WINDOW_MS = 1500
/** Bounds the renderer wake-ups a single probe run costs. */
const MAX_PAGES = 16

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

async function readSample(page: Page): Promise<VisibilityProbeSample | null> {
  const raw = (await page.host.webContents.executeJavaScript(READ_SCRIPT)) as unknown
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

async function probePage(page: Page, windowMs: number): Promise<VisibilityProbePageResult> {
  const result: VisibilityProbePageResult = {
    pageId: page.id,
    label: pageLabel(page.id),
    url: page.url,
    presentation: 'culled',
    before: null,
    after: null,
  }

  let resumed = false
  try {
    await page.host.webContents.executeJavaScript(INSTALL_SCRIPT)
    await sleep(windowMs)
    result.before = await readSample(page)

    // The user may have panned this page back into view while we waited. Its
    // painting state belongs to the layout pass then, so stop rather than
    // fight it.
    if (pagePresentationOf(page) !== 'culled') {
      result.error = 'Page returned to the viewport mid-probe; skipped.'
      return result
    }

    page.host.setPainting(true)
    resumed = true
    await sleep(windowMs)
    result.after = await readSample(page)
  } catch (error) {
    result.error = error instanceof Error ? error.message : String(error)
  } finally {
    if (resumed) {
      page.host.setPainting(false)
      // The probe's verdict is a measurement, not a policy: if the page came
      // back into view while it ran, this would otherwise leave it blank until
      // some unrelated pass happened to paint it again.
      requestLayout()
    }
    try {
      if (!page.host.webContents.isDestroyed()) {
        await page.host.webContents.executeJavaScript(CLEANUP_SCRIPT)
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
    (page) => !page.host.webContents.isDestroyed() && pagePresentationOf(page) === 'culled',
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
  const results = await Promise.all(targets.map((page) => probePage(page, windowMs)))

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
