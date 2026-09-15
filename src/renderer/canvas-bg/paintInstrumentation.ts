/**
 * Measurement taps on canvas-bg's item pass, for the pan/zoom benchmark.
 *
 * Mirrors main's `setBuildMsSink`: production calls the record functions
 * unconditionally, and they cost one null check or one increment until a
 * benchmark arms the recorder. Keeping the tap in the paint path rather than
 * reconstructing cost from a trace is what lets the benchmark report paint
 * time per phase without a 500MB capture to parse.
 */

/** Non-null only while a run is recording. */
let paintCosts: number[] | null = null

/** Monotonic across the session; the benchmark reads deltas. */
let pageFramesReceived = 0

export function armPaintRecorder(): void {
  paintCosts = []
}

export function disarmPaintRecorder(): void {
  paintCosts = null
}

/** Time one item-surface paint took, ms. Ignored while disarmed. */
export function recordPaintCost(ms: number): void {
  paintCosts?.push(ms)
}

/** One page texture arrived from main. Counted whether or not armed. */
export function recordPageFrame(): void {
  pageFramesReceived++
}

/** Costs recorded since the last drain; empties the buffer. */
export function drainPaintCosts(): number[] {
  if (!paintCosts) return []
  const drained = paintCosts
  paintCosts = []
  return drained
}

export function framesReceivedSoFar(): number {
  return pageFramesReceived
}
