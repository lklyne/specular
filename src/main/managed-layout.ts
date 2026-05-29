import type { WorkspaceGroup } from '../shared/types'
import { CLUSTER_HORIZONTAL_GUTTER, USER_GROUP_PADDING } from '../shared/constants'
import { computeRowReflow, type LayoutBox } from './layout-math'
import { pages } from './runtime/page-runtime'
import { textEntities } from './runtime/text-entity-state'
import { fileEntities } from './runtime/file-entity-state'
import { shapeEntities } from './runtime/shape-entity-state'
import { drawingEntities } from './runtime/drawing-entity-state'
import { workspaceGroups } from './runtime/workspace-model'
import { pageContentSize, requestLayout, snapToGrid } from './runtime/surface-layout'
import { markDirty } from './runtime/layout-dirty'
import { managedChildOrder, writeManagedChildOrder } from './runtime/entity-order-state'
import { commitAsOneTransaction } from './runtime/workspace-observers'
import { scheduleWorkspaceAutosave } from './runtime/workspace-session'
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

/**
 * The single writer of a managed group's child positions (ADR 0015 D3). Resolves
 * the group's direct children in `entityOrder` run order, packs them as a row,
 * writes each origin, and recomputes the group bbox. Any change to a managed
 * group — membership, child resize, reorder — funnels through here. Children
 * never hold authoritative positions; these are outputs.
 *
 * No-op for `freeform` / unmanaged groups. Does not call `requestLayout` — the
 * caller owns the layout pass and undo batching.
 */
export function reflowManagedGroup(groupId: string): boolean {
  const group = groupById(groupId)
  if (!group || !group.managedLayout) return false
  if (group.layoutMode !== 'row') return false // only row is live in Milestone 1

  const orderedIds = managedChildOrder(groupId)
  if (!orderedIds.length) return false

  const children = orderedIds
    .map(resolveManagedChild)
    .filter((c): c is ManagedChild => c !== null)
  if (!children.length) return false

  const originX = snapToGrid(Math.min(...children.map((c) => c.canvasX)))
  const originY = snapToGrid(Math.min(...children.map((c) => c.canvasY)))

  const positions = computeRowReflow(children, CLUSTER_HORIZONTAL_GUTTER, originX, originY)
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
 * The managed-row group id that directly contains `childId`, or null. Used by the
 * reorder gesture's door resolution (ADR 0015 D7): a dragged dot whose entity is
 * a managed-row child takes the managed door; everything else takes the selection
 * door.
 */
export function managedRowGroupForChild(childId: string): string | null {
  const parentId = resolveLeafParentGroupId(childId)
  if (!parentId) return null
  const group = groupById(parentId)
  if (!group || !group.managedLayout || group.layoutMode !== 'row') return null
  return parentId
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
 * with the cursor at `cursorCanvasX`. Counts how many *other* children have their
 * center left of the cursor. Returns an index into the without-dragged sequence
 * (0..n-1), directly consumable by `reorderManagedChild`.
 */
export function computeReorderDropIndex(
  groupId: string,
  childId: string,
  cursorCanvasX: number,
): number {
  const others = managedChildOrder(groupId).filter((id) => id !== childId)
  let index = 0
  for (const id of others) {
    const child = resolveManagedChild(id)
    if (!child) continue
    if (cursorCanvasX > child.canvasX + child.width / 2) index++
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

  let changed = false
  commitAsOneTransaction(() => {
    changed = writeManagedChildOrder(groupId, next)
    if (changed) reflowManagedGroup(groupId)
  })
  if (changed) requestLayout()
  return changed
}

/**
 * Headless entry point for "make auto-layout from selection" (plan O1). Marks a
 * group as a managed row — creating one from `entityIds` if no `groupId` is
 * given — seeds the layout sequence to the children's current left-to-right
 * order so nothing jumps, and reflows. One undo step.
 *
 * Returns the managed group, or null if there's nothing to manage.
 */
export function makeAutoLayoutGroup(input: {
  groupId?: string
  entityIds?: string[]
  label?: string
}): WorkspaceGroup | null {
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

    group.layoutMode = 'row'
    group.managedLayout = true
    markDirty('canvas', 'sidebar')

    // Seed layout order = current visual left-to-right so the row doesn't
    // scramble on conversion.
    const seeded = managedChildOrder(group.id)
      .map((id) => ({ id, x: resolveManagedChild(id)?.canvasX ?? 0 }))
      .sort((a, b) => a.x - b.x)
      .map((c) => c.id)
    writeManagedChildOrder(group.id, seeded)
    reflowManagedGroup(group.id)
    result = group
  })
  if (result) {
    scheduleWorkspaceAutosave()
    requestLayout()
  }
  return result
}
