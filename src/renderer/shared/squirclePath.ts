/**
 * Generate an SVG path `d` string for a squircle (Apple-style continuous
 * curvature) rounded rectangle.
 *
 * smoothing: 0 = standard circular arc corners, 1 = maximum squircle effect.
 * Apple iOS uses approximately 0.6.
 */

const KAPPA = 0.5522847498 // cubic Bezier approximation of a quarter circle

export function squirclePath(
  x: number,
  y: number,
  w: number,
  h: number,
  cornerRadius: number,
  direction: 'cw' | 'ccw' = 'cw',
  smoothing = 0.6,
): string {
  const maxR = Math.min(w, h) / 2
  const r = Math.min(cornerRadius, maxR)

  if (r <= 0) {
    return direction === 'cw'
      ? `M${x},${y}L${x + w},${y}L${x + w},${y + h}L${x},${y + h}Z`
      : `M${x},${y}L${x},${y + h}L${x + w},${y + h}L${x + w},${y}Z`
  }

  // Approach distance: how far from the geometric corner the curve begins.
  // For circular arcs this equals r; for squircles the curve starts earlier,
  // producing the characteristic smooth entry.
  const a = Math.min(r * (1 + smoothing * 0.3), maxR)

  // Handle length: interpolated between circular kappa and a longer squircle
  // handle for smoother curvature transition.
  const ha = a * (KAPPA + (1 - KAPPA) * smoothing * 0.5)

  const l = x
  const ri = x + w
  const t = y
  const b = y + h

  if (direction === 'cw') {
    return (
      `M${l + a},${t}` +
      `L${ri - a},${t}` +
      `C${ri - a + ha},${t} ${ri},${t + a - ha} ${ri},${t + a}` +
      `L${ri},${b - a}` +
      `C${ri},${b - a + ha} ${ri - a + ha},${b} ${ri - a},${b}` +
      `L${l + a},${b}` +
      `C${l + a - ha},${b} ${l},${b - a + ha} ${l},${b - a}` +
      `L${l},${t + a}` +
      `C${l},${t + a - ha} ${l + a - ha},${t} ${l + a},${t}` +
      'Z'
    )
  }

  // Counter-clockwise (for evenodd cutout)
  return (
    `M${l + a},${t}` +
    `C${l + a - ha},${t} ${l},${t + a - ha} ${l},${t + a}` +
    `L${l},${b - a}` +
    `C${l},${b - a + ha} ${l + a - ha},${b} ${l + a},${b}` +
    `L${ri - a},${b}` +
    `C${ri - a + ha},${b} ${ri},${b - a + ha} ${ri},${b - a}` +
    `L${ri},${t + a}` +
    `C${ri},${t + a - ha} ${ri - a + ha},${t} ${ri - a},${t}` +
    'Z'
  )
}
