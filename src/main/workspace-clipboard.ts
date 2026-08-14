import type {
  ClipboardEntityPayload,
  ClipboardEntitySelectionPayload,
  ClipboardPageSelectionPayload,
  PageAnchor,
} from '../shared/types'
import {
  createPage,
  findPageById,
} from './runtime/page-runtime'
import {
  selectPageById,
  setSelectedEntities,
  setSelectedPages,
} from './runtime/ui-actions'
import { getEntityKind } from './entities/contract'
import {
  cloneGroupEntity,
  cloneMapBackedEntity,
  type MapBackedEntityKind,
} from './runtime/entity-clone'
import { resolveSelectionScope, expandMembersToOperands } from './runtime/selection-scope'
import { setEntityParentGroupId } from './workspace-groups'
import { snapToGrid } from '../shared/gesture-utils'
import { mutateWorkspace } from './runtime/mutate-workspace'
import { makeId, cloneMetadata } from './workspace-utils'
import {
  anchorEntityToPage,
  reanchorEntityById,
} from './runtime/page-anchor-state'
import {
  pageAnchorElementShift,
  pageAnchorScrollShift,
} from './runtime/page-anchor-scroll'

/** Non-page kinds copy/paste and duplicate reuse persistence for. */
const MAP_BACKED_ENTITY_KINDS: readonly MapBackedEntityKind[] = [
  'text',
  'file',
  'shape',
  'drawing',
]

/**
 * An anchored entity's apparent canvas position — stored coords shifted by its
 * page's scroll and reference-element movement, the same projection the scene
 * builders use. Copy must serialize what the user sees: the clone re-anchors
 * with fresh references, so stale stored coords would paste at the wrong spot.
 */
function apparentPosition(entity: {
  canvasX: number
  canvasY: number
  pageAnchor?: PageAnchor
}): { canvasX: number; canvasY: number } {
  const scroll = pageAnchorScrollShift(entity.pageAnchor)
  const element = pageAnchorElementShift(entity.pageAnchor)
  return {
    canvasX: entity.canvasX - scroll.x - element.x,
    canvasY: entity.canvasY - scroll.y - element.y,
  }
}

export function copyablePagePayload(
  pageIds: string[],
): ClipboardPageSelectionPayload | null {
  if (!pageIds.length) return null

  const selectedPages = pageIds
    .map((pageId) => findPageById(pageId))
    .filter((page): page is Exclude<typeof page, undefined> => page !== undefined)

  if (!selectedPages.length) return null

  const minX = Math.min(...selectedPages.map((page) => page.canvasX))
  const minY = Math.min(...selectedPages.map((page) => page.canvasY))

  return {
    version: 1,
    pages: selectedPages.map((page) => ({
      url: page.pageView.webContents.getURL() || 'about:blank',
      presetIndex: page.presetIndex,
      dx: page.canvasX - minX,
      dy: page.canvasY - minY,
      colorScheme: page.colorScheme,
    })),
  }
}

/**
 * Copies the current selection. Resolves through `resolveSelectionScope()`'s
 * `memberIds` (top-level selection, groups unexpanded) rather than the raw
 * selected-id array — `copyableEntityPayload` does the group/descendant
 * expansion below, so a selected group and its whole subtree travel with it
 * (ADR 0034, "Groups become copyable"). Before this, a group in the
 * selection was silently dropped.
 */
export function copyableSelectionPayload():
  | ClipboardEntitySelectionPayload
  | null {
  return copyableEntityPayload(resolveSelectionScope().memberIds)
}

/**
 * Serializes an explicit set of entity ids into the same clipboard-entity
 * shape `pasteEntitiesFromClipboard` consumes. This is the single clone
 * source for both clipboard copy (current selection) and single-entity
 * duplicate (an explicit id, independent of selection) — see
 * `duplicateEntity` in workspace-pages.ts.
 *
 * `ids` are member ids, not yet expanded: `expandMembersToOperands` (the
 * same expansion `resolveSelectionScope` uses for every other gesture)
 * turns any group id in the set into itself plus every descendant, and
 * folds in page-anchored items riding a copied page (ADR 0031) — one
 * expansion, reused rather than restated here.
 */
