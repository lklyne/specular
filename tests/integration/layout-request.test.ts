/**
 * requestLayout()'s single-flight guard and next-turn scheduling (PR A,
 * invariant I1: a burst of mutations in one turn collapses into one pass).
 *
 * The harness's fake window reports `isDestroyed()` so `layoutAllViews()`
 * itself stays dormant by design (see harness.ts — "the fake window reports
 * isDestroyed() so the layout engine never runs"), which means nothing the
 * pass would produce (view bounds, broadcasts) is observable at this tier.
 * What IS observable here is the scheduling contract: `requestLayout()`
 * hands its pass to the real `setImmediate`, exactly once per burst, and
 * re-arms once that pass has run. Spying on the global scheduler — not on
 * `layout-engine`'s own exports — keeps this at a real process boundary
 * rather than mocking a same-layer collaborator.
 *
 * Mutation-verified by:
 *   - reverting `requestLayout()` to `setTimeout(..., 16)` — the spy on
 *     `setImmediate` sees zero calls and the first assertion fails.
 *   - dropping the `if (layoutCache.layoutTimer) return` guard — three calls
 *     schedule three passes instead of one.
 *   - dropping `layoutCache.layoutTimer = null` inside the scheduled
 *     callback — the guard never releases, so the post-run `requestLayout()`
 *     call schedules nothing and the second assertion fails.
 */

import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { requestLayout } from '../../src/main/runtime/layout-engine'

let harness: WorkspaceHarness

describe('requestLayout scheduling', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  afterAll(() => harness?.dispose())

  it('collapses a burst of calls into one scheduled pass, and re-arms once it runs', () => {
    const immediateSpy = vi.spyOn(global, 'setImmediate')

    requestLayout()
    requestLayout()
    requestLayout()
    expect(immediateSpy).toHaveBeenCalledTimes(1)

    // Run the scheduled pass directly — the same callback Node would invoke
    // on the next turn — so this assertion isn't polluted by the test's own
    // wait going through the spied scheduler.
    const scheduledPass = immediateSpy.mock.calls[0][0] as () => void
    immediateSpy.mockClear()
    scheduledPass()

    requestLayout()
    expect(immediateSpy).toHaveBeenCalledTimes(1)
  })
})
