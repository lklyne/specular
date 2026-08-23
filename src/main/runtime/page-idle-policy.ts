/**
 * When may page renderers be CPU-throttled?
 *
 * Chromium only background-throttles renderers it considers *hidden*. A window
 * that is visible but unfocused is not hidden, so every page keeps running rAF
 * and painting at full rate while the user works in another app — which is
 * where the battery goes. Nothing in Chromium decides this for us, so the
 * policy lives here.
 *
 * Pure on purpose: the caller supplies the clock and the observed state, and
 * gets back both the verdict and the moment that verdict can next flip on its
 * own, so the timing rules are testable without Electron and the caller has
 * exactly one timer to arm.
 */

export interface IdleThrottleInput {
  now: number
  /** The app window holds OS focus. */
  windowFocused: boolean
  /** Outstanding awake holds — recording, tracing, anything needing live frames. */
  awakeHoldCount: number
  /** When the window last lost focus. Ignored while focused. */
  blurredAt: number
  /** Agent traffic keeps pages awake until this timestamp. */
  agentActiveUntil: number
  /** How long after blur throttling waits, so an alt-tab glance doesn't churn. */
  graceMs: number
}

export interface IdleThrottleVerdict {
  idle: boolean
  /**
   * When to re-evaluate. Null means no timer can change the answer — only an
   * external event (focus, a hold, agent traffic) will.
   */
  nextCheckAt: number | null
}

export function evaluateIdleThrottle(input: IdleThrottleInput): IdleThrottleVerdict {
  const { now, windowFocused, awakeHoldCount, blurredAt, agentActiveUntil, graceMs } = input

  // A held or focused app is never idle, and neither condition expires on a
  // clock — releasing the hold or blurring the window re-runs this.
  if (windowFocused || awakeHoldCount > 0) return { idle: false, nextCheckAt: null }

  const idleAt = Math.max(blurredAt + graceMs, agentActiveUntil)
  if (now >= idleAt) return { idle: true, nextCheckAt: null }
  return { idle: false, nextCheckAt: idleAt }
}
