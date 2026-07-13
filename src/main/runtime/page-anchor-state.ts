/**
 * Page-anchor lifecycle for canvas entities (see shared/page-anchor.ts for
 * the concept). This module owns the runtime side:
 *
 * - resolving an anchor from where an entity sits (creation, drag end)
 * - expanding drag/nudge id sets so anchored entities travel with their page
 * - clearing anchors when their page is deleted
 *
 * The document-binding gate (hide while the page shows a different URL)
 * lives in document-binding.ts.
 *
 * Anchorable kinds today: text and drawing. The mechanism is generic — a kind
 * opts in by carrying a `pageAnchor` field and appearing in
 * `anchorableEntities()` below.
 */

import {
  pageAnchorFor,
  type PageAnchor,
  type PageAnchorTarget,
} from '../../shared/page-anchor'
import { pages } from './runtime-context'
import { pageBodyCanvasBounds } from './runtime-geometry'
import { textEntities } from './text-entity-state'
import { drawingEntities } from './drawing-entity-state'
import { markDirty } from './layout-dirty'
import { DOC_ARRAY_ENTITY_ORDER, getActiveDoc } from './workspace-doc'

export interface AnchorableEntity {
  id: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  pageAnchor?: PageAnchor
}

function anchorableEntities(): AnchorableEntity[] {
  return [...textEntities, ...drawingEntities]
}

export function findAnchorableEntity(id: string): AnchorableEntity | undefined {
  return (
    textEntities.find((entity) => entity.id === id) ??
    drawingEntities.find((entity) => entity.id === id)
  )
}

/** Runtime pages as anchor targets, in back-to-front stack order. */
function pageAnchorTargets(): PageAnchorTarget[] {
  const rank = new Map(
    getActiveDoc()
      .getArray<string>(DOC_ARRAY_ENTITY_ORDER)
      .toArray()
      .map((id, index) => [id, index]),
  )
  return pages
    .map((page) => ({
      id: page.id,
      url: page.url,
      bounds: pageBodyCanvasBounds(page),
    }))
    .sort((a, b) => (rank.get(a.id) ?? -1) - (rank.get(b.id) ?? -1))
}

export function resolvePageAnchorForBounds(bounds: {
  x: number
  y: number
  width: number
  height: number
}): PageAnchor | null {
  return pageAnchorFor(bounds, pageAnchorTargets())
}

function sameAnchor(a: PageAnchor | undefined, b: PageAnchor | null): boolean {
  if (!a && !b) return true
  if (!a || !b) return false
  return a.pageId === b.pageId && a.pageUrl === b.pageUrl
}

/**
 * Recompute an entity's anchor from where it currently sits: anchored when
 * its center is inside a page's body, free otherwise. Grouped entities never
 * anchor — group membership already owns their movement. Returns true when
 * the anchor changed.
 */
export function reanchorEntityById(entityId: string): boolean {
  const entity = findAnchorableEntity(entityId)
  if (!entity) return false
  const next = entity.parentGroupId
    ? null
    : resolvePageAnchorForBounds({
        x: entity.canvasX,
        y: entity.canvasY,
        width: entity.width,
        height: entity.height,
      })
  if (sameAnchor(entity.pageAnchor, next)) return false
  if (next) entity.pageAnchor = next
  else delete entity.pageAnchor
  markDirty('canvas', 'sidebar')
  return true
}

/**
 * Ids of entities anchored to any page in `ids`, for expanding a drag/nudge
 * set — anchored entities travel with their page. Already-present ids are
 * not duplicated.
 */
export function withPageAnchoredEntityIds(ids: string[]): string[] {
  const pageIds = new Set(ids.filter((id) => pages.some((page) => page.id === id)))
  if (!pageIds.size) return ids
  const present = new Set(ids)
  const carried = anchorableEntities()
    .filter((entity) => entity.pageAnchor && pageIds.has(entity.pageAnchor.pageId))
    .map((entity) => entity.id)
    .filter((id) => !present.has(id))
  return carried.length ? [...ids, ...carried] : ids
}

/** Anchors referencing a deleted page are cleared — the entity goes free-form. */
export function clearPageAnchorsForPage(pageId: string): void {
  let changed = false
  for (const entity of anchorableEntities()) {
    if (entity.pageAnchor?.pageId !== pageId) continue
    delete entity.pageAnchor
    changed = true
  }
  if (changed) markDirty('canvas', 'sidebar')
}
