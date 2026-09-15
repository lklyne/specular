// Guards the benchmark's frame-timing statistics. These exist because the
// prior pan/zoom metric — GPU-process busy time as a share of wall-clock —
// saturates near 100% and stops separating once the compositor is pinned, so
// it cannot size a regression past that point. Percentiles and long-frame
// counts keep separating, and the median-across-runs collapse is what makes a
// single background-process spike not become the reported number.
// Mutation-verified by (1) making percentileOfSorted interpolate between ranks
// and confirming "reports percentiles a run actually measured" fails, (2)
// dropping the 1.05 tolerance from the missed-frame threshold and confirming
// "does not count refresh-rate jitter as a missed frame" fails, and (3)
// averaging instead of taking the median in medianSummary and confirming
// "takes the median across runs, not the mean" fails.

import { describe, expect, it } from 'vitest'
import {
  medianOf,
  medianSummary,
  percentileOfSorted,
  roundSummary,
  summarizeFrameIntervals,
  type FrameIntervalSummary,
} from '../../src/shared/frame-stats'

/** A 120Hz run: `count` frames on budget, then the listed stragglers. */
function run(count: number, ...stragglers: number[]): number[] {
  return [...Array.from({ length: count }, () => 8.33), ...stragglers]
}

describe('summarizeFrameIntervals', () => {
  it('reports a clean run as pinned to the refresh budget', () => {
    const summary = roundSummary(summarizeFrameIntervals(run(100), 8.33))
    expect(summary.draws).toBe(100)
    expect(summary.drawFps).toBeCloseTo(120, 0)
    expect(summary.missedFrames).toBe(0)
    expect(summary.longFrames).toBe(0)
    expect(summary.maxFrameMs).toBeCloseTo(8.33, 2)
  })

  it('separates a missed frame from a hitch', () => {
    // 10ms is past one refresh interval but under 1.5x; 20ms is past both.
    const summary = summarizeFrameIntervals(run(98, 10, 20), 8.33)
    expect(summary.missedFrames).toBe(2)
    expect(summary.longFrames).toBe(1)
    expect(summary.maxFrameMs).toBe(20)
  })

  it('does not count refresh-rate jitter as a missed frame', () => {
    // vsync noise around a 8.33ms budget is not a dropped frame.
    const summary = summarizeFrameIntervals([8.33, 8.4, 8.6, 8.7, 8.33], 8.33)
    expect(summary.missedFrames).toBe(0)
  })

  it('reports percentiles a run actually measured', () => {
    // Bimodal: 95 fast frames and 5 slow ones, nothing in between. p95 lands
    // exactly on the boundary, where an interpolated percentile would blend
    // the two modes into a 9.6ms frame that never occurred.
    const intervals = [...Array.from({ length: 95 }, () => 8), ...Array.from({ length: 5 }, () => 40)]
    const summary = summarizeFrameIntervals(intervals, 8.33)
    expect(summary.p50FrameMs).toBe(8)
    expect(summary.p95FrameMs).toBe(8)
    expect(summary.p99FrameMs).toBe(40)
  })

  it('keeps separating after the frame budget is blown', () => {
    // The property the busy-ratio metric lacked: twice as slow reads as twice
    // as slow, rather than both runs pinning at one ceiling.
    const bad = summarizeFrameIntervals(Array.from({ length: 50 }, () => 33), 8.33)
    const worse = summarizeFrameIntervals(Array.from({ length: 50 }, () => 66), 8.33)
    expect(bad.longFrames).toBe(50)
    expect(worse.longFrames).toBe(50)
    expect(worse.p95FrameMs).toBeGreaterThan(bad.p95FrameMs * 1.9)
    expect(worse.drawFps).toBeLessThan(bad.drawFps / 1.9)
  })

  it('returns zeros for a phase that recorded nothing', () => {
    expect(summarizeFrameIntervals([], 8.33)).toEqual({
      draws: 0,
      drawFps: 0,
      meanFrameMs: 0,
      p50FrameMs: 0,
      p95FrameMs: 0,
      p99FrameMs: 0,
      maxFrameMs: 0,
      missedFrames: 0,
      longFrames: 0,
    })
  })
})

describe('percentileOfSorted', () => {
  it('uses nearest rank, staying inside the sample', () => {
    const sorted = [1, 2, 3, 4, 5]
    expect(percentileOfSorted(sorted, 0.5)).toBe(3)
    expect(percentileOfSorted(sorted, 0.95)).toBe(5)
    expect(percentileOfSorted(sorted, 0)).toBe(1)
  })

  it('is zero for an empty sample', () => {
    expect(percentileOfSorted([], 0.95)).toBe(0)
  })
})

describe('medianOf', () => {
  it('averages the middle pair for an even count', () => {
    expect(medianOf([4, 1, 3, 2])).toBe(2.5)
  })

  it('ignores input order', () => {
    expect(medianOf([9, 1, 5])).toBe(5)
  })
})

describe('medianSummary', () => {
  function summaryWith(p95: number, long: number): FrameIntervalSummary {
    return {
      draws: 100,
      drawFps: 120,
      meanFrameMs: 8.33,
      p50FrameMs: 8.33,
      p95FrameMs: p95,
      p99FrameMs: p95,
      maxFrameMs: p95,
      missedFrames: 0,
      longFrames: long,
    }
  }

  it('takes the median across runs, not the mean', () => {
    // One run caught a background process. The median discards it; a mean
    // would carry that spike into the reported number.
    const runs = [summaryWith(9, 0), summaryWith(9, 0), summaryWith(200, 40), summaryWith(9, 0), summaryWith(10, 1)]
    const median = medianSummary(runs)
    expect(median.p95FrameMs).toBe(9)
    expect(median.longFrames).toBe(0)
  })

  it('returns zeros when no run was measured', () => {
    expect(medianSummary([]).draws).toBe(0)
  })
})
