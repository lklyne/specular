/**
 * Pure arbitration for canvas-mode pointer input in aboveView.
 *
 * Exactly one owner captures window-level canvas pointerdowns at a time:
 *
 *   - 'router'             — the canvas pointer router (hit-test + routing
 *                            matrix in `canvas-pointer-actions.ts`).
 *   - 'tool-gesture'       — the placement / comment tool gesture. The
 *                            router's tool branch captures every canvas
 *                            pointerdown and classifies it as
 *                            `begin-placement` / `begin-comment-gesture`.
 *   - 'annotation-overlay' — annotation surfaces own input: the draw tool or
 *                            an in-flight drawing stroke (React handlers from
 *                            `useAnnotationDrawingGestures`), or a pending
 *                            composer outside comment mode. Window-level
 *                            capture stands down.
 *   - 'none'               — the pointerdown landed on `[data-overlay-ui]`
 *                            (I8'): overlay UI handles its own input and
 *                            every gesture layer yields. `overlayUiTarget`
 *                            is per-event, so this row is enforced at event
 *                            time by each capture layer's `isOverlayUiTarget`
 *                            check; state-level callers omit the field.
 *
 * The comment tool keeps capturing while its own composer / pending region
 * rect is open so a click can retarget the draft (the composer itself is
 * overlay UI and still wins). An in-flight drawing stroke blocks it. A
 * focused thread does not participate: its conversation lives in the right
 * panel and its canvas trace is a passive ring.
 *
 * Page focus is deliberately absent from the inputs: entering a page changes
 * what the router *dispatches* (forward-pointer-down), never who owns the
 * pointerdown.
 */

import type { ToolKind } from './tool'

export type CanvasPointerOwner =
  | 'router'
  | 'tool-gesture'
  | 'annotation-overlay'
  | 'none'

export type CanvasPointerOwnerState = {
  toolKind: ToolKind
  /** Placement broadcast in flight (`pendingPlacement`). */
  pendingPlacement: boolean
  /** Comment composer draft open (element / canvas-point anchor). */
  pendingAnnotation: boolean
  /** Region annotation held open for its composer. */
  pendingRegionRect: boolean
  /** Drawing stroke session in flight. */
  drawingSession: boolean
  /** Per-event: the pointerdown landed on `[data-overlay-ui]` (I8'). */
  overlayUiTarget?: boolean
}

/**
 * Renderer-local annotation surfaces that main cannot see — pending
 * composers, in-flight drawings, the draw tool. While any is active the
 * annotation overlay owns interaction (and aboveView syncs the flag to main
 * via `setCommentOverlayActive`).
 */
export function annotationOverlayActive(state: CanvasPointerOwnerState): boolean {
  return (
    state.pendingAnnotation ||
    state.pendingRegionRect ||
    state.drawingSession ||
    state.toolKind === 'draw'
  )
}

export function canvasPointerOwner(state: CanvasPointerOwnerState): CanvasPointerOwner {
  if (state.overlayUiTarget) return 'none'
  if (state.toolKind === 'comment') {
    return state.drawingSession ? 'annotation-overlay' : 'tool-gesture'
  }
  if (annotationOverlayActive(state)) return 'annotation-overlay'
  return state.pendingPlacement ? 'tool-gesture' : 'router'
}
