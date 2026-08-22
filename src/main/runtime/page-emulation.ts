import type { WebContents } from 'electron'

/**
 * Device metrics a page renders under: CSS viewport, pixel density, and the
 * `scale` that maps the CSS viewport into the native view (the canvas zoom).
 */
export interface PageMetrics {
  width: number
  height: number
  deviceScaleFactor: number
  scale: number
}

/**
 * Single owner of page metrics emulation. Everything that changes what the
 * renderer widget thinks its viewport is goes through here, and the record
 * kept per webContents is the only statement of what was applied. A capture
 * that needs different metrics borrows them through `withCaptureMetrics`,
 * which restores from that record.
 *
 * Backed by Electron's `enableDeviceEmulation`, never CDP's
 * `Emulation.setDeviceMetricsOverride`: the DevTools handler also resizes
 * the widget view to the override size, which renders the page at the wrong
 * scale inside a canvas-sized view.
 */
const appliedByContents = new WeakMap<WebContents, PageMetrics | 'native'>()

function metricsEqual(a: PageMetrics | 'native' | undefined, b: PageMetrics | 'native'): boolean {
  if (a === undefined) return false
  if (a === 'native' || b === 'native') return a === b
  return (
    a.width === b.width &&
    a.height === b.height &&
    a.deviceScaleFactor === b.deviceScaleFactor &&
    a.scale === b.scale
  )
}

function send(wc: WebContents, metrics: PageMetrics | 'native'): void {
  if (metrics === 'native') {
    wc.disableDeviceEmulation()
    return
  }
  wc.enableDeviceEmulation({
    screenPosition: 'desktop',
    screenSize: { width: metrics.width, height: metrics.height },
    viewSize: { width: metrics.width, height: metrics.height },
    viewPosition: { x: 0, y: 0 },
    deviceScaleFactor: metrics.deviceScaleFactor,
    scale: metrics.scale,
  })
}

function dispatch(wc: WebContents, metrics: PageMetrics | 'native'): boolean {
  try {
    send(wc, metrics)
  } catch {
    // Leave no record; the next layout pass retries.
    appliedByContents.delete(wc)
    return false
  }
  appliedByContents.set(wc, metrics)
  return true
}

/** Applies `metrics` unless the record already shows them. */
export function applyPageMetrics(wc: WebContents, metrics: PageMetrics): void {
  if (wc.isDestroyed() || metricsEqual(appliedByContents.get(wc), metrics)) return
  dispatch(wc, metrics)
}

/**
 * Releases emulation so the page renders natively into its view. Returns
 * true when this call made the switch (false when already native).
 */
export function clearPageMetrics(wc: WebContents): boolean {
  if (wc.isDestroyed() || metricsEqual(appliedByContents.get(wc), 'native')) return false
  return dispatch(wc, 'native')
}

export function pageRendersNatively(wc: WebContents): boolean {
  return appliedByContents.get(wc) === 'native'
}

/** Forgets the record so the next apply re-sends whatever the page needs. */
export function invalidatePageMetrics(wc: WebContents): void {
  appliedByContents.delete(wc)
}

export function appliedPageMetrics(wc: WebContents): PageMetrics | null {
  const applied = appliedByContents.get(wc)
  return applied && applied !== 'native' ? applied : null
}

/**
 * Runs `fn` with the page's pixel density raised by `densityFactor`, then
 * restores the metrics on record. Only the density changes: the CSS viewport
 * and the view-space size stay put, so the widget's frame never outgrows its
 * bounds. Whatever the record says at the end is what gets restored, so a
 * layout pass that changed metrics mid-capture wins.
 */
export async function withCaptureMetrics<T>(
  wc: WebContents,
  densityFactor: number,
  fn: (metrics: PageMetrics) => Promise<T>,
): Promise<T | null> {
  const base = appliedPageMetrics(wc)
  if (!base || wc.isDestroyed()) return null
  const capture: PageMetrics = {
    ...base,
    deviceScaleFactor: base.deviceScaleFactor * densityFactor,
  }
  try {
    send(wc, capture)
    return await fn(capture)
  } finally {
    if (!wc.isDestroyed()) {
      const restore = appliedByContents.get(wc) ?? base
      dispatch(wc, restore)
    }
  }
}
