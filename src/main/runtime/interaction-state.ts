import type { CanvasInteractionState, CanvasSelectableTarget, EdgeSide } from '../../shared/types'
import { interactionState, setInteractionState } from './runtime-context'
import { markDirty } from './layout-dirty'
import { requestLayout } from './viewport-control'

export function currentInteractionState(): CanvasInteractionState {
  return interactionState
}

export function clearInteractionState(): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'idle' }
  setInteractionState(next)
  markDirty('canvas')
  requestLayout()
  return next
}

export function beginDraggingEntities(entityIds: string[]): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'dragging-entities', entityIds: [...entityIds] }
  setInteractionState(next)
  markDirty('canvas')
  requestLayout()
  return next
}

export function beginMarqueeSelect(): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'marquee-select' }
  setInteractionState(next)
  markDirty('canvas')
  requestLayout()
  return next
}

export function beginCanvasPan(): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'panning-canvas' }
  setInteractionState(next)
  markDirty('canvas')
  requestLayout()
  return next
}

export function beginEntityResize(entity: CanvasSelectableTarget): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'resizing-entity', entity }
  setInteractionState(next)
  markDirty('canvas')
  requestLayout()
  return next
}

export function beginMultiSelectionResize(): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'resizing-multi-selection' }
  setInteractionState(next)
  markDirty('canvas')
  requestLayout()
  return next
}

export function beginEntityEditing(entityId: string): CanvasInteractionState {
  const next: CanvasInteractionState = { kind: 'editing-entity', entityId }
  setInteractionState(next)
  markDirty('canvas')
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
  markDirty('canvas')
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
  markDirty('canvas')
  requestLayout()
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
  markDirty('canvas')
  requestLayout()
  return next
}

export function updateReorderingDropIndex(dropIndex: number): CanvasInteractionState {
  if (interactionState.kind !== 'reordering-row') return interactionState
  if (interactionState.dropIndex === dropIndex) return interactionState
  const next: CanvasInteractionState = { ...interactionState, dropIndex }
  setInteractionState(next)
  markDirty('canvas')
  requestLayout()
  return next
}

export function interactionBlocksPageHover(state: CanvasInteractionState = interactionState): boolean {
  return (
    state.kind === 'dragging-edge' ||
    state.kind === 'resizing-entity' ||
    state.kind === 'resizing-multi-selection' ||
    state.kind === 'dragging-entities'
  )
}

export function interactionBlocksPageSelection(state: CanvasInteractionState = interactionState): boolean {
  return state.kind === 'dragging-edge'
}

export function interactionHoverTarget(state: CanvasInteractionState = interactionState): CanvasSelectableTarget | null {
  return state.kind === 'dragging-edge' ? state.target : null
}
