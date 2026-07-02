/**
 * Shared drag-session wiring for canvas pointer gestures.
 *
 * A session owns the mechanical part every gesture repeats: pointer capture
 * on the originating element, pointerId-filtered window listeners for
 * pointermove/pointerup (any pointercancel cancels the session, no
 * pointerId filter), an optional window-blur cancel, and one teardown that
 * releases capture and removes every listener before the up/cancel
 * callback runs. Gesture semantics (thresholds, IPC dispatch,
 * commit-vs-cancel outcomes) stay at the call site.
 */

export interface PointerSessionHandlers {
  /** Pointermove for this session's pointerId. */
  onMove?: (ev: PointerEvent) => void
  /** Runs after teardown when this session's pointer is released. */
  onUp?: (ev: PointerEvent) => void
  /** Runs after teardown on pointercancel, and on window blur when
   *  `listenBlur` is set and the blur is not ignored. */
  onCancel?: (ev: Event) => void
  /** Treat window blur as a cancel. Off by default — some gestures
   *  (pointer forwarding to a page) deliberately survive aboveView losing
   *  focus mid-gesture. */
  listenBlur?: boolean
  /** Phantom-blur guard: return true to swallow a blur without cancelling.
   *  Focus reconciliation can blur aboveView on the layout pass right after
   *  a prior gesture ends; an armed gesture torn down in that window dies
   *  before the user moves the pointer, with no recovery. */
  ignoreBlur?: (ev: Event) => boolean
}

export interface PointerSession {
  /** Releases capture and removes all session listeners without invoking
   *  onUp/onCancel. Idempotent — safe to call from inside onMove when a
   *  gesture hands off to another owner mid-drag. */
  end: () => void
  /** Guarded release of this session's pointer capture (null when capture
   *  could not be taken). Idempotent; safe to call after `end`. */
  releasePointer: (() => void) | null
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

export function startPointerSession(
  event: PointerEvent,
  handlers: PointerSessionHandlers,
): PointerSession {
  const { onMove, onUp, onCancel, listenBlur = false, ignoreBlur } = handlers
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)

  let ended = false
  const end = () => {
    if (ended) return
    ended = true
    releasePointer?.()
    window.removeEventListener('pointermove', handleMove)
    window.removeEventListener('pointerup', handleUp)
    window.removeEventListener('pointercancel', handleCancel)
    if (listenBlur) window.removeEventListener('blur', handleBlur)
  }

  const handleMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    onMove?.(ev)
  }
  const handleUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    end()
    onUp?.(ev)
  }
  const handleCancel = (ev: Event) => {
    end()
    onCancel?.(ev)
  }
  const handleBlur = (ev: Event) => {
    if (ignoreBlur?.(ev)) return
    end()
    onCancel?.(ev)
  }

  window.addEventListener('pointermove', handleMove)
  window.addEventListener('pointerup', handleUp)
  window.addEventListener('pointercancel', handleCancel)
  if (listenBlur) window.addEventListener('blur', handleBlur)

  return { end, releasePointer }
}
