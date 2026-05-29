/**
 * Reorder-gesture coordinator (ADR 0015 D7 — two doors, one gesture).
 *
 * Drives the `reordering-row` interaction mode: a drag of an entity's center dot.
 * The renderer's `runReorderDrag` calls start → move* → commit | cancel through
 * IPC; both the IPC handlers and the smoke test routes funnel here so there's one
 * state machine.
 *
 * The gesture, mode, dots, and cancel matrix are shared; only *eligibility* and
 * *commit* branch by door:
 *   - **Selection door** — a loose equal-gap multi-selection. The row is frozen
 *     at start (slots, gap, order, axis, box sizes); `dropIndexForCursor` runs
 *     against the frozen non-moving slots; commit repacks via `reorderSelection`
 *     (positions only, ephemeral). Freezing avoids a feedback loop between the
 *     live preview and re-detection mid-drag.
 *   - **Managed group door** — a managed-row group's child. Commit rewrites the
 *     `entityOrder` run via `reorderManagedChild` (persisted, M1 unchanged).
 *
 * `start` only carries `movingId`; this coordinator resolves which door armed it
 * (a managed-row child → managed door, else the selection door).
 *
 * Invariants (plan I2/I3, gesture-begin ordering):
 *   - `start` enters the interaction mode BEFORE any mutation, so the focus
 *     reconciler sees `reordering-row` (→ aboveView) and the renderer's
 *     window-blur cancel doesn't fire on the first tick.
 *   - `move` only updates the broadcast drop-index preview — no doc mutation.
 *   - `commit` applies the reorder as one undo step.
 *   - start pairs with exactly one commit or cancel.
 */

import { tryEnter, commitActive, cancelActive } from './runtime/interaction-controller'
import { currentInteractionState, updateReorderingDropIndex } from './runtime/interaction-state'
import { markUndoBoundary } from './runtime/workspace-undo'
import type { CancelReason } from '../shared/interaction-types'
import { managedChildOrder } from './runtime/entity-order-state'
import {
  computeReorderDropIndex,
  managedRowGroupForChild,
  reorderManagedChild,
} from './managed-layout'
import { buildSelectionRow, reorderSelection } from './runtime/document-commands'
import { dropIndexForCursor, type ReorderableRow } from '../shared/reorder-row'
import { selectedEntityIds } from './ui-state'

type ActiveGesture =
  | { door: 'managed'; groupId: string; movingId: string }
  | { door: 'selection'; row: ReorderableRow; movingId: string }

let active: ActiveGesture | null = null

function clearActive(): void {
  active = null
}

/** Begin a reorder drag for `movingId`. Resolves the door (managed-row child →
 *  managed door, else loose equal-gap selection → selection door) and freezes
 *  the row. Returns false (and enters nothing) when neither door is eligible. */
export function startReorderGesture(movingId: string): boolean {
  const groupId = managedRowGroupForChild(movingId)
  if (groupId) {
    const order = managedChildOrder(groupId)
    const dropIndex = order.indexOf(movingId)
    if (dropIndex === -1) return false
    const token = tryEnter({
      kind: 'reordering-row',
      ids: order,
      movingId,
      dropIndex,
      axis: 'x',
    })
    if ('refused' in token) return false
    active = { door: 'managed', groupId, movingId }
    return true
  }

  const row = buildSelectionRow([...selectedEntityIds()])
  if (!row) return false
  const dropIndex = row.order.indexOf(movingId)
  if (dropIndex === -1) return false
  const token = tryEnter({
    kind: 'reordering-row',
    ids: row.order,
    movingId,
    dropIndex,
    axis: row.axis,
  })
  if ('refused' in token) return false
  active = { door: 'selection', row, movingId }
  return true
}

/** Update the live drop-index preview from the cursor's canvas-space position. */
export function moveReorderGesture(cursorCanvasX: number, cursorCanvasY: number): void {
  if (!active) return
  if (active.door === 'managed') {
    updateReorderingDropIndex(
      computeReorderDropIndex(active.groupId, active.movingId, cursorCanvasX),
    )
    return
  }
  const cursorAlongAxis = active.row.axis === 'x' ? cursorCanvasX : cursorCanvasY
  updateReorderingDropIndex(dropIndexForCursor(active.row, cursorAlongAxis, active.movingId))
}

/** Commit the reorder at the current drop index. Returns true if the order
 *  changed. A drag that didn't move (drop index unchanged) is a clean no-op. */
export function commitReorderGesture(): boolean {
  const gesture = active
  clearActive()
  if (!gesture) {
    commitActive()
    return false
  }
  const state = currentInteractionState()
  const dropIndex = state.kind === 'reordering-row' ? state.dropIndex : -1
  let changed = false
  if (dropIndex >= 0) {
    changed =
      gesture.door === 'managed'
        ? reorderManagedChild(gesture.groupId, gesture.movingId, dropIndex)
        : reorderSelection(gesture.row.order, gesture.movingId, dropIndex)
  }
  commitActive()
  markUndoBoundary()
  return changed
}

/** Abort the reorder without mutating order or positions. */
export function cancelReorderGesture(reason: CancelReason): void {
  clearActive()
  cancelActive(reason)
}
