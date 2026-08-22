/**
 * The drag-freeze parking registry (`page-freeze.ts`'s registerFreeze /
 * pageParkingFor / releaseFreeze / updateFreezeParking /
 * pageClaimedByOtherFreeze) and `drag-freeze.ts`'s flag-gated no-op — the
 * contracts `beginDragFreeze`/`endDragFreeze` and the zoom freeze both
 * depend on for "a page lives in at most one freeze."
 *
 * `capturePageFrame` always returns null against this suite's Electron
 * stub (`FakeWebContentsView.getBounds()` is hardcoded to zero size — real
 * view geometry is called out as intentionally out of scope for this tier
 * in tests/README.md), so `beginDragFreeze`'s async capture path can never
 * succeed here. The registered→parked→released half of the lifecycle is
 * exercised directly through the registry it writes to; `beginDragFreeze`
 * itself is only exercised for its flag-off no-op.
 *
 * Mutation-verified by: (1) changing `registerFreeze` to overwrite an
 * existing entry's `parking` unconditionally on a second register instead
 * of via the map's normal replace-by-id semantics — the "first claim wins"
 * assertion fails; (2) dropping the `id === excludingId` guard in
 * `pageClaimedByOtherFreeze` — the self-exclusion assertion fails.
 */
import { afterEach, describe, expect, it } from 'vitest'
import {
  pageClaimedByOtherFreeze,
  pageParkingFor,
  registerFreeze,
  releaseFreeze,
  updateFreezeParking,
} from '../../src/main/runtime/page-freeze'
import { beginDragFreeze, endDragFreeze } from '../../src/main/runtime/drag-freeze'

afterEach(() => {
  // freezesById is module-global; drop anything a test registered so
  // failures don't leak state into the next test.
  releaseFreeze('drag')
  releaseFreeze('zoom')
})

describe('freeze parking registry', () => {
  it('parks a claimed page hidden, then releases it back to unclaimed', () => {
    expect(pageParkingFor('page-1')).toBeNull()
    registerFreeze('drag', { target: 'above', pageIds: ['page-1'], parking: 'hidden' })
    expect(pageParkingFor('page-1')).toBe('hidden')
    expect(pageParkingFor('page-2')).toBeNull()

    releaseFreeze('drag')
    expect(pageParkingFor('page-1')).toBeNull()
  })

  it('lets a freeze change an already-claimed page\'s parking in place', () => {
    registerFreeze('drag', { target: 'above', pageIds: ['page-1'], parking: 'hidden' })
    updateFreezeParking('drag', 'warm')
    expect(pageParkingFor('page-1')).toBe('warm')
  })

  it('a page lives in at most one freeze — the first claim wins', () => {
    registerFreeze('drag', { target: 'above', pageIds: ['page-1'], parking: 'hidden' })
    registerFreeze('zoom', { target: 'bg', pageIds: ['page-1'], parking: 'warm' })
    expect(pageParkingFor('page-1')).toBe('hidden')
  })

  it('pageClaimedByOtherFreeze excludes the asking freeze itself', () => {
    registerFreeze('drag', { target: 'above', pageIds: ['page-1'], parking: 'hidden' })
    expect(pageClaimedByOtherFreeze('page-1', 'zoom')).toBe(true)
    expect(pageClaimedByOtherFreeze('page-1', 'drag')).toBe(false)
    expect(pageClaimedByOtherFreeze('page-2', 'zoom')).toBe(false)
  })
})

describe('beginDragFreeze / endDragFreeze', () => {
  it('beginDragFreeze is a no-op behind the SPECULAR_DRAG_FREEZE flag', async () => {
    await beginDragFreeze(['page-1'])
    expect(pageParkingFor('page-1')).toBeNull()
  })

  it('endDragFreeze with no active freeze is a safe no-op', () => {
    expect(() => endDragFreeze()).not.toThrow()
    expect(pageParkingFor('page-1')).toBeNull()
  })
})
