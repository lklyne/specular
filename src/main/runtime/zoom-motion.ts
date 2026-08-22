// Coarse-grain device-emulation scale during zoom gestures so re-raster fires
// only at bucket crossings, not every tick. Exact scale is restored on settle.
let inMotion = false
let settleTimer: ReturnType<typeof setTimeout> | null = null

/** Buckets per doubling of zoom. Higher = crisper mid-gesture but more re-raster. */
const BUCKETS_PER_OCTAVE = 4
/**
 * How long after the last zoom change we treat the gesture as settled.
 * macOS trackpad momentum commonly pauses for 150 to 250ms inside one perceived
 * fast pinch; a shorter lease exposes live page views in the middle.
 */
const SETTLE_MS = 300

export function isZoomInMotion(): boolean {
  return inMotion
}

/** Call on every zoom change. Enters motion mode and (re)schedules a settle that
 * exits motion mode and runs `onSettle`. The callback should re-run layout so
 * pages re-emulate at the exact scale. */
export function markZoomMotion(onSettle: () => void): void {
  inMotion = true
  if (settleTimer) clearTimeout(settleTimer)
  settleTimer = setTimeout(() => {
    settleTimer = null
    inMotion = false
    onSettle()
  }, SETTLE_MS)
}

/** Snap zoom to a coarse log2 grid. Emulation only re-fires when the snapped
 * value changes; between buckets the compositor scales the existing texture. */
export function quantizeZoomForEmulation(zoom: number): number {
  if (zoom <= 0) return zoom
  return 2 ** (Math.round(Math.log2(zoom) * BUCKETS_PER_OCTAVE) / BUCKETS_PER_OCTAVE)
}

let panSettleTimer: ReturnType<typeof setTimeout> | null = null

/** Call on every pan change. During a pan the scene container rides the viewport
 * nudge (a cheap CSS translate), so the full canvas scene payload does NOT need
 * to be rebuilt and broadcast every tick. This schedules a single re-baseline
 * (`onSettle` re-dirties the canvas + re-runs layout) once panning stops, so
 * anything not covered by the nudge translate (agent cursors, freshly-computed
 * screen coords) reconciles to the resting camera. */
export function markPanMotion(onSettle: () => void): void {
  if (panSettleTimer) clearTimeout(panSettleTimer)
  panSettleTimer = setTimeout(() => {
    panSettleTimer = null
    onSettle()
  }, SETTLE_MS)
}
