/**
 * Live gap-resize preview (ADR 0015 Milestone 2).
 *
 * While a `resizing-gap` drag is in flight, repack the line's items at the
 * live broadcast gap so spacing tracks the cursor. Pure renderer ephemera: no
 * IPC, no doc mutation — main only updates the interaction state's `gap` per
 * tick, and the single real write happens at commit (`setGroupLayoutGap` for
 * a managed group, `applySelectionGap` for a loose selection). The packing
 * kernel (`packedGapPositions`) lives in `src/shared/gap-handles.ts`,
 * mirroring main's `computeRowReflow` anchored at the first item's current
 * position — a loose selection's items each keep their own cross-axis
 * coordinate, matching the commit.
 *
 * A nested-group child translates with its whole subtree (matching
 * `reflowManagedGroup`'s subtree translation), and a managed group's own box
 * stretches along the packing axis by the change in total extent. The stretch
 * is a delta on the broadcast box rather than a from-scratch union+padding
 * because main's `recomputeGroupBounds` unions *selectable* bounds
 * (`pageSelectableBounds` — shell insets, device frames) that scene rects
 * don't carry; anchoring on main's own box keeps the preview exact at drag
 * start regardless of how those bounds are derived.
 */

import { packedGapPositions } from '../../shared/gap-handles'
import type { CanvasSceneEntity, CanvasSceneGroupEntity, LayoutUpdateData } from '../../shared/types'

/**
 * A layout clone with the line's items moved to their previewed slots, or
 * null when not gap-resizing (or nothing shifts). Callers fall back to the
 * broadcast layout on null.
 */
export function gapPreviewLayout(layoutData: LayoutUpdateData): LayoutUpdateData | null {
  const { interaction, zoom } = layoutData
  if (interaction.kind !== 'resizing-gap') return null

  const group =
    interaction.groupId === null
      ? null
      : (layoutData.entities.find(
          (e): e is CanvasSceneGroupEntity => e.kind === 'group' && e.id === interaction.groupId,
        ) ?? null)
  if (interaction.groupId !== null && !group) return null

  const byId = new Map(layoutData.entities.map((e) => [e.id, e]))
  const children = interaction.entityIds
    .map((id) => byId.get(id))
    .filter((e): e is CanvasSceneEntity => e !== undefined)
  if (children.length < 2) return null

  const changed = packedGapPositions(children, interaction.axis, interaction.gap, {
    keepCross: group === null,
  })
  if (changed.size === 0) return null

  // Per-entity translation deltas. A moved nested-group child carries its
  // whole subtree along, like the real reflow's subtree translation.
  const deltas = new Map<string, { dx: number; dy: number }>()
  const translateSubtree = (id: string, dx: number, dy: number): void => {
    deltas.set(id, { dx, dy })
    const entity = byId.get(id)
    if (entity?.kind === 'group') {
      for (const childId of entity.entityIds) translateSubtree(childId, dx, dy)
    }
  }
  for (const child of children) {
    const next = changed.get(child.id)
    if (next) translateSubtree(child.id, next.x - child.canvasX, next.y - child.canvasY)
  }

  // A managed group's box stretches by the change in the children's extent.
  const along = interaction.axis === 'x'
  const end = (e: CanvasSceneEntity, dx: number, dy: number) =>
    along ? e.canvasX + dx + e.width : e.canvasY + dy + e.height
  const oldEnd = Math.max(...children.map((e) => end(e, 0, 0)))
  const newEnd = Math.max(
    ...children.map((e) => {
      const d = deltas.get(e.id)
      return end(e, d?.dx ?? 0, d?.dy ?? 0)
    }),
  )
  const extentDelta = newEnd - oldEnd

  const preview = <T extends CanvasSceneEntity>(e: T): T => {
    if (group !== null && e.id === group.id && e.kind === 'group' && extentDelta !== 0) {
      return along
        ? { ...e, width: e.width + extentDelta, screenWidth: e.screenWidth + extentDelta * zoom }
        : { ...e, height: e.height + extentDelta, screenHeight: e.screenHeight + extentDelta * zoom }
    }
    const d = deltas.get(e.id)
    if (!d || (d.dx === 0 && d.dy === 0)) return e
    return {
      ...e,
      canvasX: e.canvasX + d.dx,
      canvasY: e.canvasY + d.dy,
      screenX: e.screenX + d.dx * zoom,
      screenY: e.screenY + d.dy * zoom,
    }
  }

  return {
    ...layoutData,
    entities: layoutData.entities.map((e) => preview(e)),
    groups: layoutData.groups?.map((g) => preview(g)),
  }
}
