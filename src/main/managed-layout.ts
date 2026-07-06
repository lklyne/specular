import type { WorkspaceGroup, WorkspaceGroupLayoutMode } from '../shared/types'
import { CLUSTER_HORIZONTAL_GUTTER, USER_GROUP_PADDING } from '../shared/constants'
import { dominantAxis, type Box } from '../shared/reorder-row'
import { computeRowReflow, type LayoutBox } from './layout-math'
import { pages } from './runtime/page-runtime'
import { textEntities } from './runtime/text-entity-state'
import { fileEntities } from './runtime/file-entity-state'
import { shapeEntities } from './runtime/shape-entity-state'
import { drawingEntities } from './runtime/drawing-entity-state'
import { workspaceGroups } from './runtime/workspace-model'
import { pageContentSize } from './runtime/runtime-geometry'
import { snapToGrid } from '../shared/gesture-utils'
import { markDirty } from './runtime/layout-dirty'
import { managedChildOrder, writeManagedChildOrder } from './runtime/entity-order-state'
import { commitAsOneTransaction } from './runtime/workspace-observers'
import { mutateWorkspace } from './runtime/mutate-workspace'
import { createUserGroup } from './workspace-groups'
import {
  entityBoundsById,
  groupById,
  groupDescendantIds,
  pageSelectableBounds,
  unionBounds,
} from './workspace-entities'

/**
 * A managed group's child as the reflow engine sees it: a sized box plus a way
 * to write its new canvas origin. Pages report their content size (chrome is
 * excluded from the packing axis); sub-groups translate their whole subtree.
 */
interface ManagedChild extends LayoutBox {
  id: string
  /** Current top-left used as the packing origin source (min across children). */
  canvasX: number
  canvasY: number
  setOrigin: (x: number, y: number) => void
}

function resolveManagedChild(id: string): ManagedChild | null {
  const page = pages.find((p) => p.id === id)
  if (page) {
    const size = pageContentSize(page)
    return {
      id,
      width: size.width,
      height: size.height,
      canvasX: page.canvasX,
      canvasY: page.canvasY,
      setOrigin: (x, y) => {
        page.canvasX = x
        page.canvasY = y
      },
    }
  }
  for (const arr of [textEntities, fileEntities, shapeEntities, drawingEntities]) {
    const entity = (arr as Array<{ id: string; canvasX: number; canvasY: number; width: number; height: number }>).find(
      (e) => e.id === id,
    )
    if (entity) {
      return {
        id,
        width: entity.width,
        height: entity.height,
        canvasX: entity.canvasX,
        canvasY: entity.canvasY,
        setOrigin: (x, y) => {
          entity.canvasX = x
          entity.canvasY = y
        },
      }
    }
  }
  const group = workspaceGroups.find((g) => g.id === id)
  if (group) {
    return {
      id,
      width: group.width,
      height: group.height,
      canvasX: group.canvasX,
      canvasY: group.canvasY,
      // Translate the whole subtree so a nested group moves as a unit.
      setOrigin: (x, y) => translateGroupSubtree(group, x - group.canvasX, y - group.canvasY),
    }
  }
  return null
}

function translateGroupSubtree(group: WorkspaceGroup, dx: number, dy: number): void {
  if (dx === 0 && dy === 0) return
  group.canvasX += dx
  group.canvasY += dy
  for (const descendantId of groupDescendantIds(group.id)) {
    const child = resolveLeafForTranslate(descendantId)
    if (child) {
      child.canvasX += dx
      child.canvasY += dy
    }
  }
}

function resolveLeafForTranslate(
  id: string,
): { canvasX: number; canvasY: number } | null {
  return (
    pages.find((p) => p.id === id) ??
    textEntities.find((e) => e.id === id) ??
    fileEntities.find((e) => e.id === id) ??
    shapeEntities.find((e) => e.id === id) ??
    drawingEntities.find((e) => e.id === id) ??
    workspaceGroups.find((g) => g.id === id) ??
    null
  )
}

function recomputeGroupBounds(group: WorkspaceGroup, childIds: string[]): void {
  const bounds = unionBounds(
    childIds
      .map((id) => {
        const page = pages.find((p) => p.id === id)
        return page ? pageSelectableBounds(page) : entityBoundsById(id)
      })
      .filter((b): b is NonNullable<typeof b> => b !== null),
  )
  if (!bounds) return
  group.canvasX = bounds.x - USER_GROUP_PADDING
  group.canvasY = bounds.y - USER_GROUP_PADDING
  group.width = bounds.width + USER_GROUP_PADDING * 2
  group.height = bounds.height + USER_GROUP_PADDING * 2
}

