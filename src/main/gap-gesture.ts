/**
 * Gap-resize gesture coordinator (ADR 0015 Milestone 2 — draggable gap
 * handles).
 *
 * Drives the `resizing-gap` interaction mode: a drag of the strip between a
 * managed row/column group's adjacent children. The renderer's `runGapDrag`
 * calls start → move* → commit | cancel through IPC; this module is the one
 * state machine, mirroring `reorder-gesture.ts`.
 *
 * Gap derivation is linear and obvious: the cursor's movement along the
 * group's packing axis projects 1:1 onto the gap — drag right/down in a
 * row/column widens, left/up narrows; `gap = startGap + delta`, clamped ≥ 0.
 *
 * Invariants (gesture-begin ordering, §6 I5):
 *   - `start` enters the interaction mode BEFORE any mutation, so the focus
 *     reconciler sees `resizing-gap` (→ aboveView) and the renderer's
 *     window-blur cancel doesn't fire on the first tick.
 *   - `move` only updates the broadcast `gap` on the interaction state — no
 *     doc writes per tick. The renderer previews child positions from it.
 *   - `commit` writes once via `setGroupLayoutGap` (one undo step).
 *   - `cancel` restores pre-drag state with no mutation (nothing was written).
 */

import { tryEnter, commitActive, cancelActive } from './runtime/interaction-controller'
import { currentInteractionState, updateGapResizeGap } from './runtime/interaction-state'
import { markUndoBoundary } from './runtime/workspace-undo'
import type { CancelReason } from '../shared/interaction-types'
import { CLUSTER_HORIZONTAL_GUTTER } from '../shared/constants'
import { managedAxis, setGroupLayoutGap } from './managed-layout'
import { groupById } from './workspace-entities'

type ActiveGesture = {
  groupId: string
  axis: 'x' | 'y'
  startGap: number
  /** Grab point projected onto the packing axis (canvas space). */
  startCursor: number
}

let active: ActiveGesture | null = null

function clearActive(): void {
  active = null
}

/** Begin a gap drag on `groupId` from a grab point in canvas space. Returns
 *  false (and enters nothing) when the group isn't a managed row/column. */
export function startGapGesture(
  groupId: string,
  cursorCanvasX: number,
  cursorCanvasY: number,
): boolean {
  const group = groupById(groupId)
  if (!group || !group.managedLayout) return false
  if (group.layoutMode !== 'row' && group.layoutMode !== 'column') return false
  const axis = managedAxis(group.layoutMode)
  const startGap = group.layoutGap ?? CLUSTER_HORIZONTAL_GUTTER
  const token = tryEnter({ kind: 'resizing-gap', groupId, gap: startGap, axis })
  if ('refused' in token) return false
  active = {
    groupId,
    axis,
    startGap,
    startCursor: axis === 'x' ? cursorCanvasX : cursorCanvasY,
  }
  return true
}

/** Update the live gap preview from the cursor's canvas-space position. */
export function moveGapGesture(cursorCanvasX: number, cursorCanvasY: number): void {
  if (!active) return
  const cursor = active.axis === 'x' ? cursorCanvasX : cursorCanvasY
  const delta = cursor - active.startCursor
  updateGapResizeGap(Math.max(0, Math.round(active.startGap + delta)))
}

/** Commit the gap at its live preview value. One undo step (the field write
 *  and the reflow land in one transaction inside `setGroupLayoutGap`). A drag
 *  that didn't change the gap is a clean no-op. Returns whether anything
 *  changed. */
export function commitGapGesture(): boolean {
  const gesture = active
  clearActive()
  if (!gesture) {
    commitActive()
    return false
  }
  const state = currentInteractionState()
  const gap = state.kind === 'resizing-gap' ? state.gap : gesture.startGap
  const changed = gap !== gesture.startGap && setGroupLayoutGap(gesture.groupId, gap)
  commitActive()
  markUndoBoundary()
  return changed
}

/** Abort the gap drag without mutating the gap or any positions. */
export function cancelGapGesture(reason: CancelReason): void {
  clearActive()
  cancelActive(reason)
}
