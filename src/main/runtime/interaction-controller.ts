/**
 * InteractionController — the single arbiter for canvas-level gestures.
 *
 * Spec §4.1. Every canvas gesture (pan, marquee, entity drag, resize,
 * edge drag, text edit) acquires a token via tryEnter() and must pair it
 * with exactly one commit() or cancel(). Tokens expire if never closed
 * within TOKEN_EXPIRY_MS, forcing a cancel('external') — this protects
 * against renderer crashes and blur storms dropping the close call.
 *
 * Invariants (spec §6 I2, I3):
 *   - At most one non-idle mode at a time.
 *   - cancel() is idempotent.
 *   - Subscribers fire AFTER the state transition, never during dispatch.
 *
 * Phase 2 scope: controller + tokens + subscribers. The underlying state
 * still lives in runtime-context.interactionState; the controller
 * delegates to the existing interaction-state.ts helpers for the actual
 * mutations so current call sites keep working. Direct mutation from
 * elsewhere is discouraged going forward — route through tryEnter.
 */

import type { InteractionMode, Token, CancelReason } from '../../shared/interaction-types'
import { interactionState } from './runtime-context'
import {
  beginAnnotationDrag,
  beginCanvasPan,
  beginDraggingEntities,
  beginEdgeDrag,
  beginEntityEditing,
  beginEntityResize,
  beginMarqueeSelect,
  beginMultiSelectionResize,
  beginReorderingRow,
  clearInteractionState,
} from './interaction-state'
import type { CanvasSelectableTarget, EdgeSide } from '../../shared/types'

export type InteractionRefused = { refused: true; reason: string }

type InternalToken = {
  id: string
  mode: InteractionMode['kind']
  createdAt: number
  timer: ReturnType<typeof setTimeout>
}

export const TOKEN_EXPIRY_MS = 30_000

type Listener = (mode: InteractionMode) => void

let current: InternalToken | null = null
let tokenCounter = 0
const listeners = new Set<Listener>()

function newTokenId(): string {
  tokenCounter = (tokenCounter + 1) >>> 0
  return `tok_${Date.now().toString(36)}_${tokenCounter.toString(36)}`
}

/** Map the spec's InteractionMode (shared type) to a snapshot of the current runtime state. */
function snapshotMode(): InteractionMode {
  const s = interactionState
  switch (s.kind) {
    case 'idle': return { kind: 'idle' }
    case 'panning-canvas': return { kind: 'panning' }
    case 'marquee-select':
      return { kind: 'marquee', origin: { x: 0, y: 0 }, current: { x: 0, y: 0 } }
    case 'dragging-entities':
      return { kind: 'dragging-entities', ids: [...s.entityIds], anchor: { x: 0, y: 0 } }
    case 'dragging-annotation':
      return { kind: 'dragging-annotation', id: s.annotationId }
    case 'resizing-entity':
      return { kind: 'resizing-entity', id: s.entity.id, edge: 'se' }
    case 'resizing-multi-selection':
      return { kind: 'resizing-multi-selection' }
    case 'editing-entity':
      return { kind: 'editing-entity', id: s.entityId }
    case 'reordering-row':
      return { kind: 'reordering-row', ids: [...s.ids], movingId: s.movingId, dropIndex: s.dropIndex, axis: s.axis }
    case 'dragging-edge':
      return {
        kind: 'dragging-edge',
        from: { entityId: s.from.id, side: s.fromSide as 'top' | 'right' | 'bottom' | 'left' },
        target: s.target
          ? { entityId: s.target.id, side: (s.targetSide ?? 'top') as 'top' | 'right' | 'bottom' | 'left' }
          : null,
      }
  }
}

function notify(): void {
  const mode = snapshotMode()
  for (const l of listeners) {
    try { l(mode) } catch { /* subscriber errors are not our problem */ }
  }
}

