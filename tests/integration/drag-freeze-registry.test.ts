/**
 * The parking registry in `page-freeze.ts`, which the zoom and drag freezes
 * both rely on for "a page lives in at most one freeze".
 *
 * `capturePageFrame` returns null against this suite's Electron stub
 * (`FakeWebContentsView.getBounds()` is zero-size; view geometry is out of
 * scope for this tier per tests/README.md), so `beginDragFreeze`'s capture
 * path is covered only for its flag-off no-op. The park and release half
 * is exercised through the registry it writes to.
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

  it('a page lives in at most one freeze, the first claim wins', () => {
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