export function copyableEntityPayload(
  ids: string[],
): ClipboardEntitySelectionPayload | null {
  if (!ids.length) return null

  // Each builder fills dx/dy with the entity's absolute apparent position;
  // rebase them onto the selection's bounding-box origin afterwards.
  const entities = expandMembersToOperands(ids)
    .map(entityPayloadAt)
    .filter((entity): entity is ClipboardEntityPayload => entity !== null)
  if (!entities.length) return null

  const minX = Math.min(...entities.map((entity) => entity.dx))
  const minY = Math.min(...entities.map((entity) => entity.dy))
  for (const entity of entities) {
    entity.dx -= minX
    entity.dy -= minY
  }

  return { version: 2, entities }
}

/** Serialize one entity with dx/dy holding its absolute apparent position. */
function entityPayloadAt(id: string): ClipboardEntityPayload | null {
  const page = findPageById(id)
  if (page) return pagePayload(page)
  const group = getEntityKind('group').entities().find((candidate) => candidate.id === id)
  if (group) return groupPayload(group)
  for (const kind of MAP_BACKED_ENTITY_KINDS) {
    const entity = getEntityKind(kind).entities().find((candidate) => candidate.id === id)
    if (entity) return mapBackedPayload(kind, entity)
  }
  return null
}

function pagePayload(page: NonNullable<ReturnType<typeof findPageById>>): ClipboardEntityPayload {
  return {
    kind: 'page',
    sourceId: page.id,
    url: page.pageView.webContents.getURL() || 'about:blank',
    presetIndex: page.presetIndex,
    metadata: cloneMetadata(page.metadata) as Record<string, unknown> | undefined,
    dx: page.canvasX,
    dy: page.canvasY,
    colorScheme: page.colorScheme,
    parentGroupId: page.parentGroupId,
  }
}

/**
 * Serialize a group as its own persisted record (`getEntityKind('group')
 * .persist()`) plus a placement delta — the same shape `mapBackedPayload`
 * uses for text/file/shape/drawing. Its descendants are separate payload
 * entries (added by the operand expansion in `copyableEntityPayload`), and
 * `record.id`/`record.parentGroupId` are what paste's id-remap pass uses to
 * rebuild the tree. A group has no page anchor, so its dx/dy is its stored
 * position verbatim (no `apparentPosition` projection needed).
 */
function groupPayload(group: { id: string }): ClipboardEntityPayload {
  const record = getEntityKind('group').persist!(group) as unknown as Record<string, unknown>
  return {
    kind: 'group',
    dx: record.canvasX as number,
    dy: record.canvasY as number,
    record,
  }
}

/**
 * Serialize a text/file/shape/drawing entity as its own persisted record
 * (`getEntityKind(kind).persist()`) plus a placement delta. The record's
 * field set comes from persistence, not a per-kind list restated here — that
 * is the fix for copy losing fields persistence carries (shape's border
 * style, fill, and text alignment; docs/plans/entity-field-drift.md).
 */
function mapBackedPayload(
  kind: MapBackedEntityKind,
  entity: { id: string },
): ClipboardEntityPayload {
  const record = getEntityKind(kind).persist!(entity) as unknown as Record<string, unknown>
  const canvasX = record.canvasX as number
  const canvasY = record.canvasY as number
  const pageAnchor = record.pageAnchor as PageAnchor | undefined
  const apparent = apparentPosition({ canvasX, canvasY, pageAnchor })
  return {
    kind,
    dx: apparent.canvasX,
    dy: apparent.canvasY,
    record,
    ...payloadAnchor(pageAnchor),
  }
}

