/**
 * One resolver for "what does this selection mean for my operation."
 *
 * Selection *construction* already funnels through one resolver
 * (`resolveMarqueeSelectionIds`); selection *consumption* — drag, resize,
 * delete, the bounding-box overlay — used to each hand-derive their own
 * partial answer, and disagreed at the edges (ADR 0034). This module is the
 * single declaration every consumer derives from instead.
 *
 * Pure derivation over runtime arrays — no Y.Doc access. Safe to call on
 * every gesture-begin and every layout broadcast.
 */

import { selectedEntityIds as uiSelectedEntityIds } from '../ui-state'
import type { WorkspaceBounds } from '../../shared/types'
import { pages } from './runtime-context'
import { pageVisualBounds } from './runtime-geometry'
import { textEntities } from './text-entity-state'
import { fileEntities } from './file-entity-state'
import { drawingEntities } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { workspaceGroups } from './space-model'
import { descendantEntityIdsForGroup } from './group-descendants'
import { withPageAnchoredEntityIds } from './page-anchor-state'

export interface SelectionScope {
  /** Top-level selected items — what reparent-on-drop, group/ungroup, and
   *  arrange act on. A group is one member, unexpanded. */
  memberIds: string[]
  /** The flat set a gesture moves/clones/deletes: groups expanded to every
   *  descendant (recursively), plus page-anchored entities attached to any
   *  dragged page. Expansion lives here and nowhere else. */
  operandIds: string[]
  /** Union rect over the operands' bounds, in canvas space — including a
   *  selected group's own stored rect, so the union wraps the group border
   *  (padding included), not just the descendants inside it. Null when the
   *  scope is empty or contains no entity with a rect. */
  bounds: WorkspaceBounds | null
}

function isGroupId(id: string): boolean {
  return workspaceGroups.some((group) => group.id === id)
}

/**
 * Expand top-level member ids into the flat operand set. A group id keeps
 * its own slot (it has a position of its own, and membership ops still need
 * to see it as a unit) and additionally contributes every descendant;
 * page-anchored entities riding any operand page are folded in last so they
 * see the fully-expanded id set (a page can be reached only directly, groups
 * don't nest pages inside a "descendant" that isn't already an operand).
 *
 * Exported so clipboard copy (`copyableEntityPayload`) can expand an explicit
 * id set — the current selection or a single duplicated id — through the
 * same expansion `resolveSelectionScope` uses, rather than a second
 * hand-rolled group walk (ADR 0034, "Groups become copyable").
 */
export function expandMembersToOperands(memberIds: string[]): string[] {
  const expanded = new Set<string>()
  for (const id of memberIds) {
    expanded.add(id)
    if (!isGroupId(id)) continue
    for (const descendantId of descendantEntityIdsForGroup(id)) {
      expanded.add(descendantId)
    }
  }
  return withPageAnchoredEntityIds([...expanded])
}

/** Union rect over the given operand ids' entity bounds. Groups contribute
 *  their own stored rect (the visual unit, padding included); edges
 *  contribute nothing — mirrors `selectionBbox` (shared/selection-bbox.ts). */
function boundsForOperands(operandIds: readonly string[]): WorkspaceBounds | null {
  if (!operandIds.length) return null
  const idSet = new Set(operandIds)
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  let count = 0
  const accumulate = (rect: WorkspaceBounds) => {
    minX = Math.min(minX, rect.x)
    minY = Math.min(minY, rect.y)
    maxX = Math.max(maxX, rect.x + rect.width)
    maxY = Math.max(maxY, rect.y + rect.height)
    count++
  }
  for (const page of pages) {
    if (idSet.has(page.id)) accumulate(pageVisualBounds(page))
  }
  for (const group of workspaceGroups) {
    if (!idSet.has(group.id)) continue
    accumulate({ x: group.canvasX, y: group.canvasY, width: group.width, height: group.height })
  }
  const leafSources: { id: string; canvasX: number; canvasY: number; width: number; height: number }[][] = [
    textEntities,
    fileEntities,
    drawingEntities,
    shapeEntities,
  ]
  for (const source of leafSources) {
    for (const entity of source) {
      if (!idSet.has(entity.id)) continue
      accumulate({ x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height })
    }
  }
  if (!count) return null
  return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
}

/**
 * Resolve the current selection into member/operand/bounds form.
 *
 * With no `anchorId`, resolves the current selection as-is (used for the
 * bounding-box broadcast). With `anchorId` (the item under the pointer for a
 * press/drag), the anchor rule governs: pressing any operand of the current
 * selection — including a descendant of a selected group — resolves to the
 * whole selection; pressing an item outside the selection resolves to that
 * item alone (plus its own group-expansion and page-anchor attachment),
 * preserving today's click-on-unselected-item drag semantics.
 */
export function resolveSelectionScope(anchorId?: string): SelectionScope {
  const currentMemberIds = uiSelectedEntityIds()
  const currentOperandIds = expandMembersToOperands(currentMemberIds)

  let memberIds = currentMemberIds
  let operandIds = currentOperandIds
  if (anchorId !== undefined && !currentOperandIds.includes(anchorId)) {
    memberIds = [anchorId]
    operandIds = expandMembersToOperands(memberIds)
  }

  return { memberIds, operandIds, bounds: boundsForOperands(operandIds) }
}
