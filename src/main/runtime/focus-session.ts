// Focus session — the single owner of focus-presentation state (ADR 0021).
//
// Sibling to InteractionController / FocusReconciler, not an InteractionMode:
// focus spans gestures, it is not itself a gesture. This module holds the
// session state (page, mode, return camera) and is the sole writer. Every
// consumer reads `focusSession()`; every exit funnels through
// `endFocusSession(reason)` so "what ends focus" is one auditable list.

import type { FocusPresentationMode } from '../../shared/types'

export interface FocusReturnCamera {
  zoom: number
  pan: { x: number; y: number }
}

export interface FocusSession {
  pageId: string
  mode: FocusPresentationMode
  /** Camera to restore on a graceful exit. Captured when the session begins. */
  returnCamera: FocusReturnCamera
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

/**
 * End the session. The only path that clears it. Returns the ended session so
 * callers (e.g. the camera restore) can read its return camera.
 */
export function endFocusSession(_reason: FocusExitReason): FocusSession | null {
  const ended = session
  session = null
  return ended
}
