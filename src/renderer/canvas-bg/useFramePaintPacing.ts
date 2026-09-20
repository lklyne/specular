import { useCallback, useEffect, useRef } from 'react'

/**
 * Half a 60Hz display frame. A frame is due this much before its full
 * interval has passed, so a paint lands on the vsync nearest the page's
 * cadence instead of slipping to the one after it.
 */
const VSYNC_SLACK_MS = 8

/**
 * When a frame painted at `frameRate` is owed a repaint, given the last one.
 *
 * Every page's offscreen compositor runs on its own timer, so thirty pages at
 * 15fps arrive as 450 evenly spread frames a second — and one arrival repaints
 * the whole surface. Painting on each would hold the canvas at display rate
 * however low the pages' own rates are (measured: a third of GPU-process CPU
 * at thumbnail zoom). Holding a frame until its page's interval has passed
 * since the last paint shows every page at its own rate and no faster.
 */
export function framePaintDueAt(lastPaintAt: number, frameRate: number): number {
  return lastPaintAt + 1000 / frameRate - VSYNC_SLACK_MS
}

/**
 * Paces the repaints page frames ask for. `requestFramePaint` schedules
 * `requestPaint` for when the arriving frame is due; a faster page's frame
 * pulls a pending paint earlier. `notePaint` is called by every paint,
 * whatever asked for it — each one shows all the frames that have arrived.
 */
export function useFramePaintPacing(requestPaint: () => void): {
  requestFramePaint: (frameRate: number) => void
  notePaint: () => void
} {
  const lastPaintAt = useRef(0)
  const timer = useRef(0)
  const timerDueAt = useRef(Infinity)

  const clearTimer = useCallback(() => {
    window.clearTimeout(timer.current)
    timer.current = 0
    timerDueAt.current = Infinity
  }, [])

  const requestFramePaint = useCallback(
    (frameRate: number) => {
      const now = performance.now()
      const dueAt = framePaintDueAt(lastPaintAt.current, frameRate)
      if (dueAt <= now) {
        clearTimer()
        requestPaint()
        return
      }
      if (dueAt >= timerDueAt.current) return
      clearTimer()
      timerDueAt.current = dueAt
      timer.current = window.setTimeout(() => {
        clearTimer()
        requestPaint()
      }, dueAt - now)
    },
    [clearTimer, requestPaint],
  )

  const notePaint = useCallback(() => {
    lastPaintAt.current = performance.now()
    clearTimer()
  }, [clearTimer])

  useEffect(() => clearTimer, [clearTimer])

  return { requestFramePaint, notePaint }
}
