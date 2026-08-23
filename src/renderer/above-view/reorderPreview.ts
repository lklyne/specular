/**
 * Live reorder preview (ADR 0015 D7, Phase D).
 *
 * While a `reordering-row` drag is in flight, repack the frozen row at the live
 * drop index so the siblings open a gap to receive the dragged item — the swap
 * fires at the neighbour midpoint (50% crossing), the difference between
 * "elegant" and "mechanical." The dragged item keeps its packed slot here, but
 * App skips its in-row paint and floats it as a 50%-opacity ghost under the
 * cursor (so the reserved slot reads as the open gap). Pure renderer ephemera:
 * no IPC, no doc mutation. Release commits to exactly this arrangement via
 * `reorderSelection`.
 *
 * The row is rebuilt from the *current* selection geometry, which is identical
 * to the gesture-start freeze because the drag mutates nothing — `move` only
 * advances the broadcast drop index. So we reuse the one shared kernel
 * (`detectReorderableRow` + `reorderRowPositions`) the hit-tester and the
 * main-side commit already consume, never a re-derived packer.
 */

import { detectReorderableRow, reorderRowPositions, type Box } from '../../shared/reorder-row'
import type { CanvasSceneEntity, LayoutUpdateData } from '../../shared/types'
import { reprojectEntity } from '../shared/scene-projection'

/**
 * A layout clone with each row entity moved to its previewed slot, or null when
 * not reordering (or the cursor sits at the row's resting order, so nothing
 * shifts yet). Callers fall back to the broadcast layout on null.
 */
export function reorderPreviewLayout(layoutData: LayoutUpdateData): LayoutUpdateData | null {
  const { interaction } = layoutData
  if (interaction.kind !== 'reordering-row') return null

  const rowIds = new Set(interaction.ids)
  const boxes: Box[] = []
  for (const e of layoutData.entities) {
    if (e.kind === 'group') continue
    if (!rowIds.has(e.id)) continue
    boxes.push({ id: e.id, x: e.canvasX, y: e.canvasY, width: e.width, height: e.height })
  }

  const row = detectReorderableRow(boxes)
  if (!row) return null

  const changed = reorderRowPositions(row, interaction.movingId, Math.max(0, interaction.dropIndex))
  if (changed.size === 0) return null

  const entities = layoutData.entities.map((e): CanvasSceneEntity => {
    const next = changed.get(e.id)
    if (!next || e.kind === 'group') return e
    // Reorder only permutes along the dominant axis, but apply both deltas so
    // the helper stays axis-agnostic (Q2 column support is near-free).
    return reprojectEntity({ ...e, canvasX: next.x, canvasY: next.y }, layoutData)
  })

  return { ...layoutData, entities }
}
