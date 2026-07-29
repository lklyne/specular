import { beforeEach, describe, expect, it, vi } from 'vitest'

// viewport-control pulls in electron (screen, app) via layout-engine and
// workspace-autosave; the controller only uses it to trigger layout passes,
// which are external to the gesture state machine under test. No-op it.
vi.mock('../../src/main/runtime/viewport-control', () => ({
  requestLayout: () => {},
}))

import {
  tryEnter,
  commit,
  cancel,
  cancelActive,
  commitActive,
  peek,
  subscribe,
  __resetForTests,
  TOKEN_EXPIRY_MS,
  type InteractionRefused,
} from '../../src/main/runtime/interaction-controller'
import { interactionState } from '../../src/main/runtime/runtime-context'
import { clearInteractionState } from '../../src/main/runtime/interaction-state'
import type { Token } from '../../src/shared/interaction-types'

function expectGranted(result: Token | InteractionRefused): Token {
  if ('refused' in result) throw new Error(`expected token, got refused: ${result.reason}`)
  return result
}

beforeEach(() => {
  __resetForTests()
  clearInteractionState()
})

describe('InteractionController', () => {
  it('starts idle', () => {
    expect(peek()).toEqual({ kind: 'idle' })
    expect(interactionState.kind).toBe('idle')
  })

  it('tryEnter transitions runtime interactionState', () => {
    expectGranted(tryEnter({ kind: 'panning' }))
    expect(interactionState.kind).toBe('panning-canvas')
    expect(peek().kind).toBe('panning')
  })

  it('refuses concurrent tryEnter', () => {
    expectGranted(tryEnter({ kind: 'panning' }))
    const second = tryEnter({ kind: 'marquee' })
    expect('refused' in second).toBe(true)
    if ('refused' in second) expect(second.reason).toMatch(/already/)
  })

  it('commit returns to idle and clears runtime state', () => {
    const tok = expectGranted(tryEnter({ kind: 'marquee' }))
    commit(tok)
    expect(interactionState.kind).toBe('idle')
    expect(peek()).toEqual({ kind: 'idle' })
    expectGranted(tryEnter({ kind: 'panning' }))
  })

  it('cancel is idempotent', () => {
    const tok = expectGranted(tryEnter({ kind: 'panning' }))
    cancel(tok, 'escape')
    cancel(tok, 'escape')
    expect(interactionState.kind).toBe('idle')
  })

  it('stale token after commit is a no-op', () => {
    const tok = expectGranted(tryEnter({ kind: 'panning' }))
    commit(tok)
    expectGranted(tryEnter({ kind: 'marquee' }))
    cancel(tok, 'escape')
    expect(interactionState.kind).toBe('marquee-select')
  })

  it('token expiry force-cancels active gesture', () => {
    vi.useFakeTimers()
    try {
      expectGranted(tryEnter({ kind: 'panning' }))
      vi.advanceTimersByTime(TOKEN_EXPIRY_MS + 1)
      expect(interactionState.kind).toBe('idle')
    } finally {
      vi.useRealTimers()
    }
  })

  it('editing-entity does not expire', () => {
    vi.useFakeTimers()
    try {
      expectGranted(tryEnter({ kind: 'editing-entity', entityId: 'n1' }))
      vi.advanceTimersByTime(TOKEN_EXPIRY_MS * 10)
      expect(interactionState.kind).toBe('editing-entity')
    } finally {
      vi.useRealTimers()
    }
  })

  it('cancelActive on idle is a no-op', () => {
    cancelActive('external')
    expect(interactionState.kind).toBe('idle')
  })

  it('cancelActive cancels whatever is active', () => {
    expectGranted(tryEnter({ kind: 'marquee' }))
    cancelActive('external')
    expect(interactionState.kind).toBe('idle')
    expectGranted(tryEnter({ kind: 'panning' }))
  })

  it('commitActive on idle is a no-op', () => {
    commitActive()
    expect(interactionState.kind).toBe('idle')
  })

  it('commitActive ends whatever is active', () => {
    expectGranted(tryEnter({ kind: 'marquee' }))
    commitActive()
    expect(interactionState.kind).toBe('idle')
  })

  it('subscribers fire after transition, not during', async () => {
    const observed: string[] = []
    subscribe((mode) => observed.push(mode.kind))
    tryEnter({ kind: 'panning' })
    expect(observed).toEqual([])
    await Promise.resolve()
    expect(observed).toEqual(['panning'])
  })

  it('subscribers see idle on commit', async () => {
    const observed: string[] = []
    subscribe((mode) => observed.push(mode.kind))
    const tok = expectGranted(tryEnter({ kind: 'marquee' }))
    await Promise.resolve()
    commit(tok)
    await Promise.resolve()
    expect(observed).toEqual(['marquee', 'idle'])
  })
})
