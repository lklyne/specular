/**
 * Live gap-resize preview (ADR 0015 Milestone 2).
 *
 * While a `resizing-gap` drag is in flight, repack the managed group's
 * children at the live broadcast gap so spacing tracks the cursor. Pure
 * renderer ephemera: no IPC, no doc mutation — main only updates the
 * interaction state's `gap` per tick, and the single real reflow happens at
 * commit via `setGroupLayoutGap`. The packing kernel (`packedGapPositions`)
 * lives in `src/shared/gap-handles.ts`, mirroring main's `computeRowReflow`
 * anchored at the first child's current position.
 *
 * A nested-group child translates with its whole subtree (matching
 * `reflowManagedGroup`'s subtree translation), and the managed group's own box
 * stretches along the packing axis by the change in total extent.
 */

import { packedGapPositions } from '../../shared/gap-handles'
import type { CanvasSceneEntity, CanvasSceneGroupEntity, LayoutUpdateData } from '../../shared/types'

/**
 * A layout clone with the group's children moved to their previewed slots, or
 * null when not gap-resizing (or nothing shifts). Callers fall back to the
 * broadcast layout on null.
 */
export function gapPreviewLayout(layoutData: LayoutUpdateData): LayoutUpdateData | null {
  const { interaction, zoom } = layoutData
  if (interaction.kind !== 'resizing-gap') return null

  const group = layoutData.entities.find(
    (e): e is CanvasSceneGroupEntity => e.kind === 'group' && e.id === interaction.groupId,
  )
  if (!group) return null

  const byId = new Map(layoutData.entities.map((e) => [e.id, e]))
  const children = group.entityIds
    .map((id) => byId.get(id))
    .filter((e): e is CanvasSceneEntity => e !== undefined)
  if (children.length < 2) return null

  const changed = packedGapPositions(children, interaction.axis, interaction.gap)
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

  // The managed group's box stretches by the change in the children's extent.
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
    if (e.id === group.id && e.kind === 'group' && extentDelta !== 0) {
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
