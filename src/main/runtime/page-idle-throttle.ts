/**
 * Idle freezing for page renderers.
 *
 * Rides CDP's `Page.setWebLifecycleState` over each page's (single, shared)
 * debugger session — the one lever that quiets a renderer Chromium still
 * considers visible without costing anything to hold. A frozen page's task
 * queues stop (timers, rAF, script), the compositor keeps its last frame, and
 * `capturePage` still returns it — which is why this is the mechanism and
 * hiding the view is not.
 *
 * `Emulation.setCPUThrottlingRate` is not an option on macOS: Chromium's
 * throttler is a thread that signals the main thread every 200µs and busy-spins
 * in the handler, so a "throttled" page costs ~4k idle wakeups/s and ~6% CPU
 * each, idle or not (ADR 0035, postmortem).
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
import { ensurePageDebugger } from './page-debugger'
import { evaluateIdleThrottle } from './page-idle-policy'

type LifecycleState = 'active' | 'frozen'

/** Grace after blur, so clicking to an editor and back doesn't churn CDP. */
const BLUR_GRACE_MS = 5_000

/** How long one piece of agent traffic keeps every page awake. */
const AGENT_ACTIVITY_TRAILING_MS = 10_000

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

function pagesAreIdle(): boolean {
  return evaluateIdleThrottle({
    now: Date.now(),
    windowFocused,
    awakeHoldCount,
    blurredAt,
    agentActiveUntil,
    graceMs: BLUR_GRACE_MS,
  }).idle
}

function targetState(page: Page): LifecycleState {
  if (!pagesAreIdle()) return 'active'
  // An agent holding a CDP bridge on this page is mid-interaction; freezing
  // it would stop the very work the app is unfocused for.
  if (automationInteractivePageCounts.has(page.id)) return 'active'
  // A frozen page never finishes loading, and the user or agent that asked
  // for it is waiting on the finished page, not the idle one.
  if (loadingPageIds.has(page.id)) return 'active'
  return 'frozen'
}

/**
 * Returns false when the state did not dispatch. Attach can fail (a DevTools
 * frontend already owns the debugger), in which case the caller leaves
 * `lastIdleLifecycleState` stale so the next evaluation retries rather than
 * believing a page is frozen when it isn't.
 */
function applyState(page: Page, state: LifecycleState): boolean {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return false

  // A detach drops the override with it — forget the state so the next
  // evaluation re-applies instead of skipping as a no-op.
  if (!ensurePageDebugger(wc, () => { page.lastIdleLifecycleState = undefined })) return false

  wc.debugger
    .sendCommand('Page.setWebLifecycleState', { state })
    .catch(() => {
      page.lastIdleLifecycleState = undefined
    })
  return true
}

function syncPageIdleThrottle(page: Page): void {
  const state = targetState(page)
  if (page.lastIdleLifecycleState === state) return
  // Never attached, nothing to undo — don't open a debugger session on every
  // page just to tell it to run.
  if (page.lastIdleLifecycleState === undefined && state === 'active') return
  if (!applyState(page, state)) return
  page.lastIdleLifecycleState = state
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

/**
 * Snapshot of the throttle's own state, for the metrics sampler. Read-only —
 * observing the throttle must never nudge it, so this arms no timer and
 * dispatches nothing.
 */
export function idleThrottleState(): {
  idle: boolean
  windowFocused: boolean
  awakeHoldCount: number
} {
  return { idle: pagesAreIdle(), windowFocused, awakeHoldCount }
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
    page.lastIdleLifecycleState = undefined
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
