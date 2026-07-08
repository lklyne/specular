/**
 * Span-arrange kernel — the "tidy up" brain behind the row / column / grid
 * toolbar buttons. Pure and side-effect-free; unit-tested in isolation.
 *
 * Unlike a fixed-gap pack (which collapses a selection to a chosen gap from the
 * top-left, throwing away the layout the user built), these operations keep the
 * cluster's current footprint and only *regularize* the spacing inside it —
 * pinning the outer items and evening the gaps between the rest. That footprint
 * is visible and intentional; a fixed gap is neither. See the `arrange` verb's
 * `--gap` flag for the pack-to-a-gap path.
 *
 * - row    → one horizontal line: even the x-gaps across the current x-extent,
 *            align tops.
 * - column → one vertical line: even the y-gaps across the current y-extent,
 *            align left edges.
 * - grid   → keep the existing 2-D structure (rows and columns detected from
 *            current positions, holes preserved), regularize gaps on both axes.
 */

import { snapToGrid } from './gesture-utils'
import type { Box } from './reorder-row'

export type SpanArrangeMode = 'row' | 'column' | 'grid'

/** Floor for the gap between items/bands, so a too-tight footprint can't overlap. */
const MIN_GAP = 80

/**
 * Target positions for every input box (caller diffs against current). Returns
 * null when there is nothing to arrange (fewer than 2 boxes).
 */
export function arrangeInSpan(
  boxes: readonly Box[],
  mode: SpanArrangeMode,
): Map<string, { x: number; y: number }> | null {
  if (boxes.length < 2) return null

  const positions = new Map<string, { x: number; y: number }>()
  if (mode === 'grid') {
    const xs = evenBands(boxes, 'x')
    const ys = evenBands(boxes, 'y')
    for (const b of boxes) positions.set(b.id, { x: xs.get(b.id)!, y: ys.get(b.id)! })
    return positions
  }

  // row / column: one line. Distribute along the axis, align on the other.
  const axis = mode === 'row' ? 'x' : 'y'
  const along = evenCells(boxes, axis)
  const cross = snapToGrid(
    mode === 'row' ? Math.min(...boxes.map((b) => b.y)) : Math.min(...boxes.map((b) => b.x)),
  )
  for (const b of boxes) {
    const lead = along.get(b.id)!
    positions.set(b.id, axis === 'x' ? { x: lead, y: cross } : { x: cross, y: lead })
  }
  return positions
}

/** Leading edge (top/left) of a box along an axis. */
function lead(box: Box, axis: 'x' | 'y'): number {
  return axis === 'x' ? box.x : box.y
}
/** Size of a box along an axis. */
function size(box: Box, axis: 'x' | 'y'): number {
  return axis === 'x' ? box.width : box.height
}

/**
 * Even the gaps between a set of leading edges along `axis`, pinning the first
 * item's leading edge and the last item's trailing edge at their current spots.
 *
 *   gap = max((extent − Σ sizes) / (n − 1), MIN_GAP)   — floored so a tight
 *                                                        footprint grows instead
 *                                                        of overlapping
 *
 * Returns each box's new leading edge along the axis, keyed by id.
 */
function evenCells(boxes: readonly Box[], axis: 'x' | 'y'): Map<string, number> {
  const cross = axis === 'x' ? 'y' : 'x'
  const sorted = [...boxes].sort((a, b) => lead(a, axis) - lead(b, axis))
  const first = sorted[0]
  const last = sorted[sorted.length - 1]
  const along = lead(last, axis) + size(last, axis) - lead(first, axis)
  // A collapsed cluster (e.g. a column re-arranged into a row) has ~0 extent on
  // this axis, so items would stack. Fall back to the bigger cross-axis extent
  // so the line inherits the spacing it had on the other axis.
  const crossExtent =
    Math.max(...boxes.map((b) => lead(b, cross) + size(b, cross))) -
    Math.min(...boxes.map((b) => lead(b, cross)))
  const extent = Math.max(along, crossExtent)
  const totalSize = sorted.reduce((s, b) => s + size(b, axis), 0)
  // Snap the *gap* (not each edge) to the grid: one shared gap keeps every pair
  // exactly equal, so the arranged line still detects as a reorderable row even
  // when items differ wildly in size. Snapping each edge instead perturbs each
  // gap by up to half a cell — invisible with equal sizes, but enough to break
  // row detection once sizes diverge.
  const gap = snapToGrid(Math.max((extent - totalSize) / (sorted.length - 1), MIN_GAP))

  const out = new Map<string, number>()
  let cursor = snapToGrid(lead(first, axis))
  for (const box of sorted) {
    out.set(box.id, cursor)
    cursor += size(box, axis) + gap
  }
  return out
}

/**
 * Grid axis: cluster boxes into bands (a "band" is a maximal run that overlaps
 * along `axis` — a row when axis='y', a column when axis='x'), even the gaps
 * between bands across the current extent, and snap every box in a band to its
 * band's leading edge. Preserves holes: an empty cell stays empty.
 *
 * ponytail: overlap-run clustering merges a diagonal staircase into one band and
 * splits touching-but-not-overlapping boxes. Upgrade to center-tolerance
 * clustering if ragged real-world grids band wrong.
 */
function evenBands(boxes: readonly Box[], axis: 'x' | 'y'): Map<string, number> {
  const sorted = [...boxes].sort((a, b) => lead(a, axis) - lead(b, axis))
  const bands: Box[][] = []
  let bandMaxTrailing = -Infinity
  for (const box of sorted) {
    if (lead(box, axis) >= bandMaxTrailing) {
      bands.push([box])
      bandMaxTrailing = lead(box, axis) + size(box, axis)
    } else {
      bands[bands.length - 1].push(box)
      bandMaxTrailing = Math.max(bandMaxTrailing, lead(box, axis) + size(box, axis))
    }
  }

  const out = new Map<string, number>()
  if (bands.length < 2) {
    // One band on this axis — nothing to distribute; just snap in place.
    for (const box of boxes) out.set(box.id, snapToGrid(lead(box, axis)))
    return out
  }

  // Distribute the bands as cells: each band's lead = its min edge, size = its
  // max extent along the axis.
  const bandLeads = bands.map((band) => Math.min(...band.map((b) => lead(b, axis))))
  const bandSizes = bands.map((band, i) =>
    Math.max(...band.map((b) => lead(b, axis) + size(b, axis))) - bandLeads[i],
  )
  const extent = bandLeads[bands.length - 1] + bandSizes[bands.length - 1] - bandLeads[0]
  const totalSize = bandSizes.reduce((s, x) => s + x, 0)
  const gap = Math.max((extent - totalSize) / (bands.length - 1), MIN_GAP)

  let cursor = bandLeads[0]
  for (let i = 0; i < bands.length; i++) {
    const snapped = snapToGrid(cursor)
    for (const box of bands[i]) out.set(box.id, snapped)
    cursor = snapped + bandSizes[i] + gap
  }
  return out
}
