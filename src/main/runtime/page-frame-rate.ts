/**
 * Frame-rate LOD for offscreen pages.
 *
 * A page paints at its authored CSS size whatever the camera does, so at
 * canvas zoom-out a thumbnail-sized page still produces full frames. The GPU
 * cost of that pipeline is dominated by per-frame work — compositor
 * scheduling, texture export, ImageBitmap copies — not by pixels (measured:
 * halving raster resolution moved GPU CPU by ~1%; frame count moves it
 * linearly). So detail is graded on the temporal axis: a page earns frame
 * rate by how large it appears on screen, and sharpness is never traded.
 *
 * Tier switches carry hysteresis so a zoom gesture hovering near a boundary
 * does not flap the compositor's BeginFrame cadence.
 */

export const FULL_FRAME_RATE = 60

/**
 * Ordered large-to-small; a scale earns the first tier it reaches. The full
 * rate's boundary sits under 0.5 so that 50% zoom is smooth from either
 * direction once hysteresis is applied — reached zooming in at ~0.49, held
 * zooming out to 0.4.
 */
const TIERS: readonly { minScale: number; fps: number }[] = [
  { minScale: 0.44, fps: FULL_FRAME_RATE },
  { minScale: 0.25, fps: 30 },
  { minScale: 0, fps: 15 },
]

/**
 * Fractional distance past a tier boundary the scale must travel before the
 * rate switches. 0.1 puts the down-switch at 0.4 and the up-switch at
 * ~0.49 around the 0.44 boundary.
 */
const HYSTERESIS = 0.1

function tierFps(scale: number): number {
  for (const tier of TIERS) if (scale >= tier.minScale) return tier.fps
  return TIERS[TIERS.length - 1].fps
}

/**
 * The frame rate a page showing at `displayScale` (screen px per CSS px)
 * should paint at, given the rate it paints at now. Pass the returned value
 * back as `currentFps` on the next evaluation — the pair is what makes the
 * boundaries sticky.
 */
export function frameRateForDisplayScale(displayScale: number, currentFps: number): number {
  const target = tierFps(displayScale)
  if (target === currentFps) return currentFps
  // The nudged scale's tier, not all-or-nothing on the target: a jump that
  // crosses two boundaries and lands inside the far one's margin still owes
  // the tier in between.
  const nudged =
    target > currentFps ? displayScale * (1 - HYSTERESIS) : displayScale * (1 + HYSTERESIS)
  return tierFps(nudged)
}
