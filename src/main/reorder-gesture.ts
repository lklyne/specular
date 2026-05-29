/**
 * Reorder-gesture coordinator (ADR 0015 Phase 3).
 *
 * Drives the `reordering-child` interaction mode: a drag of a managed child's
 * center dot. The renderer's `runReorderDrag` calls start → move* → commit |
 * cancel through IPC; both the IPC handlers and the smoke test routes funnel
 * here so there's one state machine.
 *
 * Invariants (plan I2/I3, gesture-begin ordering):
 *   - `start` enters the interaction mode BEFORE any mutation, so the focus
 *     reconciler sees `reordering-child` (→ aboveView) and the renderer's
 *     window-blur cancel doesn't fire on the first tick.
 *   - `move` only updates the broadcast drop-index preview — no doc mutation.
 *   - `commit` applies the reorder as one undo step (`reorderManagedChild`).
 *   - start pairs with exactly one commit or cancel.
 */

import { tryEnter, commitActive, cancelActive } from './runtime/interaction-controller'
import { currentInteractionState, updateReorderingDropIndex } from './runtime/interaction-state'
import { markUndoBoundary } from './runtime/workspace-undo'
import type { CancelReason } from '../shared/interaction-types'
import { groupById } from './workspace-entities'
import { managedChildOrder } from './runtime/entity-order-state'
import { computeReorderDropIndex, reorderManagedChild } from './managed-layout'

let activeChildId: string | null = null
let activeGroupId: string | null = null

function clearActive(): void {
  activeChildId = null
  activeGroupId = null
}

/** Begin a reorder drag. Returns false (and enters nothing) when the child is
 *  not a member of a managed group. */
export function startReorderGesture(childId: string, groupId: string): boolean {
  const group = groupById(groupId)
  if (!group || !group.managedLayout) return false
  const dropIndex = managedChildOrder(groupId).indexOf(childId)
  if (dropIndex === -1) return false

  const token = tryEnter({ kind: 'reordering-child', groupId, childId, dropIndex })
  if ('refused' in token) return false
  activeChildId = childId
  activeGroupId = groupId
  return true
}

/** Update the live drop-index preview from the cursor's canvas-space X. */
export function moveReorderGesture(cursorCanvasX: number): void {
  if (!activeChildId || !activeGroupId) return
  const dropIndex = computeReorderDropIndex(activeGroupId, activeChildId, cursorCanvasX)
  updateReorderingDropIndex(dropIndex)
}

/** Commit the reorder at the current drop index. Returns true if the order
 *  changed. A drag that didn't move (drop index unchanged) is a clean no-op. */
export function commitReorderGesture(): boolean {
  const childId = activeChildId
  const groupId = activeGroupId
  clearActive()
  if (!childId || !groupId) {
    commitActive()
    return false
  }
  const state = currentInteractionState()
  const dropIndex = state.kind === 'reordering-child' ? state.dropIndex : -1
  let changed = false
  if (dropIndex >= 0) changed = reorderManagedChild(groupId, childId, dropIndex)
  commitActive()
  markUndoBoundary()
  return changed
}

/** Abort the reorder without mutating order or positions. */
export function cancelReorderGesture(reason: CancelReason): void {
  clearActive()
  cancelActive(reason)
}