function payloadAnchor(
  anchor: PageAnchor | undefined,
): Pick<ClipboardEntityPayload, 'pageAnchor'> {
  if (!anchor) return {}
  return {
    pageAnchor: {
      pageId: anchor.pageId,
      ...(anchor.pageUrl ? { pageUrl: anchor.pageUrl } : {}),
    },
  }
}

export function pastePagesFromClipboard(input: {
  payload: ClipboardPageSelectionPayload
  canvasX: number
  canvasY: number
}): { pageIds: string[] } {
  return mutateWorkspace(
    () => pastePagesInternal(input),
    { changed: (result) => result.pageIds.length > 0 },
  )
}

function pastePagesInternal(input: {
  payload: ClipboardPageSelectionPayload
  canvasX: number
  canvasY: number
}): { pageIds: string[] } {
  const pages = input.payload.pages.filter((page) =>
    Number.isFinite(page.presetIndex) &&
    Number.isFinite(page.dx) &&
    Number.isFinite(page.dy) &&
    typeof page.url === 'string' &&
    page.url.trim().length > 0,
  )

  if (!pages.length) {
    return { pageIds: [] }
  }

  const pageIds = pages.map((entry) => {
    const page = createPage({
      url: entry.url,
      presetIndex: entry.presetIndex,
      syncId: null,
      canvasX: snapToGrid(input.canvasX + entry.dx),
      canvasY: snapToGrid(input.canvasY + entry.dy),
      source: 'manual',
      metadata: {
        createdFrom: 'paste',
        showDeviceFrame: true,
      },
      colorScheme: entry.colorScheme,
    })
    return page.id
  })

  if (pageIds.length === 1) {
    selectPageById(pageIds[0])
  } else {
    setSelectedPages(pageIds)
  }

  return { pageIds }
}

export function pasteEntitiesFromClipboard(input: {
  payload: ClipboardEntitySelectionPayload
  canvasX: number
  canvasY: number
}): { entityIds: string[] } {
  return mutateWorkspace(
    () => pasteEntitiesInternal(input),
    { changed: (result) => result.entityIds.length > 0 },
  )
}

/** One created clone: its id, its source id (for the tree-rebuild id map),
 *  its source group membership (if any), and page-anchor re-attachment refs. */
type PastedEntity = {
  id: string
  sourceId?: string
  sourceParentGroupId?: string
  sourcePageId?: string
  anchorable?: boolean
  sourceAnchorPageId?: string
}

function isMapBackedKind(
  kind: ClipboardEntityPayload['kind'],
): kind is MapBackedEntityKind {
  return (MAP_BACKED_ENTITY_KINDS as readonly string[]).includes(kind)
}

function createPastedEntity(
  entity: ClipboardEntityPayload,
  canvasX: number,
  canvasY: number,
): PastedEntity | null {
  if (!Number.isFinite(entity.dx) || !Number.isFinite(entity.dy)) return null
  const x = canvasX + entity.dx
  const y = canvasY + entity.dy
  if (entity.kind === 'page') return createPastedPage(entity, x, y)
  if (entity.kind === 'group') return createPastedGroup(entity, x, y)
  if (!isMapBackedKind(entity.kind) || !entity.record) return null
  if (entity.kind === 'file') {
    const file = entity.record.file
    if (typeof file !== 'string' || !file.trim().length) return null
  }
  const clone = cloneMapBackedEntity(entity.kind, entity.record, {
    id: makeId(entity.kind),
    canvasX: x,
    canvasY: y,
  })
  return {
    id: clone.id as string,
    sourceId: entity.record.id as string | undefined,
    sourceParentGroupId: entity.record.parentGroupId as string | undefined,
    anchorable: entity.kind !== 'file',
    sourceAnchorPageId: entity.pageAnchor?.pageId,
  }
}

