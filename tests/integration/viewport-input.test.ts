/**
 * PR B: viewport input applies on arrival — no 16ms input bucket.
 *
 * `applyViewportInputDelta` used to be reachable only through
 * `enqueueViewportInputDelta`, which accumulated deltas and flushed them
 * from a `setTimeout(..., 16)`. The IPC handlers now call
 * `applyViewportInputDelta` directly, and it updates the runtime camera
 * before returning — no timer in between.
 *
 * The harness's fake window reports `isDestroyed()`, so `layoutAllViews()`
 * stays dormant here (see harness.ts) and view bounds aren't observable at
 * this tier. What IS observable is the camera state `applyViewportInputDelta`
 * owns directly: `zoom` changes synchronously, before any `await`.
 *
 * Mutation-verified by reintroducing the deleted bucket (queuing the delta
 * and flushing via `setTimeout(..., 16)` instead of applying it inline) —
 * the synchronous assertion fails because `zoom` hasn't moved yet when the
 * call returns.
 */
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { applyViewportInputDelta } from '../../src/main/runtime/viewport-input'
import { zoom } from '../../src/main/runtime/runtime-context'

let harness: WorkspaceHarness

describe('viewport input applies on arrival', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  afterAll(() => harness?.dispose())

  it('updates zoom synchronously, with no 16ms bucket timer scheduled', () => {
    const setTimeoutSpy = vi.spyOn(global, 'setTimeout')
    const zoomBefore = zoom

    applyViewportInputDelta({ zoomDeltaY: -100, mouseX: 200, mouseY: 200 })

    // No await between the call and this assertion: if the delta were still
    // bucketed behind a timer, zoom would be unchanged here.
    expect(zoom).not.toBe(zoomBefore)
    expect(setTimeoutSpy).not.toHaveBeenCalledWith(expect.any(Function), 16)
  })
})
