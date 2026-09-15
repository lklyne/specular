/**
 * Frame-timing statistics for the canvas benchmark.
 *
 * Deliberately percentile-based rather than a busy-time ratio: a "GPU process
 * was 97.5% busy" number saturates, so once the compositor is pinned it can no
 * longer say how much worse a change made things. Frame intervals keep
 * separating after that point, which is what a regression check needs.
 */

/** One phase's rAF cadence, all values ms except the counts. */
export interface FrameIntervalSummary {
  draws: number
  drawFps: number
  meanFrameMs: number
  p50FrameMs: number
  p95FrameMs: number
  p99FrameMs: number
  maxFrameMs: number
  /** Intervals past one refresh interval — a frame the compositor missed. */
  missedFrames: number
  /** Intervals past 1.5x refresh — long enough to read as a hitch. */
  longFrames: number
}

const EMPTY_SUMMARY: FrameIntervalSummary = {
  draws: 0,
  drawFps: 0,
  meanFrameMs: 0,
  p50FrameMs: 0,
  p95FrameMs: 0,
  p99FrameMs: 0,
  maxFrameMs: 0,
  missedFrames: 0,
  longFrames: 0,
}

/**
 * Nearest-rank percentile over an already-sorted ascending array. Nearest-rank
 * rather than interpolated so every reported value is one the run actually
 * measured — an interpolated p95 of a bimodal frame distribution names a
 * duration that never occurred.
 */
export function percentileOfSorted(sorted: readonly number[], fraction: number): number {
  if (sorted.length === 0) return 0
  const rank = Math.ceil(fraction * sorted.length)
  return sorted[Math.min(sorted.length - 1, Math.max(0, rank - 1))]
}

export function medianOf(values: readonly number[]): number {
  if (values.length === 0) return 0
  const sorted = [...values].sort((a, b) => a - b)
  const mid = sorted.length >> 1
  return sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid]
}

/** Summarizes the rAF intervals recorded across one benchmark phase. */
export function summarizeFrameIntervals(
  intervalsMs: readonly number[],
  refreshMs: number,
): FrameIntervalSummary {
  if (intervalsMs.length === 0) return { ...EMPTY_SUMMARY }
  const sorted = [...intervalsMs].sort((a, b) => a - b)
  let total = 0
  let missed = 0
  let long = 0
  // A frame arriving a hair past its deadline is measurement noise, not a
  // miss; the tolerance keeps a 120Hz run from counting 8.34ms as dropped.
  const missedThreshold = refreshMs * 1.05
  const longThreshold = refreshMs * 1.5
  for (const interval of intervalsMs) {
    total += interval
    if (interval > missedThreshold) missed++
    if (interval > longThreshold) long++
  }
  return {
    draws: intervalsMs.length,
    drawFps: total > 0 ? (intervalsMs.length / total) * 1000 : 0,
    meanFrameMs: total / intervalsMs.length,
    p50FrameMs: percentileOfSorted(sorted, 0.5),
    p95FrameMs: percentileOfSorted(sorted, 0.95),
    p99FrameMs: percentileOfSorted(sorted, 0.99),
    maxFrameMs: sorted[sorted.length - 1],
    missedFrames: missed,
    longFrames: long,
  }
}

/**
 * Collapses the same phase across repeated runs to its per-field median.
 *
 * Median rather than mean across runs because the failure mode being guarded
 * against is one run catching a background process; a mean carries that spike
 * into the reported number, a median discards it.
 */
export function medianSummary(runs: readonly FrameIntervalSummary[]): FrameIntervalSummary {
  if (runs.length === 0) return { ...EMPTY_SUMMARY }
  const field = (pick: (s: FrameIntervalSummary) => number): number =>
    medianOf(runs.map(pick))
  return {
    draws: field((s) => s.draws),
    drawFps: field((s) => s.drawFps),
    meanFrameMs: field((s) => s.meanFrameMs),
    p50FrameMs: field((s) => s.p50FrameMs),
    p95FrameMs: field((s) => s.p95FrameMs),
    p99FrameMs: field((s) => s.p99FrameMs),
    maxFrameMs: field((s) => s.maxFrameMs),
    missedFrames: field((s) => s.missedFrames),
    longFrames: field((s) => s.longFrames),
  }
}

/**
 * Time spent inside one paint callback. Separate from the interval summary
 * because a paint duration is not a cadence: a run can hold 120fps while each
 * paint grows, right up until the frame it doesn't.
 */
export interface PaintCostSummary {
  meanMs: number
  p95Ms: number
  maxMs: number
}

const EMPTY_PAINT_COST: PaintCostSummary = { meanMs: 0, p95Ms: 0, maxMs: 0 }

export function summarizePaintCosts(costsMs: readonly number[]): PaintCostSummary {
  if (costsMs.length === 0) return { ...EMPTY_PAINT_COST }
  const sorted = [...costsMs].sort((a, b) => a - b)
  let total = 0
  for (const cost of costsMs) total += cost
  return {
    meanMs: total / costsMs.length,
    p95Ms: percentileOfSorted(sorted, 0.95),
    maxMs: sorted[sorted.length - 1],
  }
}

export function medianPaintCost(runs: readonly PaintCostSummary[]): PaintCostSummary {
  if (runs.length === 0) return { ...EMPTY_PAINT_COST }
  return {
    meanMs: medianOf(runs.map((r) => r.meanMs)),
    p95Ms: medianOf(runs.map((r) => r.p95Ms)),
    maxMs: medianOf(runs.map((r) => r.maxMs)),
  }
}

export function roundPaintCost(cost: PaintCostSummary): PaintCostSummary {
  const round = (v: number): number => Math.round(v * 100) / 100
  return { meanMs: round(cost.meanMs), p95Ms: round(cost.p95Ms), maxMs: round(cost.maxMs) }
}

/** Rounds every field to 2dp for a report a human reads and a diff compares. */
export function roundSummary(summary: FrameIntervalSummary): FrameIntervalSummary {
  const round = (v: number): number => Math.round(v * 100) / 100
  return {
    draws: summary.draws,
    drawFps: round(summary.drawFps),
    meanFrameMs: round(summary.meanFrameMs),
    p50FrameMs: round(summary.p50FrameMs),
    p95FrameMs: round(summary.p95FrameMs),
    p99FrameMs: round(summary.p99FrameMs),
    maxFrameMs: round(summary.maxFrameMs),
    missedFrames: summary.missedFrames,
    longFrames: summary.longFrames,
  }
}