/**
 * Clone a copied group through `cloneGroupEntity` — persist → re-id → offset
 * → restore, the same shape map-backed clones use. `parentGroupId` is left
 * unset here; `pasteEntitiesInternal`'s remap pass sets it once every clone
 * in the batch has a new id, so a nested group can resolve its own cloned
 * parent regardless of creation order.
 */
function createPastedGroup(
  entity: ClipboardEntityPayload,
  canvasX: number,
  canvasY: number,
): PastedEntity | null {
  if (!entity.record) return null
  const clone = cloneGroupEntity(entity.record, {
    id: makeId('group'),
    canvasX,
    canvasY,
  })
  return {
    id: clone.id as string,
    sourceId: entity.record.id as string | undefined,
    sourceParentGroupId: entity.record.parentGroupId as string | undefined,
  }
}

function createPastedPage(
  entity: ClipboardEntityPayload,
  canvasX: number,
  canvasY: number,
): PastedEntity | null {
  if (
    !Number.isFinite(entity.presetIndex) ||
    typeof entity.url !== 'string' ||
    !entity.url?.trim().length
  ) return null
  const page = createPage({
    url: entity.url,
    presetIndex: entity.presetIndex!,
    syncId: null,
    canvasX,
    canvasY,
    source: 'manual',
    metadata: { ...entity.metadata, createdFrom: 'paste' },
    colorScheme: entity.colorScheme,
  })
  return {
    id: page.id,
    sourceId: entity.sourceId,
    sourceParentGroupId: entity.parentGroupId,
    sourcePageId: entity.sourceId,
  }
}

function pasteEntitiesInternal(input: {
  payload: ClipboardEntitySelectionPayload
  canvasX: number
  canvasY: number
}): { entityIds: string[] } {
  const pasted = input.payload.entities
    .map((entity) => createPastedEntity(entity, input.canvasX, input.canvasY))
    .filter((entry): entry is PastedEntity => entry !== null)
  const entityIds = pasted.map((entry) => entry.id)
  if (!entityIds.length) return { entityIds: [] }

  /** Copied page id → its clone's id, for re-attaching anchored items. */
  const clonedPageIds = new Map(
    pasted
      .filter((entry) => entry.sourcePageId)
      .map((entry) => [entry.sourcePageId!, entry.id]),
  )
  const anchorables = pasted.filter((entry) => entry.anchorable)

  // Attachment lifecycle for the clones: an item copied together with its
  // page re-attaches to the page's clone; otherwise placement decides
  // (ADR 0031) — the clone keeps the original page iff it still sits on it,
  // and detaches when it lands on empty canvas.
  for (const item of anchorables) {
    const clonePageId = item.sourceAnchorPageId
      ? clonedPageIds.get(item.sourceAnchorPageId)
      : undefined
    if (clonePageId) anchorEntityToPage(item.id, clonePageId)
    else reanchorEntityById(item.id)
  }

  // Group-membership remap (ADR 0034, "Groups become copyable"): a
  // pasted-entity id map spans every kind in this paste — page, group, and
  // map-backed alike — because a group's children can be any of them. An
  // entity whose source parent was copied alongside it rejoins its cloned
  // parent; one whose source parent was NOT part of the copy stays
  // unparented (`cloneMapBackedEntity`/`cloneGroupEntity` already default
  // `parentGroupId` to `undefined`), preserving the existing "paste doesn't
  // rejoin a group" behavior, now scoped to parents outside the copy.
  const sourceIdToClonedId = new Map(
    pasted
      .filter((entry) => entry.sourceId)
      .map((entry) => [entry.sourceId!, entry.id]),
  )
  for (const entry of pasted) {
    if (!entry.sourceParentGroupId) continue
    const clonedParentId = sourceIdToClonedId.get(entry.sourceParentGroupId)
    if (clonedParentId) setEntityParentGroupId(entry.id, clonedParentId)
  }

  setSelectedEntities(entityIds)
  return { entityIds }
}
