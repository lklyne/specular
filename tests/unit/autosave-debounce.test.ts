/**
 * Autosave debounce unit tests.
 *
 * Drives the pure `scheduleSpaceAutosave` / `flushSpaceAutosaveSync`
 * helpers from src/main/runtime/space-persistence.ts directly — no
 * Electron involved. Both helpers take their state (timer ref, persist
 * predicate, write-callback) as injected options, so unit testing is
 * faithful to the real call shape.
 *
 * Mutation-verified by commenting out the `clearTimeout(options.autosaveTimer)`
 * call inside `scheduleSpaceAutosave` — "coalesces rapid back-to-back
 * mutations" then sees five writes instead of one.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  AUTOSAVE_DEBOUNCE_MS,
  flushSpaceAutosaveSync,
  scheduleSpaceAutosave,
} from '../../src/main/runtime/space-persistence'

function makeHarness(opts: { shouldPersist?: () => boolean } = {}) {
  let timer: NodeJS.Timeout | null = null
  const save = vi.fn()
  const setTimer = (t: NodeJS.Timeout | null) => {
    timer = t
  }
  const shouldPersist = opts.shouldPersist ?? (() => true)
  return {
    save,
    schedule: () =>
      scheduleSpaceAutosave({
        autosaveTimer: timer,
        setAutosaveTimer: setTimer,
        shouldPersist,
        saveSpaceStore: save,
      }),
    flush: () =>
      flushSpaceAutosaveSync({
        autosaveTimer: timer,
        setAutosaveTimer: setTimer,
        saveSpaceStore: save,
      }),
    getTimer: () => timer,
  }
}

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('scheduleSpaceAutosave', () => {
  it('debounces a single mutation', () => {
    const h = makeHarness()
    h.schedule()
    expect(h.save).not.toHaveBeenCalled()

    vi.advanceTimersByTime(AUTOSAVE_DEBOUNCE_MS - 1)
    expect(h.save).not.toHaveBeenCalled()

    vi.advanceTimersByTime(1)
    expect(h.save).toHaveBeenCalledTimes(1)
  })

  it('coalesces rapid back-to-back mutations into a single write', () => {
    const h = makeHarness()
    for (let i = 0; i < 5; i++) {
      h.schedule()
      vi.advanceTimersByTime(10) // well under the debounce window
    }
    expect(h.save).not.toHaveBeenCalled()

    // After the final mutation, fire forward past the debounce.
    vi.advanceTimersByTime(AUTOSAVE_DEBOUNCE_MS)
    expect(h.save).toHaveBeenCalledTimes(1)
  })

  it('no-ops when shouldPersist returns false', () => {
    const h = makeHarness({ shouldPersist: () => false })
    h.schedule()
    vi.runAllTimers()
    expect(h.save).not.toHaveBeenCalled()
  })

  it('debounce window matches the documented 350ms', () => {
    expect(AUTOSAVE_DEBOUNCE_MS).toBe(350)
  })
})

describe('flushSpaceAutosaveSync', () => {
  it('writes immediately even if a pending timer exists', () => {
    const h = makeHarness()
    h.schedule()
    expect(h.save).not.toHaveBeenCalled()

    h.flush()
    expect(h.save).toHaveBeenCalledTimes(1)

    // The pending timer should have been cleared — advancing time past the
    // debounce should not produce a second write.
    vi.advanceTimersByTime(AUTOSAVE_DEBOUNCE_MS + 100)
    expect(h.save).toHaveBeenCalledTimes(1)
  })

  it('writes when no timer is pending (flush-on-quit shape)', () => {
    const h = makeHarness()
    h.flush()
    expect(h.save).toHaveBeenCalledTimes(1)
  })
})
