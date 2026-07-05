// Focus session — the single owner of focus-presentation state (ADR 0021).
//
// Sibling to InteractionController / FocusReconciler, not an InteractionMode:
// focus spans gestures, it is not itself a gesture. This module holds the
// session state (page, mode) and is the sole writer. Every
// consumer reads `focusSession()`; every exit funnels through
// `endFocusSession(reason)` so "what ends focus" is one auditable list.

import type { FocusPresentationMode } from '../../shared/types'

export interface FocusSession {
  pageId: string
  mode: FocusPresentationMode
  /**
   * Are annotations (stickies/text/shapes/drawings/edges) shown over the
   * focused content. Per-session, ephemeral: starts off (clean read) and
   * latches on when a working tool activates or the user clicks the eye in the
   * focus bar. Stays on after a one-shot tool reverts — that's what lets a
   * just-placed sticky remain visible.
   */
  annotationsVisible: boolean
}

/** The closed set of reasons a focus session can end (ADR 0021). */
export type FocusExitReason =
  // Graceful, camera-restoring exit: X button, Escape, dimmed-canvas click.
  | 'dismiss'
  // The user moved the camera (setZoom/setPan). Suppressed while a working
  // tool is active — see viewport-control's camera-change handling.
  | 'camera-change'
  // focusSelection re-entered on a target that isn't a single page.
  | 're-focus'

let session: FocusSession | null = null

export function focusSession(): FocusSession | null {
  return session
}

export function isFocusSessionActive(): boolean {
  return session !== null
}

/** Start or replace the focus session. */
export function beginFocusSession(next: FocusSession): void {
  session = next
}

export function setFocusSessionMode(mode: FocusPresentationMode): void {
  if (!session) return
  session = { ...session, mode }
}

export function repointFocusSession(pageId: string): void {
  if (!session) return
  session = { ...session, pageId }
}

export function setFocusAnnotationsVisible(visible: boolean): void {
  if (!session) return
  session = { ...session, annotationsVisible: visible }
}

/**
 * End the session. The only path that clears it. Returns the ended session so
 * callers can inspect it during teardown.
 */
export function endFocusSession(_reason: FocusExitReason): FocusSession | null {
  const ended = session
  session = null
  return ended
}
