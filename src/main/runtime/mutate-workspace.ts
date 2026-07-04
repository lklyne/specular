/**
 * The single mutation seam (ADR 0025).
 *
 * Every workspace mutation ends with the same ritual: mark dirty → schedule
 * autosave → request layout → mark an undo boundary. `mutateWorkspace` owns
 * that trailer so a command can never forget it; the ordering rules that used
 * to live as prose gotchas in `src/main/runtime/CLAUDE.md` are invariants of
 * this one function.
 *
 * One call = one undo step, by default. The exception is a gesture session
 * (drag ticks, resize ticks): while a session is active the trailer skips the
 * boundary and the session's finalize marks it, so a multi-tick interaction
 * collapses to a single undo step. The session object registers itself via
 * `setGestureSessionProbe`.
 *
 * Commands built on this seam are the only exported mutators of
 * `document-commands.ts` and its sibling command modules; raw per-kind state
 * mutators (`*-entity-state.ts`) stay internal to compound commands that own
 * one trailer for the whole operation.
 */

import { markDirty } from './layout-dirty'
import { scheduleWorkspaceAutosave } from './workspace-autosave'
import { requestLayout } from './viewport-control'
import { markUndoBoundary } from './workspace-undo'

export interface MutateWorkspaceOptions<T> {
  /**
   * Skip the trailer when the mutation turned out to be a no-op (entity not
   * found, empty batch). Receives `fn`'s return value.
   */
  changed?: (result: T) => boolean
}

let gestureSessionActive: () => boolean = () => false

/**
 * Registered by the gesture session (workspace-gesture-session.ts) so
 * per-tick `mutateWorkspace` calls inside an active session defer their undo
 * boundary to the session's finalize.
 */
export function setGestureSessionProbe(probe: () => boolean): void {
  gestureSessionActive = probe
}

export function isGestureSessionActive(): boolean {
  return gestureSessionActive()
}

export function mutateWorkspace<T>(fn: () => T, opts?: MutateWorkspaceOptions<T>): T {
  const result = fn()
  if (opts?.changed && !opts.changed(result)) return result
  markDirty('canvas')
  scheduleWorkspaceAutosave()
  requestLayout()
  if (!gestureSessionActive()) markUndoBoundary()
  return result
}
