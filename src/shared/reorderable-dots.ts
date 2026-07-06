/**
 * Shared reorder-dot selector (ADR 0015 D7, Phase C).
 *
 * The single source of truth for *which entities show a reorder dot* — consumed
 * by both the hit-tester (`collectReorderHandleTargets`) and the renderer painter
 * (`ReorderDotsLayer`). Before this existed the predicate was copy-pasted between
 * the two; the kernel duplication the plan warned against.
 *
 * Eligibility is the union of two doors:
 *   - **Selection door** (new, primary): an evenly-spaced loose multi-selection
 *     (`detectReorderableRow` ≠ null) shows a dot on each selected item.
 *   - **Managed group door** (M1, persisted): a managed row/column group whose
 *     group or child is selected shows a dot on each child.
 *
 * Eligibility runs on *canvas* geometry (fixed gap tolerance, matching the
 * commit's re-detection); the returned `center` is the entity's *screen* center,
 * so consumers can place the dot/hit-rect without re-deriving it. Pure: no
 * Electron, no DOM.
 */

import { managedLineAxis } from './layout-math'
import { detectReorderableRow, type Box } from './reorder-row'
import type { CanvasEntityKind, CanvasSceneEntity } from './types'

export interface ReorderDot {
  /** Entity id — becomes `movingId` when this dot's drag begins. */
  id: string
  entityKind: CanvasEntityKind
  /** Screen-space center of the entity (and so of its dot). */
  center: { x: number; y: number }
}

export interface ReorderableDotsInput {
  entities: readonly CanvasSceneEntity[]
  selectedEntityIds: readonly string[]
  selectedGroupId?: string | null
}

function screenCenter(entity: CanvasSceneEntity): { x: number; y: number } {
  return {
    x: entity.screenX + entity.screenWidth / 2,
    y: entity.screenY + entity.screenHeight / 2,
  }
}

/**
 * The union of both reorder doors as a flat dot list, deduped by entity id. The
 * managed door wins ties (a managed child is never also a selection-door dot —
 * managed children are excluded from the loose-selection box set below).
 */
export function reorderableDots(input: ReorderableDotsInput): ReorderDot[] {
  const { entities, selectedEntityIds, selectedGroupId } = input
  const selected = new Set(selectedEntityIds)
  const dots = new Map<string, ReorderDot>()

  // Managed door: map each managed row/column group's direct children → group
  // id, then light a child whose group or self is selected.
  const childToGroup = new Map<string, string>()
  for (const e of entities) {
    if (e.kind === 'group' && e.managedLayout && managedLineAxis(e.layoutMode) !== null) {
      for (const childId of e.entityIds) childToGroup.set(childId, e.id)
    }
  }
  for (const e of entities) {
    if (e.kind === 'group') continue
    const groupId = childToGroup.get(e.id)
    if (!groupId) continue
    if (selectedGroupId !== groupId && !selected.has(e.id)) continue
    dots.set(e.id, { id: e.id, entityKind: e.kind, center: screenCenter(e) })
  }

  // Selection door: a loose equal-gap multi-selection. Managed children are
  // excluded — they belong to the managed door, so the two never fight over the
  // same dot.
  const boxes: Box[] = []
  const byId = new Map<string, CanvasSceneEntity>()
  for (const e of entities) {
    if (e.kind === 'group') continue
    if (!selected.has(e.id)) continue
    if (childToGroup.has(e.id)) continue
    boxes.push({ id: e.id, x: e.canvasX, y: e.canvasY, width: e.width, height: e.height })
    byId.set(e.id, e)
  }
  const row = detectReorderableRow(boxes)
  if (row) {
    for (const id of row.order) {
      const e = byId.get(id)
      if (e && !dots.has(id)) dots.set(id, { id, entityKind: e.kind, center: screenCenter(e) })
    }
  }

  return [...dots.values()]
}
