/**
 * The drift watchdog's whole value is the line it prints.
 *
 * A count names a cell and stops one bisect short of an answer, so each cell's
 * first sighting also logs the keys that disagree (ADR 0036 §7). That detail
 * walks values the watchdog does not control, and it runs inside the renderer's
 * snapshot-apply path — so a walk that throws must cost its own keys and
 * nothing else. During the second dogfooding run the counters printed all
 * session while not one `held→snapshot:` line ever did, which is exactly what a
 * silently-swallowed detail step looks like from the log.
 *
 * Mutation-verified by:
 * - replacing `describeCell`'s try/catch with a bare `describeDelta` call — the
 *   hostile-value test fails (the report is lost, and the throw escapes into
 *   the caller's snapshot apply);
 * - dropping the `held→snapshot:` warn — the first test fails.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { RuntimeStore } from '../../src/shared/runtime-store'

type Watchdog = typeof import('../../src/renderer/shared/runtime-store-drift')

let warn: ReturnType<typeof vi.spyOn>

/** Fresh module state per test: the watchdog reports each cell once per
 *  process, which is the behavior under test rather than something to reset. */
async function loadWatchdog(): Promise<Watchdog> {
  vi.resetModules()
  return import('../../src/renderer/shared/runtime-store-drift')
}

function lines(): string[] {
  return warn.mock.calls.map((call) => String(call[0]))
}

function store(entity: unknown, inspect: unknown): RuntimeStore {
  return {
    entities: { page_a: entity as never },
    slices: { inspect: inspect as never },
  }
}

describe('drift watchdog reporting', () => {
  beforeEach(() => {
    // The periodic counter arms on the first drift; nothing here waits for it.
    vi.useFakeTimers()
    warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  it('names the keys that disagree, for a drifted entity and a drifted slice', async () => {
    const { reportReconcileDrift } = await loadWatchdog()

    reportReconcileDrift(
      store({ id: 'page_a', kind: 'page', scrollY: 0 }, { sessionId: 'one' }),
      store({ id: 'page_a', kind: 'page', scrollY: 120 }, { sessionId: 'two' }),
    )

    expect(lines()).toEqual([
      '[drift-watchdog] first drift after 1 snapshots: slice:inspect entity:page_a',
      '[drift-watchdog] slice:inspect held→snapshot: sessionId="one"→"two"',
      '[drift-watchdog] entity:page_a held→snapshot: scrollY=0→120',
    ])
  })

  it('reports a cell whose detail walk throws, and keeps reporting the rest', async () => {
    const { reportReconcileDrift } = await loadWatchdog()

    // A key only the held side carries: the store diff never reads it (it walks
    // the incoming value's keys), the detail walk does — so the throw lands in
    // the describe step alone, which is the seam under test.
    const held = {
      sessionId: 'one',
      get elementRect(): never {
        throw new TypeError('detached node')
      },
    }

    expect(() =>
      reportReconcileDrift(
        store({ id: 'page_a', kind: 'page', scrollY: 0 }, held),
        store({ id: 'page_a', kind: 'page', scrollY: 120 }, { sessionId: 'two' }),
      ),
    ).not.toThrow()

    expect(lines()).toEqual([
      '[drift-watchdog] first drift after 1 snapshots: slice:inspect entity:page_a',
      '[drift-watchdog] slice:inspect held→snapshot: describe-failed=detached node',
      '[drift-watchdog] entity:page_a held→snapshot: scrollY=0→120',
    ])
  })
})
