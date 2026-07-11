/**
 * Trace-summary aggregation tests (src/shared/trace-summary.ts) — pure math
 * over Chrome-JSON trace events, no Electron involved.
 *
 * Mutation-verified by: changing `const selfUs = Math.max(0, dur - frame.childUs)`
 * to `const selfUs = Math.max(0, dur)` in summarizeTraceEvents() (dropping the
 * nested-child subtraction) makes the self-time test below fail — the parent
 * interval's self time comes back as 10ms instead of 6ms, and thread busyMs
 * comes back as 14 instead of 10.
 */

import { describe, expect, it } from 'vitest'
import { extractTraceEvents, summarizeTraceEvents, type RawTraceEvent } from '../../src/shared/trace-summary'

describe('extractTraceEvents', () => {
  it('accepts the { traceEvents: [...] } envelope', () => {
    const events: RawTraceEvent[] = [{ ph: 'X', pid: 1, tid: 1, ts: 0, dur: 1, name: 'a' }]
    expect(extractTraceEvents({ traceEvents: events })).toEqual(events)
  })

  it('accepts a bare array', () => {
    const events: RawTraceEvent[] = [{ ph: 'X', pid: 1, tid: 1, ts: 0, dur: 1, name: 'a' }]
    expect(extractTraceEvents(events)).toEqual(events)
  })

  it('returns [] for garbage input', () => {
    expect(extractTraceEvents(null)).toEqual([])
    expect(extractTraceEvents(undefined)).toEqual([])
    expect(extractTraceEvents('not json')).toEqual([])
    expect(extractTraceEvents(42)).toEqual([])
    expect(extractTraceEvents({})).toEqual([])
    expect(extractTraceEvents({ traceEvents: 'nope' })).toEqual([])
  })
})

