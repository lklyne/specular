import type { CanvasPoint } from './coords'

export type ResizeDirection = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'

export type EdgeEndpoint = { entityId: string; side: 'top' | 'right' | 'bottom' | 'left' }

export type InteractionMode =
  | { kind: 'idle' }
  | { kind: 'panning' }
  | { kind: 'marquee'; origin: CanvasPoint; current: CanvasPoint }
  | { kind: 'dragging-entities'; ids: string[]; anchor: CanvasPoint }
  | { kind: 'dragging-annotation'; id: string }
  | { kind: 'resizing-entity'; id: string; edge: ResizeDirection }
  | { kind: 'resizing-multi-selection' }
  | { kind: 'dragging-edge'; from: EdgeEndpoint; target: EdgeEndpoint | null }
  | { kind: 'editing-entity'; id: string }
  // Dragging an entity's center dot to reorder it within its row. Door-agnostic
  // (ADR 0015 D7): the row may be a loose equal-gap selection or a managed-row
  // group's children — `reorder-gesture.ts` records which door armed it and
  // branches the commit. A distinct mode (not a `dragging-entities` payload)
  // because commit semantics differ and it owns its own drop-index preview and
  // cancel path. `ids` is the frozen row order so the renderer can draw the
  // insertion line door-agnostically.
  | { kind: 'reordering-row'; ids: string[]; movingId: string; dropIndex: number; axis: 'x' | 'y' }

export type CancelReason = 'blur' | 'escape' | 'undo' | 'tab-switch' | 'external'

export type Token = { readonly id: string; readonly mode: InteractionMode['kind'] }

type InteractionRefused = { refused: true; reason: string }

type DragDelta = {
  dxCanvas: number
  dyCanvas: number
  point: CanvasPoint
  modifiers: { shift: boolean; meta: boolean; alt: boolean; ctrl: boolean }
}

type GestureContext = {
  point: CanvasPoint
  startPoint: CanvasPoint
  delta: { dx: number; dy: number }
  modifiers: { shift: boolean; meta: boolean; alt: boolean; ctrl: boolean }
  buttons: number
}

export type FocusTarget =
  | { kind: 'bgView' }
  | { kind: 'aboveView' }
  | { kind: 'page'; id: string }
  | { kind: 'toolbar' }
  | { kind: 'sidebar' }

type DropTarget =
  | { kind: 'canvas' }
  | { kind: 'entity'; id: string }
  | { kind: 'sidebar' }
  | { kind: 'none' }
