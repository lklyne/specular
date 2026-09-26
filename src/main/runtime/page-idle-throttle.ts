/**
 * Idle throttling for page hosts.
 *
 * An offscreen page is always "visible" to Chromium, so nothing quiets it
 * when the user leaves the app. The lever is the host's own frame rate:
 * `PageHost.setIdle` drops the offscreen compositor to one frame per second,
 * which paces `requestAnimationFrame` down with it, and stops texture
 * delivery. Timers and script keep running, so an idle page still answers
 * agents and finishes loads; `capturePage` still returns its last frame.
 *
 * `Page.setWebLifecycleState('frozen')` is not the lever, even though it
 * stops more: thawing a window that was never shown leaves its compositor
 * without frames, and every document it loads afterwards starts hidden
 * (ADR 0035, offscreen postmortem). `Emulation.setCPUThrottlingRate` is not
 * either: on macOS its signal loop costs ~4k wakeups/s per page (ADR 0035,
 * CPU-throttling postmortem).
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
import { broadcastRuntimePatch } from './runtime-patch-broadcast'
import { evaluateIdleThrottle } from './page-idle-policy'

/** Grace after blur, so clicking to an editor and back doesn't churn hosts. */
const BLUR_GRACE_MS = 5_000

/** How long one piece of agent traffic keeps every page awake. */
const AGENT_ACTIVITY_TRAILING_MS = 10_000

let windowFocused = true
let blurredAt = 0
let agentActiveUntil = 0
let awakeHoldCount = 0
let recheckTimer: NodeJS.Timeout | null = null
/** Last verdict the canvas renderers were told, so a re-evaluation that
 *  changes nothing sends nothing. */
let broadcastIdle: boolean | null = null

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

function shouldIdle(page: Page): boolean {
  if (!pagesAreIdle()) return false
  // An agent holding a CDP bridge on this page is mid-interaction; quieting
  // it would slow the very work the app is unfocused for.
  if (automationInteractivePageCounts.has(page.id)) return false
  // The user or agent that asked for a load is waiting on the finished page;
  // a one-frame-per-second render of it would look broken.
  if (loadingPageIds.has(page.id)) return false
  return true
}

function syncPageIdleThrottle(page: Page): void {
  page.host.setIdle(shouldIdle(page))
}

function syncAllPages(): void {
  for (const page of pages) syncPageIdleThrottle(page)
}

/**
 * Content the renderers host themselves — inline HTML files run as iframes
 * inside canvas-bg — has no host to throttle, so the verdict travels as a
 * runtime slice and the renderer quiets it.
 */
function broadcastIdleVerdict(): void {
  const idle = pagesAreIdle()
  if (broadcastIdle === idle) return
  broadcastIdle = idle
  broadcastRuntimePatch({ kind: 'slice', slice: 'idle', value: idle })
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
  broadcastIdleVerdict()

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
 * touches no host.
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
 * Wire a freshly created page. The host's frame rate and painting state are
 * properties of its offscreen view, not of a renderer, so they survive
 * navigations and renderer crashes and need no re-apply.
 */
export function registerPageIdleThrottle(page: Page): void {
  const wc = page.host.webContents
  wc.on('did-start-loading', () => {
    loadingPageIds.add(page.id)
    syncPageIdleThrottle(page)
  })
  wc.on('did-stop-loading', () => {
    loadingPageIds.delete(page.id)
    syncPageIdleThrottle(page)
  })
  wc.on('render-process-gone', () => {
    loadingPageIds.delete(page.id)
    syncPageIdleThrottle(page)
  })
  wc.once('destroyed', () => loadingPageIds.delete(page.id))
  syncPageIdleThrottle(page)
}
