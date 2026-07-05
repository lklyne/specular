/**
 * The mutation seam's own contract (ADR 0025): every call runs the trailer in
 * order (fn → markDirty → autosave → layout → undo boundary), the boundary is
 * the default, an active gesture session suppresses only the boundary, and a
 * `changed`-guarded no-op skips the trailer entirely.
 *
 * Mutation-verified: reordering the trailer in `mutateWorkspace` fails the
 * ordering test; dropping the session probe check fails the suppression test.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const { calls } = vi.hoisted(() => ({ calls: [] as string[] }))

vi.mock('../../src/main/runtime/layout-dirty', () => ({
  markDirty: vi.fn((...surfaces: string[]) => calls.push(`markDirty(${surfaces.join(',')})`)),
}))
vi.mock('../../src/main/runtime/workspace-autosave', () => ({
  scheduleWorkspaceAutosave: vi.fn(() => calls.push('scheduleWorkspaceAutosave')),
}))
vi.mock('../../src/main/runtime/viewport-control', () => ({
  requestLayout: vi.fn(() => calls.push('requestLayout')),
}))
vi.mock('../../src/main/runtime/workspace-undo', () => ({
  markUndoBoundary: vi.fn(() => calls.push('markUndoBoundary')),
}))

import {
  isGestureSessionActive,
  mutateWorkspace,
  setGestureSessionProbe,
} from '../../src/main/runtime/mutate-workspace'

beforeEach(() => {
  calls.length = 0
})

afterEach(() => {
  setGestureSessionProbe(() => false)
})

describe('mutateWorkspace', () => {
  it('runs fn first, then the trailer in dirty → autosave → layout → boundary order', () => {
    const result = mutateWorkspace(() => {
      calls.push('fn')
      return 42
    })

    expect(result).toBe(42)
    expect(calls).toEqual([
      'fn',
      'markDirty(canvas)',
      'scheduleWorkspaceAutosave',
      'requestLayout',
      'markUndoBoundary',
    ])
  })

  it('marks an undo boundary by default — one call, one undo step', () => {
    mutateWorkspace(() => {})
    mutateWorkspace(() => {})
    expect(calls.filter((c) => c === 'markUndoBoundary')).toHaveLength(2)
  })

  it('skips the whole trailer when changed() reports a no-op', () => {
    const result = mutateWorkspace(() => null, { changed: (r) => r !== null })
    expect(result).toBeNull()
    expect(calls).toEqual([])
  })

  it('runs the trailer when changed() reports a mutation', () => {
    mutateWorkspace(() => 'entity', { changed: (r) => r !== null })
    expect(calls).toContain('markUndoBoundary')
  })

  it('suppresses only the boundary while a gesture session is active', () => {
    setGestureSessionProbe(() => true)
    expect(isGestureSessionActive()).toBe(true)

    mutateWorkspace(() => calls.push('fn'))

    expect(calls).toEqual([
      'fn',
      'markDirty(canvas)',
      'scheduleWorkspaceAutosave',
      'requestLayout',
    ])
  })

  it('resumes marking boundaries once the session probe clears', () => {
    let active = true
    setGestureSessionProbe(() => active)
    mutateWorkspace(() => {})
    expect(calls).not.toContain('markUndoBoundary')

    active = false
    calls.length = 0
    mutateWorkspace(() => {})
    expect(calls).toContain('markUndoBoundary')
  })

  it('propagates fn exceptions without running the trailer', () => {
    expect(() =>
      mutateWorkspace(() => {
        throw new Error('boom')
      }),
    ).toThrow('boom')
    expect(calls).toEqual([])
  })
})
