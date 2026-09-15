// Guards how a benchmark run is collapsed into its report. Both helpers exist
// because the raw numbers mislead if aggregated naively: page-host counters
// are cumulative across a session, so a run that reported them directly would
// include every frame since launch; and maxOutstandingTextures is a high-water
// mark, so differencing it would hide the pool pressure it exists to surface —
// the signal that identified the texture-pool cap as the ceiling at 40 pages.
// Mutation-verified by (1) reporting after-values instead of differencing them
// in diffPageHostStats and confirming "counts only what the run itself did"
// fails, (2) differencing maxOutstandingTextures like the cumulative fields and
// confirming "keeps the pool high-water mark" fails, and (3) sorting the phase
// map in medianPhases and confirming "keeps phases in gesture order" fails.

import { describe, expect, it } from 'vitest'
import {
  diffPageHostStats,
  medianPhases,
  type CanvasBenchRunResult,
  type PageHostStats,
} from '../../src/shared/canvas-bench'
import type { FrameIntervalSummary, PaintCostSummary } from '../../src/shared/frame-stats'

function host(pageId: string, over: Partial<PageHostStats> = {}): PageHostStats {
  return {
    pageId,
    framesReceived: 0,
    popupFrames: 0,
    framesWithoutTexture: 0,
    framesDroppedForPoolPressure: 0,
    sendFailures: 0,
    outstandingTextures: 0,
    maxOutstandingTextures: 0,
    releaseLatencyMs: null,
    ...over,
  }
}

describe('diffPageHostStats', () => {
  it('counts only what the run itself did', () => {
    // The session had already delivered 1000 frames before the run started.
    const before = [host('a', { framesReceived: 1000, framesWithoutTexture: 40 })]
    const after = [host('a', { framesReceived: 1240, framesWithoutTexture: 46 })]
    const totals = diffPageHostStats(before, after)
    expect(totals.framesReceived).toBe(240)
    expect(totals.framesWithoutTexture).toBe(6)
  })

  it('keeps the pool high-water mark rather than differencing it', () => {
    // The cap was already touched before the run; the run touched it again.
    // Differencing would report 0 and hide the pressure entirely.
    const before = [host('a', { maxOutstandingTextures: 6 })]
    const after = [host('a', { maxOutstandingTextures: 6 })]
    expect(diffPageHostStats(before, after).maxOutstandingTextures).toBe(6)
  })

  it('counts a page opened mid-run in full', () => {
    const totals = diffPageHostStats([], [host('new', { framesReceived: 90 })])
    expect(totals.hosts).toBe(1)
    expect(totals.framesReceived).toBe(90)
  })

  it('sums across hosts and averages the latency of those reporting one', () => {
    const after = [
      host('a', { framesReceived: 10, releaseLatencyMs: 2 }),
      host('b', { framesReceived: 5, releaseLatencyMs: 4 }),
      host('c', { framesReceived: 1, releaseLatencyMs: null }),
    ]
    const totals = diffPageHostStats([], after)
    expect(totals.framesReceived).toBe(16)
    expect(totals.releaseLatencyMs).toBe(3)
  })

  it('reports no latency when no host measured one', () => {
    expect(diffPageHostStats([], [host('a')]).releaseLatencyMs).toBeNull()
  })
})

function frames(p95: number): FrameIntervalSummary {
  return {
    draws: 120,
    drawFps: 120,
    meanFrameMs: 8.33,
    p50FrameMs: 8.33,
    p95FrameMs: p95,
    p99FrameMs: p95,
    maxFrameMs: p95,
    missedFrames: 0,
    longFrames: 0,
  }
}

const paint: PaintCostSummary = { meanMs: 1, p95Ms: 2, maxMs: 3 }

function run(...p95s: number[]): CanvasBenchRunResult {
  // Phase order here is the gesture order the driver runs them in.
  const ids = ['slow-pan', 'slow-zoom', 'fast-diagonal-pan'] as const
  return {
    refreshMs: 8.33,
    phases: ids.map((phase, i) => ({
      phase,
      durationMs: 2000,
      steps: 125,
      framesReceived: 200,
      frames: frames(p95s[i]),
      paintCost: paint,
    })),
  }
}

describe('medianPhases', () => {
  it('takes the median per phase across runs', () => {
    const median = medianPhases([run(9, 20, 30), run(50, 21, 31), run(10, 22, 32)])
    expect(median.find((p) => p.phase === 'slow-pan')?.frames.p95FrameMs).toBe(10)
    expect(median.find((p) => p.phase === 'slow-zoom')?.frames.p95FrameMs).toBe(21)
  })

  it('keeps phases in gesture order, not alphabetical', () => {
    // Alphabetical would put fast-diagonal-pan first, so a report would read
    // in an order no gesture was ever driven in.
    expect(medianPhases([run(9, 20, 30)]).map((p) => p.phase)).toEqual([
      'slow-pan',
      'slow-zoom',
      'fast-diagonal-pan',
    ])
  })

  it('carries the step count through, since the workload is step-fixed', () => {
    expect(medianPhases([run(9, 20, 30)])[0].steps).toBe(125)
  })

  it('returns nothing when no run was measured', () => {
    expect(medianPhases([])).toEqual([])
  })
})
