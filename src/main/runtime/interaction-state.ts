/**
 * The interaction mode token: which gesture, if any, the canvas is in.
 *
 * It is a runtime-store cell, so a change to it is an `interaction` patch —
 * no entity moves when a drag begins. The pass still runs whenever the *kind*
 * changes, because three things outside the store read it and are only
 * computed inside `layoutAllViews`: `reconcileFocus` (a gesture and an inline
 * editor own the keyboard differently), viewport culling (a page dragged past
 * the edge must not blink out), and the page-cursor bridge. A tick that
 * refines a gesture without changing its kind reaches none of those, so it is
 * the patch alone.
 */
import type { CanvasInteractionState, CanvasSelectableTarget, EdgeSide } from '../../shared/types'
import { interactionState, setInteractionState } from './runtime-context'
import {
  broadcastInteractionChange,
  broadcastInteractionTick,
} from './runtime-slice-broadcast'
import { requestLayout } from './viewport-control'

export function currentInteractionState(): CanvasInteractionState {
  return interactionState
}

export function clearInteractionState(): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'idle' }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function beginDraggingEntities(entityIds: string[]): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'dragging-entities', entityIds: [...entityIds] }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function beginMarqueeSelect(): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'marquee-select' }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function beginCanvasPan(): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'panning-canvas' }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function beginEntityResize(entity: CanvasSelectableTarget): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'resizing-entity', entity }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function beginMultiSelectionResize(): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'resizing-multi-selection' }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function beginEntityEditing(entityId: string): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'editing-entity', entityId }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function beginEdgeDrag(from: CanvasSelectableTarget, fromSide: EdgeSide): CanvasInteractionState {
  const next: CanvasInteractionState = {
    kind: 'dragging-edge',
    from,
    fromSide,
    target: null,
    targetSide: null,
  }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function updateEdgeDragTarget(
  target: CanvasSelectableTarget | null,
  targetSide: EdgeSide | null,
): CanvasInteractionState {
  if (interactionState.kind !== 'dragging-edge') return interactionState
  const next: CanvasInteractionState = {
    ...interactionState,
    target,
    targetSide,
  }
  setInteractionState(next)
  broadcastInteractionTick()
  return next
}

export function beginReorderingRow(
  ids: string[],
  movingId: string,
  dropIndex: number,
  axis: 'x' | 'y',
): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'reordering-row', ids: [...ids], movingId, dropIndex, axis }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

export function beginGapResize(
  groupId: string | null,
  entityIds: string[],
  gap: number,
  axis: 'x' | 'y',
): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'resizing-gap', groupId, entityIds: [...entityIds], gap, axis }
  setInteractionState(next)
  broadcastInteractionChange()
  requestLayout()
  return next
}

/** Live gap preview tick — updates only the broadcast interaction state (no
 *  doc writes, §6 I5). The renderer derives child positions from this value. */
export function updateGapResizeGap(gap: number): CanvasInteractionState {
  if (interactionState.kind !== 'resizing-gap') return interactionState
  if (interactionState.gap === gap) return interactionState
  const next: CanvasInteractionState = { ...interactionState, gap }
  setInteractionState(next)
  broadcastInteractionTick()
  return next
}

export function updateReorderingDropIndex(dropIndex: number): CanvasInteractionState {
  if (interactionState.kind !== 'reordering-row') return interactionState
  if (interactionState.dropIndex === dropIndex) return interactionState
  const next: CanvasInteractionState = { ...interactionState, dropIndex }
  setInteractionState(next)
  broadcastInteractionTick()
  return next
}

export function interactionBlocksPageHover(state: CanvasInteractionState = interactionState): boolean {
  return (
    state.kind === 'dragging-edge' ||
    state.kind === 'resizing-entity' ||
    state.kind === 'resizing-multi-selection' ||
    state.kind === 'resizing-gap' ||
    state.kind === 'dragging-entities'
  )
}

export function interactionBlocksPageSelection(state: CanvasInteractionState = interactionState): boolean {
  return state.kind === 'dragging-edge'
}

export function interactionHoverTarget(state: CanvasInteractionState = interactionState): CanvasSelectableTarget | null {
  return state.kind === 'dragging-edge' ? state.target : null
}
