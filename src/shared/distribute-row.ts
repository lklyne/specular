/**
 * Distribute kernel — evens out edge-to-edge gaps along the dominant axis of a
 * loose multi-selection, keeping the first and last items fixed. Pure and
 * side-effect-free; unit-tested in isolation.
 *
 * The on-ramp to a reorderable row (ADR 0015 D7): distribute equalizes gaps
 * using the *same* axis rule and gap definition as `detectReorderableRow`, so
 * the output is reorder-eligible by construction. The two kernels share
 * `dominantAxis` from `reorder-row.ts` — never a copy-pasted predicate.
 */

import { type Box, dominantAxis } from './reorder-row'

export interface DistributeResult {
  axis: 'x' | 'y'
  /** Positions of boxes that moved (boxes already at their target are omitted). */
  positions: Map<string, { x: number; y: number }>
}

/** Leading edge (top/left) of a box along the given axis. */
function leadingEdge(box: Box, axis: 'x' | 'y'): number {
  return axis === 'x' ? box.x : box.y
}

/** Trailing edge (bottom/right) of a box along the given axis. */
function trailingEdge(box: Box, axis: 'x' | 'y'): number {
  return axis === 'x' ? box.x + box.width : box.y + box.height
}

/** Size of a box along the given axis. */
function sizeAlong(box: Box, axis: 'x' | 'y'): number {
  return axis === 'x' ? box.width : box.height
}

/**
 * Even out edge-to-edge gaps along the dominant axis, endpoints fixed, each
 * box keeping its cross-axis coordinate. Returns null when there is nothing to
 * do: fewer than 3 boxes, or already even within tolerance.
 *
 * gap = (extent − Σ sizeAlongAxis) / (n − 1)  — may be negative (overlap) —
 * first item's leading edge and last item's trailing edge stay put.
 */
export function distributeRowPositions(
  boxes: readonly Box[],
  opts?: { axis?: 'x' | 'y'; tolerance?: number },
): DistributeResult | null {
  if (boxes.length < 3) return null

  const axis = opts?.axis ?? dominantAxis(boxes)
  const tolerance = opts?.tolerance ?? 1

  const sorted = [...boxes].sort((a, b) => leadingEdge(a, axis) - leadingEdge(b, axis))
  const first = sorted[0]
  const last = sorted[sorted.length - 1]

  const extent = trailingEdge(last, axis) - leadingEdge(first, axis)
  const totalSize = sorted.reduce((sum, b) => sum + sizeAlong(b, axis), 0)
  const gap = (extent - totalSize) / (sorted.length - 1)

  const positions = new Map<string, { x: number; y: number }>()
  let cursor = leadingEdge(first, axis)
  for (const box of sorted) {
    const target =
      axis === 'x' ? { x: cursor, y: box.y } : { x: box.x, y: cursor }
    if (Math.abs(target.x - box.x) > tolerance || Math.abs(target.y - box.y) > tolerance) {
      positions.set(box.id, target)
    }
    cursor += sizeAlong(box, axis) + gap
  }

  if (positions.size === 0) return null
  return { axis, positions }
}