function startTimer(token: InternalToken): void {
  token.timer = setTimeout(() => {
    if (current === token) cancel({ id: token.id, mode: token.mode }, 'external')
  }, TOKEN_EXPIRY_MS)
}

export type TryEnterInput =
  | { kind: 'panning' }
  | { kind: 'marquee' }
  | { kind: 'dragging-entities'; entityIds: string[] }
  | { kind: 'dragging-annotation'; annotationId: string }
  | { kind: 'resizing-entity'; target: CanvasSelectableTarget }
  | { kind: 'resizing-multi-selection' }
  | { kind: 'editing-entity'; entityId: string }
  | { kind: 'dragging-edge'; from: CanvasSelectableTarget; fromSide: EdgeSide }
  | { kind: 'reordering-row'; ids: string[]; movingId: string; dropIndex: number; axis: 'x' | 'y' }

export function peek(): InteractionMode {
  return snapshotMode()
}

export function tryEnter(input: TryEnterInput): Token | InteractionRefused {
  if (current && interactionState.kind !== 'idle') {
    return { refused: true, reason: `already in mode '${current.mode}'` }
  }
  switch (input.kind) {
    case 'panning': beginCanvasPan(); break
    case 'marquee': beginMarqueeSelect(); break
    case 'dragging-entities': beginDraggingEntities(input.entityIds); break
    case 'dragging-annotation': beginAnnotationDrag(input.annotationId); break
    case 'resizing-entity': beginEntityResize(input.target); break
    case 'resizing-multi-selection': beginMultiSelectionResize(); break
    case 'editing-entity': beginEntityEditing(input.entityId); break
    case 'dragging-edge': beginEdgeDrag(input.from, input.fromSide); break
    case 'reordering-row': beginReorderingRow(input.ids, input.movingId, input.dropIndex, input.axis); break
  }
  const token: InternalToken = {
    id: newTokenId(),
    mode: input.kind,
    createdAt: Date.now(),
    timer: undefined as unknown as ReturnType<typeof setTimeout>,
  }
  startTimer(token)
  current = token
  queueMicrotask(notify)
  return { id: token.id, mode: token.mode }
}

function isActive(token: Token): boolean {
  return current !== null && current.id === token.id
}

export function commit(token: Token): void {
  if (!isActive(token)) return
  clearTimeout(current!.timer)
  current = null
  clearInteractionState()
  queueMicrotask(notify)
}

export function cancel(token: Token, _reason: CancelReason): void {
  if (!isActive(token)) return
  clearTimeout(current!.timer)
  current = null
  clearInteractionState()
  queueMicrotask(notify)
}

/**
 * Commit whatever is active without needing a token. Twin of cancelActive,
 * for IPC gesture-end handlers that don't plumb tokens through the IPC
 * schema.
 */
export function commitActive(): void {
  if (!current) {
    if (interactionState.kind !== 'idle') clearInteractionState()
    return
  }
  const token: Token = { id: current.id, mode: current.mode }
  commit(token)
}

/**
 * Cancel whatever is active without needing a token. The canonical external
 * interrupter — used by undo, tab switch, selection mutations, blur storms,
 * and the token-expiry timer. Idempotent: a no-op if idle.
 *
 * Per spec §4.1, only four blessed sites should call this: the token timer,
 * the undo observer, external selection mutations, and tab-switch teardown.
 * IPC gesture abort paths also rely on it where tokens aren't plumbed yet.
 */
export function cancelActive(reason: CancelReason): void {
  if (!current) {
    if (interactionState.kind !== 'idle') clearInteractionState()
    return
  }
  const token: Token = { id: current.id, mode: current.mode }
  cancel(token, reason)
}



export function subscribe(listener: Listener): () => void {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

/** Testing hook. Not for production callers. */
export function __resetForTests(): void {
  if (current) clearTimeout(current.timer)
  current = null
  listeners.clear()
  tokenCounter = 0
}