describe('summarizeTraceEvents', () => {
  // One thread named via ph:'M' metadata (pid 1 / tid 1, "Browser" /
  // "CrBrowserMain") with a 10ms parent 'X' event containing a nested 4ms
  // child 'X' event; one unnamed thread (pid 2 / tid 2) carrying a matched
  // B/E pair equivalent to a 5ms interval; and an unmatched 'E' on a third,
  // otherwise-empty thread (pid 2 / tid 3) that must be ignored, not thrown.
  const combined: RawTraceEvent[] = [
    { ph: 'M', name: 'process_name', pid: 1, args: { name: 'Browser' } },
    { ph: 'M', name: 'thread_name', pid: 1, tid: 1, args: { name: 'CrBrowserMain' } },
    { ph: 'X', pid: 1, tid: 1, ts: 0, dur: 10_000, name: 'ParentTask' },
    { ph: 'X', pid: 1, tid: 1, ts: 1_000, dur: 4_000, name: 'ChildTask' },
    { ph: 'B', pid: 2, tid: 2, ts: 20_000, name: 'BEPairTask' },
    { ph: 'E', pid: 2, tid: 2, ts: 25_000 },
    { ph: 'E', pid: 2, tid: 3, ts: 30_000 },
  ]

  it('names threads from process_name/thread_name metadata, falling back to pid/tid', () => {
    const summary = summarizeTraceEvents(combined)
    const named = summary.threads.find((t) => t.pid === 1 && t.tid === 1)
    expect(named?.process).toBe('Browser')
    expect(named?.thread).toBe('CrBrowserMain')

    const unnamed = summary.threads.find((t) => t.pid === 2 && t.tid === 2)
    expect(unnamed?.process).toBe('pid 2')
    expect(unnamed?.thread).toBe('tid 2')
  })

  it('computes self time: a nested child is subtracted from its parent, but not double-counted', () => {
    const summary = summarizeTraceEvents(combined)
    const thread = summary.threads.find((t) => t.pid === 1 && t.tid === 1)
    // 10ms parent - 4ms nested child = 6ms parent self + 4ms child self = 10ms busy.
    expect(thread?.busyMs).toBe(10)

    const parentStat = summary.topEvents.find((e) => e.name === 'ParentTask')
    const childStat = summary.topEvents.find((e) => e.name === 'ChildTask')
    // topEvents totals are wall-clock durations, not self time.
    expect(parentStat).toMatchObject({ totalMs: 10, count: 1 })
    expect(childStat).toMatchObject({ totalMs: 4, count: 1 })
  })

  it('treats a matched B/E pair as an interval and ignores an unmatched E without throwing', () => {
    expect(() => summarizeTraceEvents(combined)).not.toThrow()
    const summary = summarizeTraceEvents(combined)

    const bePairThread = summary.threads.find((t) => t.pid === 2 && t.tid === 2)
    expect(bePairThread?.busyMs).toBe(5) // 25_000us - 20_000us = 5ms
    expect(bePairThread?.eventCount).toBe(1)

    // The unmatched E lives on pid 2 / tid 3, which never produced an
    // interval, so no thread entry for it should exist at all.
    expect(summary.threads.some((t) => t.pid === 2 && t.tid === 3)).toBe(false)
  })

  it('sorts threads by busyMs descending and spans durationMs from min ts to max end', () => {
    const summary = summarizeTraceEvents(combined)
    expect(summary.threads.map((t) => `${t.pid}:${t.tid}`)).toEqual(['1:1', '2:2'])
    expect(summary.threads[0].busyMs).toBeGreaterThan(summary.threads[1].busyMs)

    // min ts is 0 (ParentTask), max end is 25_000us (the B/E pair's E) -> 25ms.
    expect(summary.durationMs).toBe(25)
  })

  it('buckets a thread busy time into >=100ms buckets that sum to its busyMs', () => {
    const events: RawTraceEvent[] = [
      { ph: 'X', pid: 5, tid: 5, ts: 0, dur: 50_000, name: 'A' }, // 0-50ms
      { ph: 'X', pid: 5, tid: 5, ts: 120_000, dur: 30_000, name: 'B' }, // 120-150ms
    ]
    const summary = summarizeTraceEvents(events)

    expect(summary.bucketMs).toBeGreaterThanOrEqual(100)
    expect(summary.bucketCount).toBe(2)

    const thread = summary.threads.find((t) => t.pid === 5 && t.tid === 5)
    const series = summary.timeline.find((s) => s.key === '5:5')
    expect(series?.busyMs).toHaveLength(2)
    // A lands fully in bucket 0, B fully in bucket 1 — different buckets.
    expect(series?.busyMs[0]).toBeGreaterThan(0)
    expect(series?.busyMs[1]).toBeGreaterThan(0)

    const bucketSum = (series?.busyMs ?? []).reduce((a, b) => a + b, 0)
    expect(bucketSum).toBeCloseTo(thread?.busyMs ?? -1, 1)
  })

  it('produces marker entries for matching event names with correct counts, omitting zero-match markers', () => {
    const events: RawTraceEvent[] = [
      { ph: 'X', pid: 6, tid: 6, ts: 0, dur: 2_000, name: 'Display::DrawAndSwap' },
      { ph: 'X', pid: 6, tid: 6, ts: 5_000, dur: 1_000, name: 'Display::DrawAndSwap' },
      { ph: 'X', pid: 6, tid: 6, ts: 8_000, dur: 3_000, name: 'UpdateLayoutTree' },
    ]
    const summary = summarizeTraceEvents(events)

    const drawAndSwap = summary.markers.find((m) => m.label === 'Display draw & swap')
    expect(drawAndSwap).toMatchObject({ count: 2, totalMs: 3 })

    const layout = summary.markers.find((m) => m.label === 'Layout / style recalc')
    expect(layout).toMatchObject({ count: 1, totalMs: 3 })

    // No raster, surface-aggregation, compositor-commit, or device-emulation
    // events were present — those markers must not appear at all.
    expect(summary.markers.some((m) => m.count === 0)).toBe(false)
    expect(summary.markers).toHaveLength(2)
  })
})
