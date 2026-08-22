/**
 * The renderer-side hover store (phase 1 of the diffed-runtime-store plan).
 * Hover arrives twice: as a per-move patch on the runtime-patch channel, and
 * again inside every full `layoutUpdate` snapshot. The load-bearing property is
 * that the two converge — a patched store reconciled to a snapshot ends up
 * holding the snapshot, which is what makes patches safe to drop.
 *
 * Mutation-verified by:
 * - making `reconcile` a no-op — the convergence test fails;
 * - dropping the `sameHoverTarget` guard in `set` — both "stays quiet" tests fail;
 * - ignoring `patch.target` in `applyPatch` — three tests fail;
 * - leaving the listener in the set on unsubscribe — the teardown test fails.
 */

import { describe, expect, it } from 'vitest'
import { createHoverStore } from '../../src/renderer/shared/hover-store'

function trackingStore() {
  const store = createHoverStore()
  let notifications = 0
  store.subscribe(() => {
    notifications += 1
  })
  return { store, count: () => notifications }
}

describe('hover store', () => {
  it('starts empty and takes the hover a patch carries', () => {
    const { store, count } = trackingStore()
    expect(store.read()).toBeNull()

    store.applyPatch({ kind: 'hover', target: { id: 'page-a', kind: 'page' } })

    expect(store.read()).toEqual({ id: 'page-a', kind: 'page' })
    expect(count()).toBe(1)
  })

  it('ignores a patch that repeats the target it already holds', () => {
    const { store, count } = trackingStore()
    store.applyPatch({ kind: 'hover', target: { id: 'page-a', kind: 'page' } })
    store.applyPatch({ kind: 'hover', target: { id: 'page-a', kind: 'page' } })

    expect(count()).toBe(1)
  })

  it('converges on the snapshot when a full layout update lands', () => {
    const { store } = trackingStore()
    store.applyPatch({ kind: 'hover', target: { id: 'page-a', kind: 'page' } })

    // The snapshot is main's truth at the moment it was built, so it wins over
    // whatever the patch stream left behind.
    store.reconcile({ id: 'text-b', kind: 'text' })
    expect(store.read()).toEqual({ id: 'text-b', kind: 'text' })

    store.reconcile(null)
    expect(store.read()).toBeNull()
  })

  it('stays quiet when the snapshot agrees with the patch already applied', () => {
    const { store, count } = trackingStore()
    store.applyPatch({ kind: 'hover', target: { id: 'page-a', kind: 'page' } })

    store.reconcile({ id: 'page-a', kind: 'page' })

    expect(count()).toBe(1)
    expect(store.read()).toEqual({ id: 'page-a', kind: 'page' })
  })

  it('stops notifying an unsubscribed listener', () => {
    const store = createHoverStore()
    let notifications = 0
    const unsubscribe = store.subscribe(() => {
      notifications += 1
    })

    store.applyPatch({ kind: 'hover', target: { id: 'page-a', kind: 'page' } })
    unsubscribe()
    store.applyPatch({ kind: 'hover', target: null })

    expect(notifications).toBe(1)
  })
})
