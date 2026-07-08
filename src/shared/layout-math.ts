import type { WorkspaceGroupLayoutMode } from './types'

export interface LayoutBox { width: number; height: number }

/**
 * The packing axis of a managed *line* layout (row → 'x', column → 'y'), or
 * null for modes that aren't a single packed line (freeform, grid). The single
 * source of truth for "is this a managed line, and which way does it pack?" —
 * callers wanting a boolean predicate test `!== null`.
 */
export function managedLineAxis(mode: WorkspaceGroupLayoutMode): 'x' | 'y' | null {
  if (mode === 'row') return 'x'
  if (mode === 'column') return 'y'
  return null
}

/**
 * Pack boxes into a single line along `axis` from an explicit origin, separated
 * by a fixed gap. The managed-layout kernel: each child keeps its own size, the
 * cursor advances by the child's main-axis size + gap. All children share the
 * line's cross-axis origin; cross-axis alignment is a Milestone 2 concern.
 * Pure — no grid-snap (the caller snaps the origin; see managed-layout reflow,
 * ADR 0015 D5). Shared so the renderer preview and the main commit pack through
 * one function and cannot drift (`packedGapPositions` wraps this).
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
