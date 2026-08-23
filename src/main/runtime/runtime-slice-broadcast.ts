/**
 * The patch producers for the slices a mutator changes without touching the
 * scene.
 *
 * A selection, a tool switch, or a gesture beginning changes one cell and no
 * entity, yet each used to `markDirty('canvas')` — so the pass rebuilt every
 * entity, diffed the whole scene, and fanned it out to deliver an id. Naming
 * the slices makes the delivery cost the size of the cell.
 *
 * Each of these carries `focus` alongside its own slice because
 * `keyboardTargetPageId` is derived, not stored: `shouldFocusSelectedPage`
 * reads the selection, the interaction kind, and the active tool together, so
 * a change to any one of them can move the keyboard target. Sending them in
 * one batch keeps a renderer from painting a new selection against the old
 * focus target.
 */

import {
  currentFocusSlice,
  currentSelectionSlice,
  currentToolSlice,
} from './canvas-layout-data'
import { interactionState } from './runtime-context'
import { broadcastRuntimePatches } from './runtime-patch-broadcast'

export function broadcastSelectionChange(): void {
  broadcastRuntimePatches([
    { kind: 'slice', slice: 'selection', value: currentSelectionSlice() },
    { kind: 'slice', slice: 'focus', value: currentFocusSlice() },
  ])
}

export function broadcastToolChange(): void {
  broadcastRuntimePatches([
    { kind: 'slice', slice: 'tool', value: currentToolSlice() },
    { kind: 'slice', slice: 'focus', value: currentFocusSlice() },
  ])
}

export function broadcastInteractionChange(): void {
  broadcastRuntimePatches([
    { kind: 'slice', slice: 'interaction', value: interactionState },
    { kind: 'slice', slice: 'focus', value: currentFocusSlice() },
  ])
}

/** A gesture tick that refines the interaction state without changing its kind
 *  — an edge drag's target, a gap's width, a reorder's drop index. The focus
 *  predicate reads only the kind, so nothing else moves. */
export function broadcastInteractionTick(): void {
  broadcastRuntimePatches([{ kind: 'slice', slice: 'interaction', value: interactionState }])
}
