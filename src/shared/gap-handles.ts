/**
 * Shared gap-handle selector (ADR 0015 Milestone 2 — draggable gap handles).
 *
 * The single source of truth for *where gap strips sit* — consumed by both the
 * hit-tester (`collectGapHandleTargets`) and the renderer painter
 * (`GapHandlesLayer`), so the visible affordance and the grabbable target line
 * up by construction (same pattern as `reorderable-dots.ts`).
 *
 * A zone is the strip between two adjacent items of a line: it spans the gap
 * along the packing axis and the full cross-axis extent. Two doors, mirroring
 * the reorder dots:
 *   - **Managed door** (`groupId` set): a managed row/column group whose group
 *     or child is selected gets a strip per gap; the drag edits the group's
 *     persisted `layoutGap`.
 *   - **Selection door** (`groupId` null): a loose equal-gap multi-selection
 *     (`detectReorderableRow` ≠ null) gets a strip per gap; the drag just
 *     moves the items — nothing persists but the new positions.
 *
 * Pure: no Electron, no DOM.
 */

import { GAP_HANDLE_MIN_HIT_PX } from './canvas-hit-geometry'
import type { Rect } from './hit-regions'
import { computeRowReflow, managedLineAxis } from './layout-math'
import { detectReorderableRow, SELECTION_ROW_GAP_TOLERANCE } from './reorder-row'
import { rowBox } from './reorderable-dots'
import type { CanvasSceneEntity } from './types'

export interface GapHandleZone {
  /** The managed group whose gap this edits, or null for a loose selection. */
  groupId: string | null
  /** The line's packing axis — the drag projects onto this axis. */
  axis: 'x' | 'y'
  /** Gap index: the strip between sorted items `index` and `index + 1`. */
  index: number
  /** Screen-space hit/paint strip spanning the gap, full cross-axis extent. */
  rect: Rect
}

export interface GapHandleInput {
  entities: readonly CanvasSceneEntity[]
  selectedEntityIds: readonly string[]
  selectedGroupId?: string | null
  zoom?: number
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

/** Strips between adjacent `boxes` (sorted along the axis by the caller). */
function stripsBetween(
  groupId: string | null,
  axis: 'x' | 'y',
  boxes: readonly ScreenBox[],
): GapHandleZone[] {
  const out: GapHandleZone[] = []
  const crossStart = Math.min(...boxes.map((b) => b.crossLeading))
  const crossEnd = Math.max(...boxes.map((b) => b.crossTrailing))
  for (let i = 0; i < boxes.length - 1; i++) {
    let start = boxes[i].trailing
    let thickness = boxes[i + 1].leading - start
    if (thickness < GAP_HANDLE_MIN_HIT_PX) {
      const mid = start + thickness / 2
      start = mid - GAP_HANDLE_MIN_HIT_PX / 2
      thickness = GAP_HANDLE_MIN_HIT_PX
    }
    out.push({
      groupId,
      axis,
      index: i,
      rect:
        axis === 'x'
          ? { x: start, y: crossStart, width: thickness, height: crossEnd - crossStart }
          : { x: crossStart, y: start, width: crossEnd - crossStart, height: thickness },
    })
  }
  return out
}

/**
 * The gap strips of every eligible line — managed row/column groups first,
 * then the loose selection door. Managed groups sort children by current
 * geometry along the packing axis (a managed group is already packed, so
 * geometric order equals layout order).
 */
export function collectGapHandleZones(input: GapHandleInput): GapHandleZone[] {
  const { entities, selectedEntityIds, selectedGroupId, zoom = 1 } = input
  const selected = new Set(selectedEntityIds)
  const byId = new Map(entities.map((e) => [e.id, e]))
  const out: GapHandleZone[] = []

  // Managed door. Also collects managed children so the selection door below
  // never fights over them (mirrors `reorderableDots`).
  const managedChildIds = new Set<string>()
  for (const group of entities) {
    if (group.kind !== 'group') continue
    if (!group.managedLayout) continue
    const axis = managedLineAxis(group.layoutMode)
    if (axis === null) continue
    for (const id of group.entityIds) managedChildIds.add(id)
    const eligible =
      selectedGroupId === group.id || group.entityIds.some((id) => selected.has(id))
    if (!eligible) continue

    const children = group.entityIds
      .map((id) => byId.get(id))
      .filter((e): e is CanvasSceneEntity => e !== undefined)
      .map((e) => screenBoxAlong(e, axis))
      .sort((a, b) => a.leading - b.leading)
    if (children.length < 2) continue
    out.push(...stripsBetween(group.id, axis, children))
  }

  // Selection door: a loose equal-gap multi-selection. Eligibility runs on
  // canvas geometry (fixed gap tolerance), strips paint on screen geometry —
  // same split as the reorder dots.
  const loose = entities.filter(
    (e) => e.kind !== 'group' && selected.has(e.id) && !managedChildIds.has(e.id),
  )
  const row = detectReorderableRow(
    loose.map((e) => rowBox(e, zoom)),
    { gapTolerance: SELECTION_ROW_GAP_TOLERANCE },
  )
  if (row) {
    const looseById = new Map(loose.map((e) => [e.id, e]))
    const boxes = row.order
      .map((id) => looseById.get(id))
      .filter((e): e is CanvasSceneEntity => e !== undefined)
      .map((e) => screenBoxAlong(e, row.axis))
    out.push(...stripsBetween(null, row.axis, boxes))
  }

  return out
}

/**
 * Preview repack for the gap drag: pack `children` along `axis` at `gap` through
 * the shared `computeRowReflow` kernel — the same function that owns the
 * commit-time reflow, so preview and commit cannot drift — anchored at the first
 * child's current leading edge. `keepCross` preserves each child's own
 * cross-axis coordinate (the selection door: loose items needn't be aligned);
 * without it the cross axis aligns to the first child, matching
 * `reflowManagedGroup`. Returns only the positions that changed (this never
 * mutates anything).
 */
export function packedGapPositions(
  children: readonly Pick<CanvasSceneEntity, 'id' | 'canvasX' | 'canvasY' | 'width' | 'height'>[],
  axis: 'x' | 'y',
  gap: number,
  opts?: { keepCross?: boolean },
): Map<string, { x: number; y: number }> {
  const along = axis === 'x'
  const sorted = [...children].sort((a, b) =>
    along ? a.canvasX - b.canvasX : a.canvasY - b.canvasY,
  )
  const changed = new Map<string, { x: number; y: number }>()
  if (!sorted.length) return changed
  const positions = computeRowReflow(sorted, gap, sorted[0].canvasX, sorted[0].canvasY, axis)
  sorted.forEach((child, i) => {
    const next = along
      ? { x: positions[i].canvasX, y: opts?.keepCross ? child.canvasY : positions[i].canvasY }
      : { x: opts?.keepCross ? child.canvasX : positions[i].canvasX, y: positions[i].canvasY }
    if (next.x !== child.canvasX || next.y !== child.canvasY) changed.set(child.id, next)
  })
  return changed
}
