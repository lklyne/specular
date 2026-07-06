/**
 * Shared gap-handle selector (ADR 0015 Milestone 2 — draggable gap handles).
 *
 * The single source of truth for *where a managed group's gap strips sit* —
 * consumed by both the hit-tester (`collectGapHandleTargets`) and the renderer
 * painter (`GapHandlesLayer`), so the visible affordance and the grabbable
 * target line up by construction (same pattern as `reorderable-dots.ts`).
 *
 * A zone is the strip between two adjacent children of a managed row/column
 * group: it spans the gap along the packing axis and the full cross-axis
 * extent of the line. Managed groups only — loose equal-gap selections keep
 * distribute; there is no persisted gap to edit there.
 *
 * Eligibility mirrors the reorder dots' managed door: strips light up when the
 * group itself or any of its children is selected. Pure: no Electron, no DOM.
 */

import { GAP_HANDLE_MIN_HIT_PX } from './canvas-hit-geometry'
import type { Rect } from './hit-regions'
import type { CanvasSceneEntity } from './types'

export interface GapHandleZone {
  groupId: string
  /** The group's packing axis — the drag projects onto this axis. */
  axis: 'x' | 'y'
  /** Gap index: the strip between sorted children `index` and `index + 1`. */
  index: number
  /** Screen-space hit/paint strip spanning the gap, full cross-axis extent. */
  rect: Rect
}

export interface GapHandleInput {
  entities: readonly CanvasSceneEntity[]
  selectedEntityIds: readonly string[]
  selectedGroupId?: string | null
}

interface ScreenBox {
  id: string
  leading: number
  trailing: number
  crossLeading: number
  crossTrailing: number
}

function screenBoxAlong(entity: CanvasSceneEntity, axis: 'x' | 'y'): ScreenBox {
  const along = axis === 'x'
  return {
    id: entity.id,
    leading: along ? entity.screenX : entity.screenY,
    trailing: along
      ? entity.screenX + entity.screenWidth
      : entity.screenY + entity.screenHeight,
    crossLeading: along ? entity.screenY : entity.screenX,
    crossTrailing: along
      ? entity.screenY + entity.screenHeight
      : entity.screenX + entity.screenWidth,
  }
}

/**
 * The gap strips of every eligible managed row/column group. Sorted by the
 * children's current geometry along the packing axis — a managed group is
 * already packed, so geometric order equals layout order.
 */
export function collectGapHandleZones(input: GapHandleInput): GapHandleZone[] {
  const { entities, selectedEntityIds, selectedGroupId } = input
  const selected = new Set(selectedEntityIds)
  const byId = new Map(entities.map((e) => [e.id, e]))
  const out: GapHandleZone[] = []

  for (const group of entities) {
    if (group.kind !== 'group') continue
    if (!group.managedLayout) continue
    if (group.layoutMode !== 'row' && group.layoutMode !== 'column') continue
    const eligible =
      selectedGroupId === group.id || group.entityIds.some((id) => selected.has(id))
    if (!eligible) continue

    const axis = group.layoutMode === 'column' ? 'y' : 'x'
    const children = group.entityIds
      .map((id) => byId.get(id))
      .filter((e): e is CanvasSceneEntity => e !== undefined)
      .map((e) => screenBoxAlong(e, axis))
      .sort((a, b) => a.leading - b.leading)
    if (children.length < 2) continue

    const crossStart = Math.min(...children.map((c) => c.crossLeading))
    const crossEnd = Math.max(...children.map((c) => c.crossTrailing))

    for (let i = 0; i < children.length - 1; i++) {
      let start = children[i].trailing
      let thickness = children[i + 1].leading - start
      if (thickness < GAP_HANDLE_MIN_HIT_PX) {
        const mid = start + thickness / 2
        start = mid - GAP_HANDLE_MIN_HIT_PX / 2
        thickness = GAP_HANDLE_MIN_HIT_PX
      }
      out.push({
        groupId: group.id,
        axis,
        index: i,
        rect:
          axis === 'x'
            ? { x: start, y: crossStart, width: thickness, height: crossEnd - crossStart }
            : { x: crossStart, y: start, width: crossEnd - crossStart, height: thickness },
      })
    }
  }
  return out
}

/**
 * Pure preview kernel for the gap drag: repack `children` along `axis` at
 * `gap`, anchored at the first child's current leading edge; every child keeps
 * its cross-axis coordinate. Returns only positions that changed — the
 * renderer-side mirror of main's `computeRowReflow` (which owns the real
 * commit-time reflow; this never mutates anything).
 */
export function packedGapPositions(
  children: readonly Pick<CanvasSceneEntity, 'id' | 'canvasX' | 'canvasY' | 'width' | 'height'>[],
  axis: 'x' | 'y',
  gap: number,
): Map<string, { x: number; y: number }> {
  const along = axis === 'x'
  const sorted = [...children].sort((a, b) =>
    along ? a.canvasX - b.canvasX : a.canvasY - b.canvasY,
  )
  const changed = new Map<string, { x: number; y: number }>()
  if (!sorted.length) return changed
  let cursor = along ? sorted[0].canvasX : sorted[0].canvasY
  for (const child of sorted) {
    const next = along ? { x: cursor, y: child.canvasY } : { x: child.canvasX, y: cursor }
    if (next.x !== child.canvasX || next.y !== child.canvasY) changed.set(child.id, next)
    cursor += (along ? child.width : child.height) + gap
  }
  return changed
}
