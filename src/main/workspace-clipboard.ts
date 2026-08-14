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
  getSelectedEntityIds,
  selectPageById,
  setSelectedEntities,
  setSelectedPages,
} from './runtime/ui-actions'
import { getEntityKind } from './entities/contract'
import {
  cloneMapBackedEntity,
  type MapBackedEntityKind,
} from './runtime/entity-clone'
import { snapToGrid } from '../shared/gesture-utils'
import { mutateWorkspace } from './runtime/mutate-workspace'
import { makeId, cloneMetadata } from './workspace-utils'
import {
  anchorEntityToPage,
  reanchorEntityById,
  withPageAnchoredEntityIds,
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

export function copyableSelectionPayload():
  | ClipboardEntitySelectionPayload
  | null {
  return copyableEntityPayload(getSelectedEntityIds())
}

/**
 * Serializes an explicit set of entity ids into the same clipboard-entity
 * shape `pasteEntitiesFromClipboard` consumes. This is the single clone
 * source for both clipboard copy (current selection) and single-entity
 * duplicate (an explicit id, independent of selection) — see
 * `duplicateEntity` in workspace-pages.ts.
 *
 * Copying a page carries its page-anchored items the same way dragging a
 * page carries them (ADR 0031); the paste side re-attaches the cloned items
 * to the cloned page.
 */
export function copyableEntityPayload(
  ids: string[],
): ClipboardEntitySelectionPayload | null {
  if (!ids.length) return null

  // Each builder fills dx/dy with the entity's absolute apparent position;
  // rebase them onto the selection's bounding-box origin afterwards.
  const entities = withPageAnchoredEntityIds(ids)
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

/** One created clone: its id, whether it can re-anchor, and its source refs. */
type PastedEntity = {
  id: string
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
    anchorable: entity.kind !== 'file',
    sourceAnchorPageId: entity.pageAnchor?.pageId,
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
  return { id: page.id, sourcePageId: entity.sourceId }
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

  setSelectedEntities(entityIds)
  return { entityIds }
}
