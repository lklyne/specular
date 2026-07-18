/**
 * Element-attachment reflow tracker (ADR 0030) — MutationObserver lifecycle.
 *
 * The valuable, testable seam is the install/uninstall rule: the observer is
 * installed only while there are subscriptions and disconnected the moment the
 * set goes empty, so an idle page pays nothing (plan step 3's explicit
 * requirement: zero subscriptions → observer not installed).
 *
 * Unit tests run in plain Node (no jsdom — see vitest.unit.config.ts), so this
 * hand-rolls the sliver of the browser API the tracker touches: a fake
 * MutationObserver recording observe/disconnect, a minimal `document`/`window`,
 * and a mocked `electron` (the module imports `ipcRenderer`). The
 * position-diff/flush path is covered by the integration round-trip; this file
 * only exercises the observer lifecycle.
 *
 * Mutation-verified by dropping the `disconnectMutationObserver()` call from
 * the empty-set branch of `setElementAttachmentSubscriptions` — the
 * "disconnects when the set goes empty" case then fails.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('electron', () => ({ ipcRenderer: { send: () => {} } }))

import {
  isElementAttachmentObserverInstalled,
  refreshElementAttachmentObserver,
  setElementAttachmentSubscriptions,
} from '../../src/preload/annotation-bbox-tracker'

interface ObserveCall {
  target: unknown
  options: MutationObserverInit
}

let observeCalls: ObserveCall[]
let disconnectCount: number

class FakeMutationObserver {
  constructor(_callback: () => void) {}
  observe(target: unknown, options: MutationObserverInit): void {
    observeCalls.push({ target, options })
  }
  disconnect(): void {
    disconnectCount++
  }
}

beforeEach(() => {
  observeCalls = []
  disconnectCount = 0
  ;(globalThis as any).MutationObserver = FakeMutationObserver
  ;(globalThis as any).document = { body: {}, querySelector: () => null }
  ;(globalThis as any).window = {
    requestAnimationFrame: () => 0,
    cancelAnimationFrame: () => {},
    setTimeout: () => 0,
    clearTimeout: () => {},
    scrollX: 0,
    scrollY: 0,
  }
})

afterEach(() => {
  // Reset the module's global observer/subscription state between tests.
  setElementAttachmentSubscriptions([])
  delete (globalThis as any).MutationObserver
  delete (globalThis as any).document
  delete (globalThis as any).window
})

describe('element-position-tracker observer lifecycle', () => {
  it('installs no observer with zero subscriptions', () => {
    expect(isElementAttachmentObserverInstalled()).toBe(false)
    setElementAttachmentSubscriptions([])
    expect(isElementAttachmentObserverInstalled()).toBe(false)
    expect(observeCalls).toHaveLength(0)
  })

  it('installs the observer on document.body while subscriptions exist', () => {
    setElementAttachmentSubscriptions(['#hero'])
    expect(isElementAttachmentObserverInstalled()).toBe(true)
    expect(observeCalls).toHaveLength(1)
    expect(observeCalls[0].target).toBe((globalThis as any).document.body)
    expect(observeCalls[0].options).toEqual({
      childList: true,
      subtree: true,
      attributes: true,
    })
  })

  it('disconnects when the subscription set goes empty', () => {
    setElementAttachmentSubscriptions(['#hero'])
    expect(isElementAttachmentObserverInstalled()).toBe(true)

    setElementAttachmentSubscriptions([])
    expect(isElementAttachmentObserverInstalled()).toBe(false)
    expect(disconnectCount).toBe(1)
  })

  it('does not reinstall on churn while subscriptions stay non-empty', () => {
    setElementAttachmentSubscriptions(['#hero'])
    setElementAttachmentSubscriptions(['#hero', '#footer'])
    // One observe total — the single observer keeps watching across set changes.
    expect(observeCalls).toHaveLength(1)
    expect(isElementAttachmentObserverInstalled()).toBe(true)
  })

  it('guards against a missing body on early load', () => {
    ;(globalThis as any).document = { body: null, querySelector: () => null }
    expect(() => setElementAttachmentSubscriptions(['#hero'])).not.toThrow()
    expect(isElementAttachmentObserverInstalled()).toBe(false)
    expect(observeCalls).toHaveLength(0)
  })

  it('reinstalls on a document (re)load once body exists', () => {
    ;(globalThis as any).document = { body: null, querySelector: () => null }
    setElementAttachmentSubscriptions(['#hero'])
    expect(isElementAttachmentObserverInstalled()).toBe(false)

    // Body appears (DOMContentLoaded); onDomReady calls the refresh.
    ;(globalThis as any).document = { body: {}, querySelector: () => null }
    refreshElementAttachmentObserver()
    expect(isElementAttachmentObserverInstalled()).toBe(true)
    expect(observeCalls).toHaveLength(1)
  })
})
