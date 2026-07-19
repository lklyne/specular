/**
 * startPointerSession's live-modifier channel: keydown/keyup while the pointer
 * is stationary must fire onModifiers, deduped so key auto-repeat doesn't spam,
 * and hand back the last pointer position. Unit tests run in plain Node (no
 * jsdom — see vitest.unit.config.ts), so we stub the window event registry and
 * drive the registered handlers directly.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { startPointerSession, type ModifierState } from '../../src/renderer/above-view/pointer-session'

type Handler = (ev: unknown) => void
let handlers: Map<string, Set<Handler>>

function fire(type: string, ev: unknown) {
  for (const h of handlers.get(type) ?? []) h(ev)
}

function keyEvent(mods: Partial<ModifierState>) {
  return { metaKey: false, ctrlKey: false, shiftKey: false, altKey: false, ...mods }
}

beforeEach(() => {
  handlers = new Map()
  const g = globalThis as Record<string, unknown>
  // capturePointer does `target instanceof Element`; a null target sidesteps
  // capture, but Element must exist as a constructor for instanceof not to throw.
  g.Element = class {}
  g.window = {
    addEventListener: (type: string, h: Handler) => {
      if (!handlers.has(type)) handlers.set(type, new Set())
      handlers.get(type)!.add(h)
    },
    removeEventListener: (type: string, h: Handler) => handlers.get(type)?.delete(h),
  }
})

afterEach(() => {
  const g = globalThis as Record<string, unknown>
  delete g.window
  delete g.Element
})

const down = { pointerId: 1, target: null, clientX: 10, clientY: 20, metaKey: false, ctrlKey: false, shiftKey: false, altKey: false }

describe('startPointerSession live modifiers', () => {
  it('fires once per transition and dedupes key auto-repeat', () => {
    const onModifiers = vi.fn()
    startPointerSession(down as never, { onModifiers })

    fire('keydown', keyEvent({ metaKey: true }))
    expect(onModifiers).toHaveBeenCalledTimes(1)
    expect(onModifiers.mock.calls[0][0]).toMatchObject({ metaKey: true })
    // lastPointer defaults to the pointerdown until the first move.
    expect(onModifiers.mock.calls[0][1]).toMatchObject({ clientX: 10, clientY: 20 })

    // Auto-repeat: same modifier state, no new fire.
    fire('keydown', keyEvent({ metaKey: true }))
    fire('keydown', keyEvent({ metaKey: true }))
    expect(onModifiers).toHaveBeenCalledTimes(1)

    // Release transitions back — fires again.
    fire('keyup', keyEvent({ metaKey: false }))
    expect(onModifiers).toHaveBeenCalledTimes(2)
    expect(onModifiers.mock.calls[1][0]).toMatchObject({ metaKey: false })
  })

  it('re-baselines from pointermove so a key matching the moved state does not fire', () => {
    const onModifiers = vi.fn()
    startPointerSession(down as never, { onModifiers })

    // A move already carrying meta updates the baseline and the last position.
    fire('pointermove', { pointerId: 1, clientX: 99, clientY: 88, metaKey: true, ctrlKey: false, shiftKey: false, altKey: false })
    // A keydown with the same meta state is a no-op (move already delivered it).
    fire('keydown', keyEvent({ metaKey: true }))
    expect(onModifiers).not.toHaveBeenCalled()

    // Toggling ctrl on top is a real transition; re-renders at the moved position.
    fire('keydown', keyEvent({ metaKey: true, ctrlKey: true }))
    expect(onModifiers).toHaveBeenCalledTimes(1)
    expect(onModifiers.mock.calls[0][1]).toMatchObject({ clientX: 99, clientY: 88 })
  })

  it('does not register key listeners when onModifiers is omitted', () => {
    startPointerSession(down as never, { onMove: () => {} })
    expect(handlers.has('keydown')).toBe(false)
    expect(handlers.has('keyup')).toBe(false)
  })
})
