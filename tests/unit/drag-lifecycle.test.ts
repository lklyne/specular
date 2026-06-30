import { describe, expect, it } from 'vitest'
import { withDragLifecycle, type DragLifecycleTarget } from '../../src/shared/drag-lifecycle'

/** Minimal EventTarget fake — lets these tests run under the unit suite's
 *  plain-node environment (no jsdom/window). Dispatch fires registered
 *  listeners synchronously, mirroring real DOM dispatch order. */
function fakeTarget(): DragLifecycleTarget & { dispatch: (type: string, event: unknown) => void } {
  const listeners = new Map<string, Set<(event: Event) => void>>()
  return {
    addEventListener(type, listener) {
      if (!listeners.has(type)) listeners.set(type, new Set())
      listeners.get(type)!.add(listener)
    },
    removeEventListener(type, listener) {
      listeners.get(type)?.delete(listener)
    },
    dispatch(type, event) {
      for (const listener of listeners.get(type) ?? []) listener(event as Event)
    },
  }
}

function pointerEvent(overrides: Partial<PointerEvent> = {}): PointerEvent {
  return {
    pointerId: 1,
    screenX: 0,
    screenY: 0,
    ...overrides,
  } as PointerEvent
}

describe('withDragLifecycle', () => {
  it('gates onMove behind the threshold and reports dragging on commit', () => {
    const target = fakeTarget()
    const moves: number[] = []
    let committed: boolean | null = null

    withDragLifecycle(pointerEvent(), {
      target,
      onMove: (event) => moves.push(event.screenX),
      onCommit: (_event, dragging) => {
        committed = dragging
      },
    })

    // Below threshold (default 4) — no dispatch yet.
    target.dispatch('pointermove', pointerEvent({ screenX: 2 }))
    expect(moves).toEqual([])

    // Crosses threshold — this same event is forwarded to onMove.
    target.dispatch('pointermove', pointerEvent({ screenX: 5 }))
    expect(moves).toEqual([5])

    target.dispatch('pointermove', pointerEvent({ screenX: 9 }))
    expect(moves).toEqual([5, 9])

    target.dispatch('pointerup', pointerEvent({ screenX: 9 }))
    expect(committed).toBe(true)
  })

  it('treats threshold 0 as dragging from the first move (resize/pan/edge-drag/reorder shape)', () => {
    const target = fakeTarget()
    const moves: number[] = []
    withDragLifecycle(pointerEvent(), {
      target,
      threshold: 0,
      onMove: (event) => moves.push(event.screenX),
      onCommit: () => {},
    })
    target.dispatch('pointermove', pointerEvent({ screenX: 1 }))
    expect(moves).toEqual([1])
  })

  it('ignores events for other pointer ids', () => {
    const target = fakeTarget()
    const moves: number[] = []
    let commits = 0
    withDragLifecycle(pointerEvent({ pointerId: 1 }), {
      target,
      threshold: 0,
      onMove: (event) => moves.push(event.screenX),
      onCommit: () => {
        commits += 1
      },
    })
    target.dispatch('pointermove', pointerEvent({ pointerId: 2, screenX: 100 }))
    target.dispatch('pointerup', pointerEvent({ pointerId: 2 }))
    expect(moves).toEqual([])
    expect(commits).toBe(0)
  })

  it('runs beginBeforeMove before any listener can fire', () => {
    const target = fakeTarget()
    const order: string[] = []
    withDragLifecycle(pointerEvent(), {
      target,
      threshold: 0,
      beginBeforeMove: () => order.push('begin'),
      onMove: () => order.push('move'),
      onCommit: () => {},
    })
    target.dispatch('pointermove', pointerEvent())
    expect(order).toEqual(['begin', 'move'])
  })

  it('skipBlurCancel omits the blur listener entirely (forwardPointerDown shape)', () => {
    const target = fakeTarget()
    let cancelled = false
    withDragLifecycle(pointerEvent(), {
      target,
      threshold: 0,
      skipBlurCancel: true,
      onMove: () => {},
      onCommit: () => {},
      onCancel: () => {
        cancelled = true
      },
    })
    target.dispatch('blur', { type: 'blur' })
    expect(cancelled).toBe(false)
  })

  it('suppressPreThresholdBlur swallows blur before the threshold but not after (press shape)', () => {
    const target = fakeTarget()
    let cancelCount = 0
    let lastDragging: boolean | null = null
    withDragLifecycle(pointerEvent(), {
      target,
      suppressPreThresholdBlur: true,
      onMove: () => {},
      onCommit: () => {},
      onCancel: (_event, dragging) => {
        cancelCount += 1
        lastDragging = dragging
      },
    })

    // Pre-threshold blur is swallowed — gesture stays armed.
    target.dispatch('blur', { type: 'blur' })
    expect(cancelCount).toBe(0)

    // Cross the threshold, then blur cancels for real.
    target.dispatch('pointermove', pointerEvent({ screenX: 10 }))
    target.dispatch('blur', { type: 'blur' })
    expect(cancelCount).toBe(1)
    expect(lastDragging).toBe(true)
  })

  it('cleanup() is idempotent and stops further dispatch (press-to-handoff shape)', () => {
    const target = fakeTarget()
    let releaseCount = 0
    const moves: number[] = []
    const handle = withDragLifecycle(pointerEvent(), {
      target,
      releasePointer: () => {
        releaseCount += 1
      },
      onMove: (event) => {
        moves.push(event.screenX)
        handle.cleanup()
        handle.cleanup() // a caller calling cleanup twice must not double-release
      },
      onCommit: () => {},
    })

    target.dispatch('pointermove', pointerEvent({ screenX: 10 }))
    expect(moves).toEqual([10])
    expect(releaseCount).toBe(1)

    // Listeners are gone — a later move/up is a no-op.
    target.dispatch('pointermove', pointerEvent({ screenX: 20 }))
    target.dispatch('pointerup', pointerEvent({ screenX: 20 }))
    expect(moves).toEqual([10])
  })

  it('calls releasePointer exactly once on commit', () => {
    const target = fakeTarget()
    let releaseCount = 0
    withDragLifecycle(pointerEvent(), {
      target,
      threshold: 0,
      releasePointer: () => {
        releaseCount += 1
      },
      onMove: () => {},
      onCommit: () => {},
    })
    target.dispatch('pointerup', pointerEvent())
    expect(releaseCount).toBe(1)
  })
})
