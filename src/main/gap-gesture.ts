/**
 * Gap-resize gesture coordinator (ADR 0015 Milestone 2 — draggable gap
 * handles).
 *
 * Drives the `resizing-gap` interaction mode: a drag of the strip between
 * adjacent items of a line. The renderer's `runGapDrag` calls start → move* →
 * commit | cancel through IPC; this module is the one state machine, mirroring
 * `reorder-gesture.ts` — including its two doors:
 *   - **Managed door** (`groupId` set): a managed row/column group's gap.
 *     Commit writes the group's persisted `layoutGap` via `setGroupLayoutGap`.
 *   - **Selection door** (`groupId` null): a loose equal-gap multi-selection.
 *     Commit just moves the entities via `applySelectionGap` — nothing
 *     persists but the new positions.
 *
 * Gap derivation is linear and obvious: the cursor's movement along the
 * line's packing axis projects 1:1 onto the gap — drag right/down in a
 * row/column widens, left/up narrows; `gap = startGap + delta`, clamped ≥ 0.
 *
 * Invariants (gesture-begin ordering, §6 I5):
 *   - `start` enters the interaction mode BEFORE any mutation, so the focus
 *     reconciler sees `resizing-gap` (→ aboveView) and the renderer's
 *     window-blur cancel doesn't fire on the first tick.
 *   - `move` only updates the broadcast `gap` on the interaction state — no
 *     doc writes per tick. The renderer previews positions from it.
 *   - `commit` writes once (one undo step).
 *   - `cancel` restores pre-drag state with no mutation (nothing was written).
 */

import { tryEnter, commitActive, cancelActive } from './runtime/interaction-controller'
import { updateGapResizeGap } from './runtime/interaction-state'
import { markUndoBoundary } from './runtime/workspace-undo'
import type { CancelReason } from '../shared/interaction-types'
import { managedLineAxis } from '../shared/layout-math'
import { applySelectionGap, buildSelectionRow } from './runtime/document-commands'
import { managedChildOrder } from './runtime/entity-order-state'
import { effectiveLayoutGap, setGroupLayoutGap } from './managed-layout'
import { groupById } from './workspace-entities'
import { selectedEntityIds } from './ui-state'

type ActiveGesture = {
  /** Managed group whose `layoutGap` the drag edits, or null for a selection. */
  groupId: string | null
  /** The line's items — the selection door's commit repacks exactly these. */
  entityIds: string[]
  axis: 'x' | 'y'
  startGap: number
  /** Grab point projected onto the packing axis (canvas space). */
  startCursor: number
  /** Last gap computed by `move` — the value `commit` writes. */
  gap: number
}

let active: ActiveGesture | null = null

function clearActive(): void {
  active = null
}

/** Begin a gap drag from a grab point in canvas space. `groupId` set → managed
 *  door; null → selection door (the current selection must be an equal-gap
 *  row). Returns false (and enters nothing) when the door isn't eligible. */
export function startGapGesture(
  groupId: string | null,
  cursorCanvasX: number,
  cursorCanvasY: number,
): boolean {
  let entityIds: string[]
  let axis: 'x' | 'y'
  let startGap: number

  if (groupId !== null) {
    const group = groupById(groupId)
    if (!group || !group.managedLayout) return false
    const managedAxis = managedLineAxis(group.layoutMode)
    if (managedAxis === null) return false
    entityIds = managedChildOrder(groupId)
    if (entityIds.length < 2) return false
    axis = managedAxis
    startGap = effectiveLayoutGap(group)
  } else {
    const row = buildSelectionRow([...selectedEntityIds()])
    if (!row) return false
    entityIds = row.order
    axis = row.axis
    startGap = Math.round(row.gap)
  }

  const token = tryEnter({ kind: 'resizing-gap', groupId, entityIds, gap: startGap, axis })
  if ('refused' in token) return false
  active = {
    groupId,
    entityIds,
    axis,
    startGap,
    startCursor: axis === 'x' ? cursorCanvasX : cursorCanvasY,
    // A no-move drag stays at startGap → commit no-ops cleanly.
    gap: startGap,
  }
  return true
}

/** Update the live gap preview from the cursor's canvas-space position. */
export function moveGapGesture(cursorCanvasX: number, cursorCanvasY: number): void {
  if (!active) return
  const cursor = active.axis === 'x' ? cursorCanvasX : cursorCanvasY
  const delta = cursor - active.startCursor
  active.gap = Math.max(0, Math.round(active.startGap + delta))
  updateGapResizeGap(active.gap)
}

/** Commit the gap at its live preview value. One undo step (managed door: the
 *  field write and the reflow land in one transaction inside
 *  `setGroupLayoutGap`; selection door: the position writes share one gesture
 *  session inside `applySelectionGap`). A drag that didn't change the gap is a
 *  clean no-op. Returns whether anything changed. */
export function commitGapGesture(): boolean {
  const gesture = active
  clearActive()
  if (!gesture) {
    commitActive()
    return false
  }
  const changed =
    gesture.gap !== gesture.startGap &&
    (gesture.groupId !== null
      ? setGroupLayoutGap(gesture.groupId, gesture.gap)
      : applySelectionGap(gesture.entityIds, gesture.axis, gesture.gap))
  commitActive()
  markUndoBoundary()
  return changed
}

/** Abort the gap drag without mutating the gap or any positions. */
export function cancelGapGesture(reason: CancelReason): void {
  clearActive()
  cancelActive(reason)
}
