// Shared visuals for popup color swatches. Two problems both stem from
// near-white swatches (the neutral slot) on the near-white light popup surface:
// the dot itself vanishes, and a selection ring drawn in the swatch's own color
// vanishes too. These helpers give every dot a hairline and swap the active
// ring to a contrasting gray when the swatch is too light.

/** Perceived luminance of a #RRGGBB hex, 0 (black) – 1 (white). */
function luminance(hex: string): number {
  if (!hex.startsWith('#') || hex.length !== 7) return 0
  const r = parseInt(hex.slice(1, 3), 16) / 255
  const g = parseInt(hex.slice(3, 5), 16) / 255
  const b = parseInt(hex.slice(5, 7), 16) / 255
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

/**
 * Active-swatch selection ring. The swatch's own color for saturated hues (a
 * subtle matching ring), a contrasting gray for near-white swatches that would
 * otherwise disappear against the popup surface.
 */
function swatchRingColor(swatch: string, isDark: boolean): string {
  // Only near-white swatches (the neutral slot, ~0.98) flip to gray; the soft
  // pastels (yellow, the lightest hue, is ~0.88) keep their matching colored ring.
  if (luminance(swatch) > 0.92) return isDark ? '#a1a1aa' : '#71717a'
  return swatch
}

/** Hairline around the swatch dot so light/near-white swatches stay visible
 *  against the popup surface even when unselected. */
export function swatchDotShadow(isDark: boolean): string {
  return `inset 0 0 0 1px ${isDark ? 'rgba(255,255,255,0.18)' : 'rgba(0,0,0,0.12)'}`
}

/**
 * The active-swatch selection ring, as an inset shadow so it thickens inward
 * (2px) without enlarging the swatch. `undefined` when not selected.
 */
export function swatchRingShadow(
  swatch: string,
  active: boolean,
  isDark: boolean,
): string | undefined {
  return active ? `inset 0 0 0 2px ${swatchRingColor(swatch, isDark)}` : undefined
}
