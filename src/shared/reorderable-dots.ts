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

import type { ProjectedSceneEntity } from './scene-projection'
import { managedLineAxis } from './layout-math'
import { detectReorderableRow, SELECTION_ROW_GAP_TOLERANCE, type Box } from './reorder-row'
import type { CanvasEntityKind } from './types'

export interface ReorderDot {
  /** Entity id — becomes `movingId` when this dot's drag begins. */
  id: string
  entityKind: CanvasEntityKind
  /** Screen-space center of the entity (and so of its dot). */
  center: { x: number; y: number }
  /** Screen-space size of the entity — caps how big the dot's hit square gets. */
  size: { width: number; height: number }
}

export interface ReorderableDotsInput {
  entities: readonly ProjectedSceneEntity[]
  selectedEntityIds: readonly string[]
  selectedGroupId?: string | null
  /** Live zoom — recovers a page's canvas-space *shell* size from its screen
   *  bounds. Defaults to 1 (canvas and screen coincide). */
  zoom?: number
}

function screenCenter(entity: ProjectedSceneEntity): { x: number; y: number } {
  return {
    x: entity.screenX + entity.screenWidth / 2,
    y: entity.screenY + entity.screenHeight / 2,
  }
}

function screenSize(entity: ProjectedSceneEntity): { width: number; height: number } {
  return { width: entity.screenWidth, height: entity.screenHeight }
}

/**
 * The box the row is detected on, in canvas space. Non-page entities use their
 * canvas box directly. A page's canvas `width`/`height` is its web-content size,
 * but the shell it occupies (and the dot's center) includes the device bezel —
 * so unproject the screen shell size by zoom. Using canvas space (not screen)
 * keeps detection exact: an arranged row's gaps are equal here, before the
 * native views round their bounds to integers.
 */
export function rowBox(entity: ProjectedSceneEntity, zoom: number): Box {
  if (entity.kind === 'page') {
    const z = zoom || 1
    return {
      id: entity.id,
      x: entity.canvasX,
      y: entity.canvasY,
      width: entity.screenWidth / z,
      height: entity.screenHeight / z,
    }
  }
  return { id: entity.id, x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height }
}

/**
 * The union of both reorder doors as a flat dot list, deduped by entity id. The
 * managed door wins ties (a managed child is never also a selection-door dot —
 * managed children are excluded from the loose-selection box set below).
 */
export function reorderableDots(input: ReorderableDotsInput): ReorderDot[] {
  const { entities, selectedEntityIds, selectedGroupId, zoom = 1 } = input
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
    dots.set(e.id, { id: e.id, entityKind: e.kind, center: screenCenter(e), size: screenSize(e) })
  }

  // Selection door: a loose equal-gap multi-selection. Managed children are
  // excluded — they belong to the managed door, so the two never fight over the
  // same dot.
  const boxes: Box[] = []
  const byId = new Map<string, ProjectedSceneEntity>()
  for (const e of entities) {
    if (e.kind === 'group') continue
    if (!selected.has(e.id)) continue
    if (childToGroup.has(e.id)) continue
    boxes.push(rowBox(e, zoom))
    byId.set(e.id, e)
  }
  const row = detectReorderableRow(boxes, { gapTolerance: SELECTION_ROW_GAP_TOLERANCE })
  if (row) {
    for (const id of row.order) {
      const e = byId.get(id)
      if (e && !dots.has(id))
        dots.set(id, { id, entityKind: e.kind, center: screenCenter(e), size: screenSize(e) })
    }
  }

  return [...dots.values()]
}
