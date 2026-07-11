/**
 * Timing constants for agent presence cursor animation.
 *
 * The cursor follows a move → dwell → act → hold sequence:
 *
 *   1. TRAVEL — CSS transition moves the cursor to the target position.
 *   2. DWELL  — short pause so the user registers the cursor on the target
 *               before the action fires and the page changes.
 *   3. ACT    — the browser action (click, type, etc.) executes.
 *   4. HOLD   — label stays visible so it doesn't flicker away.
 *
 * The intent system gives the server a head start: the shim fires an intent
 * before agent-browser sends the CDP command, so elapsed travel time is
 * subtracted from the pre-action delay. In practice the agent blocks for
 * max(0, STEP_DELAY - elapsed) instead of the full STEP_DELAY.
 */

/** Duration of the CSS cubic-bezier transition that animates the cursor
 *  between positions. This runs on the renderer and never blocks the agent. */
export const PRESENCE_TRAVEL_MS = 250

/** Extra pause after the cursor arrives but before the action fires.
 *  Gives the user a moment to see where the cursor landed. */
const PRESENCE_DWELL_MS = 50

/** Total pre-action delay: travel + dwell. The CDP proxy sleeps for at most
 *  this long before forwarding a click, minus any time already elapsed since
 *  the intent was received. Also used as the per-step pause during workspace
 *  scan animations. */
export const PRESENCE_STEP_DELAY_MS = PRESENCE_TRAVEL_MS + PRESENCE_DWELL_MS

/** How long to wait after the last tool call before auto-transitioning the
 *  cursor to the "Thinking…" state. Covers the gap while the agent's chain
 *  of thought runs between actions. */
export const PRESENCE_THINKING_DELAY_MS = 3_000

/** Maximum time an intent stays in the pending map before being discarded.
 *  Prevents stale intents from affecting unrelated CDP commands. */
export const PRESENCE_INTENT_TTL_MS = 2_000

/** Duration of the rAF-driven scroll ramp. Scroll is intercepted in the
 *  preload and animated with an ease curve instead of applying the full
 *  delta instantly, so the page motion reads as continuous like the cursor. */
export const PRESENCE_SCROLL_ANIMATION_MS = 300

/** Below this gap since the session's last real CDP act (mouse press,
 *  scroll), the session is treated as mid-burst rather than coming out of
 *  an LLM thinking pause. */
export const PRESENCE_BURST_WINDOW_MS = 1_000

/** Pre-act dwell budget while mid-burst. The user is already watching
 *  continuous cursor motion, so a small dwell reads as free — and it caps
 *  the resolve→dispatch coordinate-staleness race window (ADR 0029) that
 *  the dwell itself widens. */
export const PRESENCE_BURST_STEP_DELAY_MS = 120

/**
 * Regime selection for the pre-act dwell (ADR 0029). After a gap of
 * `msSinceLastAct` since the session's last real dispatch, decide how much
 * of the full dwell budget to pay before the next mutating input event
 * dispatches: the short burst budget mid-sequence, or the full budget after
 * a thinking pause, where it's perceptually free.
 *
 * Pure and event-shaped (`msSinceLastAct`, not a `PresenceCursorEntry`
 * read) so #319 Phase 5's event-timeline choreography can port it
 * unchanged (ADR 0029, "Future path: presence event timeline").
 */
export function selectDwellBudgetMs(msSinceLastAct: number | null): number {
  if (msSinceLastAct !== null && msSinceLastAct < PRESENCE_BURST_WINDOW_MS) {
    return PRESENCE_BURST_STEP_DELAY_MS
  }
  return PRESENCE_STEP_DELAY_MS
}

