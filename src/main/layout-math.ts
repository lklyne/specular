import type { BatchLayoutMode } from '../shared/types'
import type { LayoutBox } from '../shared/layout-math'

export type { LayoutBox }

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
