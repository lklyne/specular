import type { BatchLayoutMode } from '../shared/types'

export interface LayoutBox { width: number; height: number }

export interface LayoutMetrics {
  cols: number
  maxW: number
  maxH: number
  bbWidth: number
  bbHeight: number
}

export function computeLayoutMetrics(
  items: LayoutBox[],
  kind: BatchLayoutMode,
  colGap: number,
  rowGap: number,
  cols?: number,
): LayoutMetrics {
  if (items.length === 0) return { cols: 0, maxW: 0, maxH: 0, bbWidth: 0, bbHeight: 0 }
  if (kind === 'column') {
    return {
      cols: 1,
      maxW: 0,
      maxH: 0,
      bbWidth: Math.max(...items.map((i) => i.width)),
      bbHeight: items.reduce((s, i) => s + i.height, 0) + (items.length - 1) * rowGap,
    }
  }
  if (kind === 'grid') {
    const gridCols = cols && cols > 0 ? cols : Math.ceil(Math.sqrt(items.length))
    const maxW = Math.max(...items.map((i) => i.width))
    const maxH = Math.max(...items.map((i) => i.height))
    const rows = Math.ceil(items.length / gridCols)
    return {
      cols: gridCols,
      maxW,
      maxH,
      bbWidth: gridCols * maxW + (gridCols - 1) * colGap,
      bbHeight: rows * maxH + (rows - 1) * rowGap,
    }
  }
  return {
    cols: 0,
    maxW: 0,
    maxH: 0,
    bbWidth: items.reduce((s, i) => s + i.width, 0) + (items.length - 1) * colGap,
    bbHeight: Math.max(...items.map((i) => i.height)),
  }
}

/**
 * Pack boxes into a single line along `axis` from an explicit origin, separated
 * by a fixed gap. The managed-layout kernel: each child keeps its own size, the
 * cursor advances by the child's main-axis size + gap. All children share the
 * line's cross-axis origin; cross-axis alignment is a Milestone 2 concern.
 * Pure — no grid-snap (the caller snaps the origin; see managed-layout reflow,
 * ADR 0015 D5).
 */
export function computeRowReflow(
  children: LayoutBox[],
  gap: number,
  originX: number,
  originY: number,
  axis: 'x' | 'y' = 'x',
): Array<{ canvasX: number; canvasY: number }> {
  const positions: Array<{ canvasX: number; canvasY: number }> = []
  let cursor = axis === 'x' ? originX : originY
  for (const child of children) {
    positions.push(
      axis === 'x'
        ? { canvasX: cursor, canvasY: originY }
        : { canvasX: originX, canvasY: cursor },
    )
    cursor += (axis === 'x' ? child.width : child.height) + gap
  }
  return positions
}

/**
 * Place items at an explicit origin in row/column/grid. Grid uses uniform
 * tracks (each cell sized to the largest item's dim) so heterogeneous content
 * still aligns to a clean grid.
 */
export function computeLayoutPositions(
  items: LayoutBox[],
  kind: BatchLayoutMode,
  colGap: number,
  rowGap: number,
  origin: { x: number; y: number },
  cols?: number,
): Array<{ canvasX: number; canvasY: number }> {
  if (items.length === 0) return []
  const positions: Array<{ canvasX: number; canvasY: number }> = []

  if (kind === 'column') {
    let cursorY = origin.y
    for (const item of items) {
      positions.push({ canvasX: origin.x, canvasY: cursorY })
      cursorY += item.height + rowGap
    }
    return positions
  }
  if (kind === 'grid') {
    const m = computeLayoutMetrics(items, 'grid', colGap, rowGap, cols)
    for (let idx = 0; idx < items.length; idx++) {
      positions.push({
        canvasX: origin.x + (idx % m.cols) * (m.maxW + colGap),
        canvasY: origin.y + Math.floor(idx / m.cols) * (m.maxH + rowGap),
      })
    }
    return positions
  }
  let cursorX = origin.x
  for (const item of items) {
    positions.push({ canvasX: cursorX, canvasY: origin.y })
    cursorX += item.width + colGap
  }
  return positions
}
