/**
 * Gesture session (ADR 0025 §3) — one undo step per multi-tick interaction.
 *
 * A drag, resize, reorder, or distribute produces many fine-grained mutations
 * that are one user action. `beginGestureSession()` opens the batch window
 * (doc sync held) and registers with the mutation seam so per-tick
 * `mutateWorkspace` calls defer their undo boundary; `finalize()` closes the
 * window — one doc sync for the whole gesture, then one undo boundary.
 *
 * At most one session at a time (interaction-layer §6 I2: a single
 * interaction token, so gestures never overlap). A second begin while one is
 * active is a programming error: the stale session is finalized — its
 * mutations commit as one step, mirroring the interaction controller's
 * force-close of expired tokens — and a fresh session starts.
 *
 * The session drives the existing machinery; it owns no batching of its own.
 * `commitAsOneTransaction` stays separate: it merges *direct* doc writes with
 * runtime-synced mutations inside one Y.Doc transaction, which batch
 * suppression cannot do.
 */

import { setGestureSessionProbe } from './mutate-workspace'
import { beginBatch, endBatch } from './workspace-observers'
import { markUndoBoundary } from './workspace-undo'

export interface GestureSession {
  /**
   * End the batch window: one doc sync for the whole gesture, then one undo
   * boundary. Idempotent — finalizing a session that is no longer active is a
   * no-op.
   */
  finalize(): void
}

let active: GestureSession | null = null

setGestureSessionProbe(() => active !== null)

export function beginGestureSession(): GestureSession {
  if (active) {
    console.warn('[gesture-session] begin while a session is active — finalizing the stale session')
    active.finalize()
  }
  const session: GestureSession = {
    finalize() {
      if (active !== session) return
      active = null
      endBatch()
      markUndoBoundary()
    },
  }
  active = session
  beginBatch()
  return session
}
