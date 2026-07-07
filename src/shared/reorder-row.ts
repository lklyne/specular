/**
 * Reorderable-row kernel — the pure eligibility brain + repacker behind
 * selection reorder (ADR 0015 D7). No Electron, no DOM: unit-tested in isolation
 * like `computeRowReflow`. Shared so the hit-tester, the renderer painter, and
 * the main-side commit all consume *one* definition of "is this a row, and where
 * do the slots sit" — never a copy-pasted predicate.
 *
 * Geometry is the source of truth: an evenly-spaced multi-selection *is* a row,
 * and the order is read off the boxes per gesture. Nothing here persists.
 */

export interface Box {
  id: string
  x: number
  y: number
  width: number
  height: number
}

/**
 * Gap-equality slack (canvas px) for the loose-selection reorder door. Wider
 * than the 1px default so an arranged row still reads as even after the small
 * float/rounding drift a page's shell size picks up round-tripping through
 * screen bounds — the dots stay put instead of flickering off. The renderer's
 * `reorderableDots` and main's `buildSelectionRow` share this so the visible
 * dot and the commit agree on what counts as a row.
 */
export const SELECTION_ROW_GAP_TOLERANCE = 4

export interface ReorderableRow {
  /** Dominant axis — the one with the larger center spread. */
  axis: 'x' | 'y'
  /** Ids sorted along the axis by leading edge. */
  order: string[]
  /** The common gap (average of the equal gaps) between consecutive items. */
  gap: number
  /** Min corner of the selection — the packing start. */
  origin: { x: number; y: number }
  /** Sizes and positions frozen at detect time, keyed by id. */
  boxesById: Map<string, Box>
}

/** Leading edge (top/left) of a box along the given axis. */
function leadingEdge(box: Box, axis: 'x' | 'y'): number {
  return axis === 'x' ? box.x : box.y
}

/** Size of a box along the given axis. */
function sizeAlong(box: Box, axis: 'x' | 'y'): number {
  return axis === 'x' ? box.width : box.height
}

/** Center of a box along the given axis. */
function centerAlong(box: Box, axis: 'x' | 'y'): number {
  return leadingEdge(box, axis) + sizeAlong(box, axis) / 2
}

function spread(values: number[]): number {
  return Math.max(...values) - Math.min(...values)
}

/**
 * Dominant axis: whichever axis has the larger spread of box centers. Used by
 * `detectReorderableRow` to pick the row's axis.
 */
function dominantAxis(boxes: readonly Box[]): 'x' | 'y' {
  const centersX = boxes.map((b) => centerAlong(b, 'x'))
  const centersY = boxes.map((b) => centerAlong(b, 'y'))
  return spread(centersX) >= spread(centersY) ? 'x' : 'y'
}

/**
 * Equal-gap row detector + eligibility gate. Returns null when the selection is
 * not a clean, evenly-spaced, non-overlapping line — that's also the gate for
 * the dots (no well-defined slots ⇒ no affordance). FigJam parity: equal gaps
 * show dots, unequal spacing hides them.
 */
export function detectReorderableRow(
  boxes: readonly Box[],
  opts?: { gapTolerance?: number },
): ReorderableRow | null {
  if (boxes.length < 2) return null

  const axis = dominantAxis(boxes)

  const sorted = [...boxes].sort((a, b) => leadingEdge(a, axis) - leadingEdge(b, axis))

  const gaps: number[] = []
  for (let i = 0; i < sorted.length - 1; i++) {
    const cur = sorted[i]
    const next = sorted[i + 1]
    const gap = leadingEdge(next, axis) - (leadingEdge(cur, axis) + sizeAlong(cur, axis))
    if (gap < 0) return null // overlap — not a row
    gaps.push(gap)
  }

  const tolerance = opts?.gapTolerance ?? 1
  if (spread(gaps) > tolerance) return null // unequal spacing — no dots

  const avgGap = gaps.reduce((s, g) => s + g, 0) / gaps.length
  const boxesById = new Map<string, Box>()
  for (const b of boxes) boxesById.set(b.id, b)

  return {
    axis,
    order: sorted.map((b) => b.id),
    gap: avgGap,
    origin: {
      x: Math.min(...boxes.map((b) => b.x)),
      y: Math.min(...boxes.map((b) => b.y)),
    },
    boxesById,
  }
}

/**
 * Drop index for a cursor position along the row's axis: how many *other* items
 * have their center before the cursor. Generalizes M1's `computeReorderDropIndex`
 * off a `groupId` onto a frozen box list. Returns an index into the
 * without-moving sequence (0..n-1), consumable directly by `reorderRowPositions`.
 */
export function dropIndexForCursor(
  row: ReorderableRow,
  cursorAlongAxis: number,
  movingId: string,
): number {
  let index = 0
  for (const id of row.order) {
    if (id === movingId) continue
    const box = row.boxesById.get(id)
    if (!box) continue
    if (cursorAlongAxis > centerAlong(box, row.axis)) index++
  }
  return index
}

/**
 * Repack the row with `movingId` moved to `dropIndex`. Packs along `axis` from
 * `origin` by the frozen `gap`; each item KEEPS its own cross-axis coordinate
 * (a box that sat lower stays lower — Q1). Returns only the positions that
 * actually changed.
 */
export function reorderRowPositions(
  row: ReorderableRow,
  movingId: string,
  dropIndex: number,
): Map<string, { x: number; y: number }> {
  const others = row.order.filter((id) => id !== movingId)
  const clamped = Math.max(0, Math.min(dropIndex, others.length))
  const nextOrder = [...others.slice(0, clamped), movingId, ...others.slice(clamped)]

  const changed = new Map<string, { x: number; y: number }>()
  let cursor = row.axis === 'x' ? row.origin.x : row.origin.y
  for (const id of nextOrder) {
    const box = row.boxesById.get(id)
    if (!box) continue
    const next =
      row.axis === 'x' ? { x: cursor, y: box.y } : { x: box.x, y: cursor }
    if (next.x !== box.x || next.y !== box.y) changed.set(id, next)
    cursor += sizeAlong(box, row.axis) + row.gap
  }
  return changed
}