/** Packing axis for a managed layout mode: columns stack vertically, everything
 *  else packs horizontally (grid isn't live yet). */
export function managedAxis(mode: WorkspaceGroupLayoutMode): 'x' | 'y' {
  return mode === 'column' ? 'y' : 'x'
}

function isManagedLineMode(mode: WorkspaceGroupLayoutMode): boolean {
  return mode === 'row' || mode === 'column'
}

/**
 * The single writer of a managed group's child positions (ADR 0015 D3). Resolves
 * the group's direct children in `entityOrder` run order, packs them as a line
 * along the mode's axis, writes each origin, and recomputes the group bbox. Any
 * change to a managed group — membership, child resize, reorder — funnels
 * through here. Children never hold authoritative positions; these are outputs.
 *
 * No-op for `freeform` / unmanaged groups. Does not call `requestLayout` — the
 * caller owns the layout pass and undo batching.
 */
export function reflowManagedGroup(groupId: string): boolean {
  const group = groupById(groupId)
  if (!group || !group.managedLayout) return false
  if (!isManagedLineMode(group.layoutMode)) return false // grid isn't live yet

  const orderedIds = managedChildOrder(groupId)
  if (!orderedIds.length) return false

  const children = orderedIds
    .map(resolveManagedChild)
    .filter((c): c is ManagedChild => c !== null)
  if (!children.length) return false

  const originX = snapToGrid(Math.min(...children.map((c) => c.canvasX)))
  const originY = snapToGrid(Math.min(...children.map((c) => c.canvasY)))

  const gap = group.layoutGap ?? CLUSTER_HORIZONTAL_GUTTER
  const positions = computeRowReflow(children, gap, originX, originY, managedAxis(group.layoutMode))
  children.forEach((child, index) => {
    const pos = positions[index]
    child.setOrigin(pos.canvasX, pos.canvasY)
  })

  recomputeGroupBounds(group, orderedIds)
  markDirty('canvas', 'sidebar')
  return true
}

/**
 * Reflow the managed group that directly contains `childId`, if any. Used by
 * gesture commits (resize) that know the child but not whether its group is
 * managed.
 */
export function reflowManagedGroupForChild(childId: string): boolean {
  const parentId = resolveLeafParentGroupId(childId)
  if (!parentId) return false
  return reflowManagedGroup(parentId)
}

/**
 * The managed row/column group that directly contains `childId` (with its
 * packing axis), or null. Used by the reorder gesture's door resolution
 * (ADR 0015 D7): a dragged dot whose entity is a managed child takes the
 * managed door; everything else takes the selection door.
 */
export function managedGroupForChild(
  childId: string,
): { groupId: string; axis: 'x' | 'y' } | null {
  const parentId = resolveLeafParentGroupId(childId)
  if (!parentId) return null
  const group = groupById(parentId)
  if (!group || !group.managedLayout || !isManagedLineMode(group.layoutMode)) return null
  return { groupId: parentId, axis: managedAxis(group.layoutMode) }
}

function resolveLeafParentGroupId(id: string): string | null {
  const page = pages.find((p) => p.id === id)
  if (page) return page.parentGroupId ?? null
  const entity =
    textEntities.find((e) => e.id === id) ??
    fileEntities.find((e) => e.id === id) ??
    shapeEntities.find((e) => e.id === id) ??
    drawingEntities.find((e) => e.id === id)
  if (entity) return entity.parentGroupId ?? null
  const group = workspaceGroups.find((g) => g.id === id)
  if (group) return group.parentGroupId ?? null
  return null
}

/**
 * Drop index for a reorder-in-progress: where `childId` would land if released
 * with the cursor at `cursorAlongAxis` (canvas-space, along the group's packing
 * axis). Counts how many *other* children have their center before the cursor.
 * Returns an index into the without-dragged sequence (0..n-1), directly
 * consumable by `reorderManagedChild`.
 */
export function computeReorderDropIndex(
  groupId: string,
  childId: string,
  cursorAlongAxis: number,
): number {
  const group = groupById(groupId)
  const axis = group ? managedAxis(group.layoutMode) : 'x'
  const others = managedChildOrder(groupId).filter((id) => id !== childId)
  let index = 0
  for (const id of others) {
    const child = resolveManagedChild(id)
    if (!child) continue
    const center =
      axis === 'y' ? child.canvasY + child.height / 2 : child.canvasX + child.width / 2
    if (cursorAlongAxis > center) index++
  }
  return index
}

