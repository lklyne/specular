/**
 * Shared scaffold for renderer pointer gestures: pointer-id filtering,
 * drag-threshold detection, window-level `pointermove`/`pointerup`/
 * `pointercancel`/`blur` listener install + cleanup. See
 * docs/interaction-layer.md §4.6 for the handlers this consolidates and the
 * three divergences below.
 */

import { DRAG_THRESHOLD } from './gesture-utils'

export interface DragLifecycleTarget {
  addEventListener: (type: string, listener: (event: Event) => void) => void
  removeEventListener: (type: string, listener: (event: Event) => void) => void
}

export interface DragLifecycleOptions {
  /** Screen-space pixel distance before the gesture counts as dragging.
   *  Defaults to `DRAG_THRESHOLD`. Pass 0 for gestures that act on every
   *  move from the start (resize, pan, edge-drag, reorder, forward). */
  threshold?: number
  /** Released (if given) whenever the lifecycle tears down — on commit,
   *  cancel, or an explicit `cleanup()` call. */
  releasePointer?: (() => void) | null
  /** `forwardPointerDown` omits the blur listener entirely: focus
   *  reconciliation blurs aboveView mid-gesture when input is forwarded to
   *  a page, and treating that as cancel would kill forwarding. */
  skipBlurCancel?: boolean
  /** `entityPress`/`pageBodyPress` ignore blur until the drag threshold is
   *  crossed — a pre-threshold blur is a phantom from a pending focus
   *  reconcile left over from the prior gesture. */
  suppressPreThresholdBlur?: boolean
  /** Run before the window listeners are installed — for gestures that
   *  must flip main's interaction mode before the next layout pass, or the
   *  focus reconciler cancels the gesture on its first tick (resize,
   *  reorder; see runtime/CLAUDE.md gesture-begin ordering). */
  beginBeforeMove?: () => void
  /** Called once dragging is true: on the move that crosses the threshold
   *  and every move after, until the caller tears the lifecycle down
   *  (directly via `cleanup()`, or by returning from `onCommit`/`onCancel`). */
  onMove: (event: PointerEvent) => void
  /** Called on `pointerup` for the captured pointer id. `dragging` reflects
   *  whether the threshold was ever crossed. */
  onCommit: (event: PointerEvent, dragging: boolean) => void
  /** Called on `pointercancel`, and on `blur` unless `skipBlurCancel`. */
  onCancel?: (event: Event, dragging: boolean) => void
  /** Listener target — defaults to `window`. Overridable for tests. */
  target?: DragLifecycleTarget
}

export interface DragLifecycleHandle {
  /** True once the drag threshold has been crossed. */
  isDragging: () => boolean
  /** Tear down listeners without invoking `onCommit`/`onCancel`. Idempotent. */
  cleanup: () => void
}

function defaultTarget(): DragLifecycleTarget {
  return window as unknown as DragLifecycleTarget
}

export function capturePointer(event: PointerEvent): (() => void) | null {
  const target = event.target
  if (!(target instanceof Element)) return null
  try {
    target.setPointerCapture(event.pointerId)
  } catch {
    return null
  }
  return () => {
    try {
      if (target.hasPointerCapture(event.pointerId)) {
        target.releasePointerCapture(event.pointerId)
      }
    } catch {
      /* ignore */
    }
  }
}

export function withDragLifecycle(
  startEvent: PointerEvent,
  opts: DragLifecycleOptions,
): DragLifecycleHandle {
  const pointerId = startEvent.pointerId
  const startScreenX = startEvent.screenX
  const startScreenY = startEvent.screenY
  const threshold = opts.threshold ?? DRAG_THRESHOLD
  const target = opts.target ?? defaultTarget()

  let dragging = threshold <= 0
  let active = true

  opts.beginBeforeMove?.()

  const cleanup = () => {
    if (!active) return
    active = false
    opts.releasePointer?.()
    target.removeEventListener('pointermove', onMove as (event: Event) => void)
    target.removeEventListener('pointerup', onUp as (event: Event) => void)
    target.removeEventListener('pointercancel', onCancel)
    if (!opts.skipBlurCancel) target.removeEventListener('blur', onCancel)
  }

  const onMove = (event: PointerEvent) => {
    if (!active || event.pointerId !== pointerId) return
    if (!dragging) {
      const dx = event.screenX - startScreenX
      const dy = event.screenY - startScreenY
      if (Math.abs(dx) < threshold && Math.abs(dy) < threshold) return
      dragging = true
    }
    opts.onMove(event)
  }

  const onUp = (event: PointerEvent) => {
    if (!active || event.pointerId !== pointerId) return
    cleanup()
    opts.onCommit(event, dragging)
  }

  const onCancel = (event: Event) => {
    if (!active) return
    if (event.type === 'blur' && !dragging && opts.suppressPreThresholdBlur) return
    cleanup()
    opts.onCancel?.(event, dragging)
  }

  target.addEventListener('pointermove', onMove as (event: Event) => void)
  target.addEventListener('pointerup', onUp as (event: Event) => void)
  target.addEventListener('pointercancel', onCancel)
  if (!opts.skipBlurCancel) target.addEventListener('blur', onCancel)

  return {
    isDragging: () => dragging,
    cleanup,
  }
}
