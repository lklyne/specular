/**
 * The gesture session's contract (ADR 0025 §3): begin opens the batch window,
 * per-tick `mutateWorkspace` calls inside it defer their undo boundary, and
 * finalize ends the batch then marks exactly one boundary — so a multi-tick
 * gesture is one doc sync and one undo step. Finalize is idempotent; a second
 * begin while a session is active warns and finalizes the stale session.
 *
 * Mutation-verified: swapping endBatch/markUndoBoundary in finalize fails the
 * ordering test; dropping finalize's active-session guard fails the
 * idempotence test; dropping the stale-finalize on double begin fails the
 * double-begin test.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest'

const { calls } = vi.hoisted(() => ({ calls: [] as string[] }))

vi.mock('../../src/main/runtime/workspace-observers', () => ({
  beginBatch: vi.fn(() => calls.push('beginBatch')),
  endBatch: vi.fn(() => calls.push('endBatch')),
}))
vi.mock('../../src/main/runtime/workspace-undo', () => ({
  markUndoBoundary: vi.fn(() => calls.push('markUndoBoundary')),
}))
vi.mock('../../src/main/runtime/layout-dirty', () => ({
  markDirty: vi.fn(),
}))
vi.mock('../../src/main/runtime/workspace-autosave', () => ({
  scheduleWorkspaceAutosave: vi.fn(),
}))
vi.mock('../../src/main/runtime/viewport-control', () => ({
  requestLayout: vi.fn(),
}))

import {
  isGestureSessionActive,
  mutateWorkspace,
} from '../../src/main/runtime/mutate-workspace'
import { beginGestureSession } from '../../src/main/runtime/workspace-gesture-session'

beforeEach(() => {
  calls.length = 0
})

describe('beginGestureSession', () => {
  it('brackets many mutateWorkspace calls into one boundary, marked at finalize after the batch ends', () => {
    const session = beginGestureSession()
    expect(calls).toEqual(['beginBatch'])
    expect(isGestureSessionActive()).toBe(true)

    mutateWorkspace(() => {})
    mutateWorkspace(() => {})
    mutateWorkspace(() => {})
    expect(calls.filter((c) => c === 'markUndoBoundary')).toHaveLength(0)

    session.finalize()
    expect(calls.filter((c) => c === 'markUndoBoundary')).toHaveLength(1)
    expect(calls.slice(-2)).toEqual(['endBatch', 'markUndoBoundary'])
    expect(isGestureSessionActive()).toBe(false)
  })

  it('restores per-call boundaries after finalize', () => {
    beginGestureSession().finalize()
    calls.length = 0

    mutateWorkspace(() => {})
    expect(calls).toContain('markUndoBoundary')
  })

  it('finalize is idempotent', () => {
    const session = beginGestureSession()
    session.finalize()
    calls.length = 0

    session.finalize()
    expect(calls).toEqual([])
  })

  it('a second begin warns, finalizes the stale session, and starts a fresh one', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const stale = beginGestureSession()

    const fresh = beginGestureSession()
    expect(warn).toHaveBeenCalledOnce()
    expect(calls).toEqual(['beginBatch', 'endBatch', 'markUndoBoundary', 'beginBatch'])
    expect(isGestureSessionActive()).toBe(true)

    // The superseded handle can no longer close the fresh session's window.
    stale.finalize()
    expect(isGestureSessionActive()).toBe(true)

    fresh.finalize()
    expect(isGestureSessionActive()).toBe(false)
    expect(calls.slice(-2)).toEqual(['endBatch', 'markUndoBoundary'])
    warn.mockRestore()
  })
})
