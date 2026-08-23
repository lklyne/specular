// A pan arrives as a stream of wheel and trackpad deltas with no phase, so
// there is no gesture boundary to read — a settle timer is the boundary.
//
// Deliberately separate from `zoom-motion.ts`, which has the same shape: that
// latch also gates device-emulation quantization, and a pan must never trip it.
// Quantizing snaps the emulation scale to a coarse grid, so a pan that raised
// that flag would re-raster every page at a different scale for no reason —
// the zoom storm, on the one gesture that has no zoom in it.
let inMotion = false
let settleTimer: ReturnType<typeof setTimeout> | null = null

/**
 * Matches the zoom settle. A trackpad's momentum commonly pauses for 150 to
 * 250ms inside one perceived flick; a shorter lease would drop the freeze in
 * the middle of it and show live views for a few frames.
 */
const SETTLE_MS = 300

export function isPanInMotion(): boolean {
  return inMotion
}

/** Call on every pan-only camera change. Enters motion mode and (re)schedules
 *  the settle that leaves it and runs `onSettle`. */
export function markPanMotion(onSettle: () => void): void {
  inMotion = true
  if (settleTimer) clearTimeout(settleTimer)
  settleTimer = setTimeout(() => {
    settleTimer = null
    inMotion = false
    onSettle()
  }, SETTLE_MS)
}

/** Drops a pending settle without running it, for when zoom takes the gesture
 *  over and its own settle becomes the one that releases the freeze. */
export function cancelPanMotion(): void {
  if (settleTimer) clearTimeout(settleTimer)
  settleTimer = null
  inMotion = false
}
