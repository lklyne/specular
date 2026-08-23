/**
 * Press-vs-drag decisions for the canvas pointer router's press gestures
 * (`runEntityPress`, `runPageBodyPress`, `runGroupDrag`).
 *
 * A press arms on pointerdown and resolves on the first event that decides
 * it: a pointermove reaching DRAG_THRESHOLD on either axis promotes the
 * press to a drag (the shell ends its session and hands off to the
 * option-aware drag), a stationary release commits the press (entity →
 * request edit, page/group → select), and pointercancel discards it. Pure
 * so the promotion boundary and the phantom-blur guard — the bug class
 * `docs/interaction-layer.md` §9 calls out — are unit-testable without
 * the DOM.
 *
 * Coordinates are whatever the caller measures; the press handlers compare
 * screenX/screenY (§4.6 — per-handler coordinate choices are intentional,
 * the machine only differences them).
 */

import { DRAG_THRESHOLD } from './gesture-utils'

export interface PressGestureState {
  readonly startX: number
  readonly startY: number
  readonly dragging: boolean
}

export type PressGestureEvent =
  | { type: 'move'; x: number; y: number }
  | { type: 'up' }
  | { type: 'cancel' }

export type PressGestureOutcome =
  /** No IPC. A below-threshold move keeps the press armed; a pre-promotion
   *  cancel discards it with nothing to unwind. */
  | 'ignore'
  /** First move at or past DRAG_THRESHOLD on either axis: end the press
   *  session and hand the pointer to the drag gesture. */
  | 'promote-to-drag'
  /** Stationary release: commit the press (edit request / selection). */
  | 'commit-press'
  /** Release or cancel after promotion: end the in-flight drag. */
  | 'end-drag'

export function beginPressGesture(startX: number, startY: number): PressGestureState {
  return { startX, startY, dragging: false }
}

export function pressGestureStep(
  state: PressGestureState,
  event: PressGestureEvent,
): { state: PressGestureState; outcome: PressGestureOutcome } {
  switch (event.type) {
    case 'move': {
      if (state.dragging) return { state, outcome: 'ignore' }
      const totalDx = event.x - state.startX
      const totalDy = event.y - state.startY
      if (
        Math.abs(totalDx) < DRAG_THRESHOLD &&
        Math.abs(totalDy) < DRAG_THRESHOLD
      ) {
        return { state, outcome: 'ignore' }
      }
      return { state: { ...state, dragging: true }, outcome: 'promote-to-drag' }
    }
    case 'up':
      return { state, outcome: state.dragging ? 'end-drag' : 'commit-press' }
    case 'cancel':
      return { state, outcome: state.dragging ? 'end-drag' : 'ignore' }
  }
}

/**
 * Phantom-blur guard (§4.6). Pre-promotion blur is a phantom: focus
 * reconciliation routes focus aboveView → bgView on the layout pass that
 * runs the turn after a prior gesture ends. A second click landing inside
 * that window arms this gesture, then the pending reconcile blurs
 * aboveView before any cursor movement — tearing the armed press down
 * there would drop the edit, or kill the second drag with no recovery.
 * Wait for actual movement; pointerup / pointercancel still abort cleanly.
 */
export function pressGestureIgnoresBlur(state: PressGestureState): boolean {
  return !state.dragging
}