function clampIndex(index: number, length: number): number {
  if (length <= 0) return 0
  return Math.max(0, Math.min(index, length - 1))
}

/**
 * Move `childId` to `toIndex` within its managed group's layout sequence, then
 * reflow. The order rewrite and the position reflow run inside one transaction
 * so the whole reorder is a single undo step (ADR 0015 undo batching). Returns
 * true if the order changed. No-op when the group is unmanaged, the child isn't
 * a direct child, or the move is a no-op.
 */
export function reorderManagedChild(
  groupId: string,
  childId: string,
  toIndex: number,
): boolean {
  const group = groupById(groupId)
  if (!group || !group.managedLayout) return false

  const order = managedChildOrder(groupId)
  const from = order.indexOf(childId)
  if (from === -1) return false

  const clampedTo = clampIndex(toIndex, order.length)
  if (from === clampedTo) return false

  const next = order.filter((id) => id !== childId)
  next.splice(clampedTo, 0, childId)

  return mutateWorkspace(() => {
    let changed = false
    commitAsOneTransaction(() => {
      changed = writeManagedChildOrder(groupId, next)
      if (changed) reflowManagedGroup(groupId)
    })
    return changed
  }, { changed: (changed) => changed })
}

/** Clamp a requested packing gap to a usable value (non-negative whole px), or
 *  null when it isn't a finite number. */
function normalizeLayoutGap(gap: number): number | null {
  if (!Number.isFinite(gap)) return null
  return Math.max(0, Math.round(gap))
}

/**
 * Set a managed group's packing gap (px) and reflow at the new spacing. The gap
 * is clamped to a non-negative integer. One undo step (the field write and the
 * reflow positions land in the same forward-sync transaction). Returns whether
 * anything changed.
 */
export function setGroupLayoutGap(groupId: string, gap: number): boolean {
  const group = groupById(groupId)
  if (!group || !group.managedLayout) return false
  const next = normalizeLayoutGap(gap)
  if (next === null || group.layoutGap === next) return false
  return mutateWorkspace(() => {
    group.layoutGap = next
    markDirty('canvas', 'sidebar')
    reflowManagedGroup(groupId)
    return true
  }, { changed: (changed) => changed })
}

/**
 * Headless entry point for "make auto-layout from selection" (plan O1). Marks a
 * group as a managed row or column — creating one from `entityIds` if no
 * `groupId` is given — picking the mode from the children's dominant axis,
 * seeds the layout sequence to their current order along that axis so nothing
 * jumps, and reflows. One undo step.
 *
 * Returns the managed group, or null if there's nothing to manage.
 */
export function makeAutoLayoutGroup(input: {
  groupId?: string
  entityIds?: string[]
  label?: string
  /** Packing gap (px); validated like `setGroupLayoutGap` — invalid values are ignored. */
  gap?: number
}): WorkspaceGroup | null {
  return mutateWorkspace(() => {
    let result: WorkspaceGroup | null = null
    commitAsOneTransaction(() => {
      let group: WorkspaceGroup | undefined
      if (input.groupId) {
        group = groupById(input.groupId)
      } else if (input.entityIds && input.entityIds.length === 1 && groupById(input.entityIds[0])) {
        group = groupById(input.entityIds[0])
      } else if (input.entityIds && input.entityIds.length >= 2) {
        group = createUserGroup(input.entityIds, input.label ?? 'Auto-layout')
      }
      if (!group) return

      const children = managedChildOrder(group.id).map((id) => ({
        id,
        child: resolveManagedChild(id),
      }))
      const boxes: Box[] = children.flatMap(({ id, child }) =>
        child
          ? [{ id, x: child.canvasX, y: child.canvasY, width: child.width, height: child.height }]
          : [],
      )
      const axis = boxes.length ? dominantAxis(boxes) : 'x'
      group.layoutMode = axis === 'y' ? 'column' : 'row'
      group.managedLayout = true
      if (input.gap !== undefined) {
        const gap = normalizeLayoutGap(input.gap)
        if (gap !== null) group.layoutGap = gap
      }
      markDirty('canvas', 'sidebar')

      // Seed layout order = current visual order along the axis so the line
      // doesn't scramble on conversion.
      const seeded = children
        .map(({ id, child }) => ({
          id,
          pos: (axis === 'y' ? child?.canvasY : child?.canvasX) ?? 0,
        }))
        .sort((a, b) => a.pos - b.pos)
        .map((c) => c.id)
      writeManagedChildOrder(group.id, seeded)
      reflowManagedGroup(group.id)
      result = group
    })
    return result
  }, { changed: (group) => group !== null })
}
