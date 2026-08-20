/**
 * Idle CPU throttling for page renderers.
 *
 * Rides CDP's `Emulation.setCPUThrottlingRate` over each page's (single,
 * shared) debugger session — the one lever that works on a renderer Chromium
 * still considers visible. It suspends the guest's main thread in short
 * quanta, so rAF-driven work (the expensive kind) collapses while the
 * compositor keeps its last frame. That matters for more than smoothness:
 * `capturePage` still returns live pixels from a throttled page, which is why
 * this is the mechanism and hiding the view is not.
 *
 * Two things keep pages awake, and both exist because agents drive this app
 * while nobody is looking at it:
 *
 *   - **Awake holds** — a ref-counted claim taken by anything that needs live
 *     frames for a stretch with no traffic to prove it (recording, tracing).
 *   - **Agent activity** — a trailing window pulsed by control-server traffic,
 *     so a burst of CLI/MCP calls doesn't thrash the throttle on and off.
 *
 * A page an agent is actively driving over CDP is exempt outright, on the same
 * `automationInteractivePageCounts` signal viewport culling uses.
 */

import { automationInteractivePageCounts, pages } from './runtime-context'
import type { Page } from './runtime-entities'
import { evaluateIdleThrottle } from './page-idle-policy'

/**
 * Slow the guest main thread by this factor while idle. High enough that a
 * WebGL or canvas render loop stops costing real power, low enough that a page
 * still makes progress — a throttled page is slowed, never frozen.
 */
const IDLE_CPU_THROTTLE_RATE = 20
const UNTHROTTLED_RATE = 1

/** Grace after blur, so clicking to an editor and back doesn't churn CDP. */
const BLUR_GRACE_MS = 5_000

/** How long one piece of agent traffic keeps every page awake. */
const AGENT_ACTIVITY_TRAILING_MS = 10_000

/** Escape hatch for diagnosing a page that misbehaves under throttling. */
const disabled = process.env.SPECULAR_DISABLE_IDLE_THROTTLE === '1'

let windowFocused = true
let blurredAt = 0
let agentActiveUntil = 0
let awakeHoldCount = 0
let recheckTimer: NodeJS.Timeout | null = null

/**
 * Pages with a load in flight, tracked here rather than read off `page.isLoading`
 * so the throttle does not depend on which listener page-factory registered first.
 */
const loadingPageIds = new Set<string>()

const detachListenerAttached = new WeakSet<Electron.WebContents>()

function pagesAreIdle(): boolean {
  if (disabled) return false
  return evaluateIdleThrottle({
    now: Date.now(),
    windowFocused,
    awakeHoldCount,
    blurredAt,
    agentActiveUntil,
    graceMs: BLUR_GRACE_MS,
  }).idle
}

function targetRate(page: Page): number {
  if (!pagesAreIdle()) return UNTHROTTLED_RATE
  // An agent holding a CDP bridge on this page is mid-interaction; throttling
  // it would slow the very work the app is unfocused for.
  if (automationInteractivePageCounts.has(page.id)) return UNTHROTTLED_RATE
  // Throttling a load makes it take twenty times as long, and the user or agent
  // that asked for it is waiting on the finished page, not the idle one.
  if (loadingPageIds.has(page.id)) return UNTHROTTLED_RATE
  return IDLE_CPU_THROTTLE_RATE
}

/**
 * Returns false when the rate did not dispatch. Attach can fail (a DevTools
 * frontend already owns the debugger), in which case the caller leaves
 * `lastCpuThrottleRate` stale so the next evaluation retries rather than
 * believing a page is throttled when it isn't.
 */
function applyRate(page: Page, rate: number): boolean {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return false

  try {
    if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')
    if (!detachListenerAttached.has(wc)) {
      detachListenerAttached.add(wc)
      wc.debugger.on('detach', () => {
        detachListenerAttached.delete(wc)
        // A detach drops the override with it — forget the rate so the next
        // evaluation re-applies instead of skipping as a no-op.
        page.lastCpuThrottleRate = undefined
      })
    }
    wc.debugger
      .sendCommand('Emulation.setCPUThrottlingRate', { rate })
      .catch(() => {
        page.lastCpuThrottleRate = undefined
      })
    return true
  } catch {
    return false
  }
}

function syncPageIdleThrottle(page: Page): void {
  const rate = targetRate(page)
  if (page.lastCpuThrottleRate === rate) return
  // Never attached, nothing to undo — don't open a debugger session on every
  // page just to tell it to run at full speed.
  if (page.lastCpuThrottleRate === undefined && rate === UNTHROTTLED_RATE) return
  if (!applyRate(page, rate)) return
  page.lastCpuThrottleRate = rate
}

function syncAllPages(): void {
  for (const page of pages) syncPageIdleThrottle(page)
}

/**
 * Apply the current verdict to every page and arm a single timer for the
 * moment it can next flip on its own.
 */
function reevaluate(): void {
  if (recheckTimer) {
    clearTimeout(recheckTimer)
    recheckTimer = null
  }
  if (disabled) return

  syncAllPages()

  const { nextCheckAt } = evaluateIdleThrottle({
    now: Date.now(),
    windowFocused,
    awakeHoldCount,
    blurredAt,
    agentActiveUntil,
    graceMs: BLUR_GRACE_MS,
  })
  if (nextCheckAt === null) return
  recheckTimer = setTimeout(reevaluate, Math.max(0, nextCheckAt - Date.now()))
  recheckTimer.unref?.()
}

export function setWindowFocused(focused: boolean): void {
  if (windowFocused === focused) return
  windowFocused = focused
  if (!focused) blurredAt = Date.now()
  reevaluate()
}

/**
 * Pulse the agent-activity window. Called for control-server traffic that can
 * reach a page — page work an agent starts while the app sits unfocused must
 * run at full speed, whether or not it opens a CDP bridge.
 */
export function noteAgentActivity(): void {
  agentActiveUntil = Date.now() + AGENT_ACTIVITY_TRAILING_MS
  reevaluate()
}

/**
 * Claim live frames for a stretch of work that produces no traffic of its own.
 * The returned release is idempotent.
 */
export function holdPagesAwake(): () => void {
  awakeHoldCount += 1
  reevaluate()
  let released = false
  return () => {
    if (released) return
    released = true
    awakeHoldCount = Math.max(0, awakeHoldCount - 1)
    reevaluate()
  }
}

/**
 * Wire a freshly created page. A navigation or renderer crash starts a new
 * renderer that never saw the override, so the recorded rate is forgotten and
 * re-applied rather than trusted.
 */
export function registerPageIdleThrottle(page: Page): void {
  const wc = page.pageView.webContents
  const reapply = (): void => {
    page.lastCpuThrottleRate = undefined
    syncPageIdleThrottle(page)
  }
  wc.on('did-start-loading', () => {
    loadingPageIds.add(page.id)
    syncPageIdleThrottle(page)
  })
  wc.on('did-stop-loading', () => {
    loadingPageIds.delete(page.id)
    syncPageIdleThrottle(page)
  })
  wc.on('did-navigate', reapply)
  wc.on('render-process-gone', () => {
    loadingPageIds.delete(page.id)
    reapply()
  })
  wc.once('destroyed', () => loadingPageIds.delete(page.id))
  syncPageIdleThrottle(page)
}
