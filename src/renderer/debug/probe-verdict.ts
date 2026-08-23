/**
 * Turns a probe's two measurement windows into a verdict. Pure — shared by the
 * panel and the copyable report so both call the same result the same thing.
 */

import type { VisibilityProbePageResult, VisibilityProbeSample } from '../../shared/process-metrics'

/** Frames per second below which a page is not animating in any real sense. */
const THROTTLED_FPS = 5

export type Verdict = 'throttled' | 'unchanged' | 'unknown'

export const VERDICT_LABEL: Record<Verdict, string> = {
  throttled: 'Throttled',
  unchanged: 'Still awake',
  unknown: 'No data',
}

export function perSecond(count: number, elapsedMs: number): number {
  if (elapsedMs <= 0) return 0
  return (count * 1000) / elapsedMs
}

export function verdictOf(page: VisibilityProbePageResult): Verdict {
  const { before, after } = page
  if (!before || !after) return 'unknown'
  const beforeFps = perSecond(before.frames, before.elapsedMs)
  const afterFps = perSecond(after.frames, after.elapsedMs)
  if (after.visibilityState === 'hidden' && afterFps < THROTTLED_FPS) return 'throttled'
  if (beforeFps >= THROTTLED_FPS && afterFps < THROTTLED_FPS) return 'throttled'
  return 'unchanged'
}

/** e.g. "visible · 60 fps · 9.9 timers/s" */
export function describeSample(sample: VisibilityProbeSample | null): string {
  if (!sample) return '—'
  return [
    sample.visibilityState,
    `${perSecond(sample.frames, sample.elapsedMs).toFixed(0)} fps`,
    `${perSecond(sample.timerTicks, sample.elapsedMs).toFixed(1)} timers/s`,
  ].join(' · ')
}
