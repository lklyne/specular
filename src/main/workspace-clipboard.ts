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
import { textEntities } from './runtime/text-entity-state'
import { fileEntities } from './runtime/file-entity-state'
import { shapeEntities } from './runtime/shape-entity-state'
import { createTextEntity as createTextEntityInState } from './runtime/text-entity-state'
import { createFileEntity as createFileEntityInState } from './runtime/file-entity-state'
import { createShapeEntity as createShapeEntityInState } from './runtime/shape-entity-state'
import {
  createDrawingEntity as createDrawingEntityInState,
  drawingEntities,
} from './runtime/drawing-entity-state'
import { snapToGrid } from '../shared/gesture-utils'
import { mutateWorkspace } from './runtime/mutate-workspace'
import {
  anchorEntityToPage,
  reanchorEntityById,
  withPageAnchoredEntityIds,
} from './runtime/page-anchor-state'
import {
  pageAnchorElementShift,
  pageAnchorScrollShift,
} from './runtime/page-anchor-scroll'
import { cloneMetadata } from './workspace-utils'

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
  const note = textEntities.find((n) => n.id === id)
  if (note) return textPayload(note)
  const file = fileEntities.find((f) => f.id === id)
  if (file) return filePayload(file)
  const shape = shapeEntities.find((s) => s.id === id)
  if (shape) return shapePayload(shape)
  const drawing = drawingEntities.find((d) => d.id === id)
  if (drawing) return drawingPayload(drawing)
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

function textPayload(note: (typeof textEntities)[number]): ClipboardEntityPayload {
  const apparent = apparentPosition(note)
  return {
    kind: 'text',
    text: note.text,
    color: note.color,
    textStyle: note.textStyle,
    textSize: note.textSize,
    width: note.width,
    height: note.height,
    dx: apparent.canvasX,
    dy: apparent.canvasY,
    ...payloadAnchor(note.pageAnchor),
  }
}

function filePayload(file: (typeof fileEntities)[number]): ClipboardEntityPayload {
  return {
    kind: 'file',
    file: file.file,
    subpath: file.subpath,
    width: file.width,
    height: file.height,
    dx: file.canvasX,
    dy: file.canvasY,
    presetIndex: file.presetIndex,
    metadata: file.metadata,
    objectFit: file.objectFit,
  }
}

function shapePayload(shape: (typeof shapeEntities)[number]): ClipboardEntityPayload {
  const apparent = apparentPosition(shape)
  return {
    kind: 'shape',
    shapeKind: shape.shapeKind,
    text: shape.text,
    color: shape.color,
    strokeWidth: shape.strokeWidth,
    textSize: shape.textSize,
    theme: shape.theme,
    label: shape.label,
    width: shape.width,
    height: shape.height,
    dx: apparent.canvasX,
    dy: apparent.canvasY,
    ...payloadAnchor(shape.pageAnchor),
  }
}

function drawingPayload(drawing: (typeof drawingEntities)[number]): ClipboardEntityPayload {
  const apparent = apparentPosition(drawing)
  return {
    kind: 'drawing',
    width: drawing.width,
    height: drawing.height,
    // Stroke points stay relative to the STORED origin: points and origin
    // shift together, so the offsets are shift-invariant.
    strokes: drawing.strokes.map((stroke) => ({
      ...stroke,
      points: stroke.points.map((point) => ({
        x: point.x - drawing.canvasX,
        y: point.y - drawing.canvasY,
      })),
    })),
    label: drawing.label,
    dx: apparent.canvasX,
    dy: apparent.canvasY,
    ...payloadAnchor(drawing.pageAnchor),
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

function createPastedEntity(
  entity: ClipboardEntityPayload,
  canvasX: number,
  canvasY: number,
): PastedEntity | null {
  if (!Number.isFinite(entity.dx) || !Number.isFinite(entity.dy)) return null
  const x = canvasX + entity.dx
  const y = canvasY + entity.dy
  switch (entity.kind) {
    case 'page':
      return createPastedPage(entity, x, y)
    case 'text':
      return {
        id: createTextEntityInState({
          canvasX: x,
          canvasY: y,
          text: entity.text,
          color: entity.color,
          textStyle: entity.textStyle,
          textSize: entity.textSize,
          width: entity.width,
          height: entity.height,
        }).id,
        anchorable: true,
        sourceAnchorPageId: entity.pageAnchor?.pageId,
      }
    case 'file':
      if (typeof entity.file !== 'string' || !entity.file.trim().length) return null
      return {
        id: createFileEntityInState({
          canvasX: x,
          canvasY: y,
          file: entity.file,
          subpath: entity.subpath,
          width: entity.width,
          height: entity.height,
          presetIndex: entity.presetIndex,
          metadata: entity.metadata ? { ...entity.metadata } : undefined,
          objectFit: entity.objectFit,
        }).id,
      }
    case 'shape':
      return {
        id: createShapeEntityInState({
          canvasX: x,
          canvasY: y,
          shapeKind: entity.shapeKind,
          text: entity.text,
          color: entity.color,
          strokeWidth: entity.strokeWidth,
          textSize: entity.textSize,
          theme: entity.theme,
          label: entity.label,
          width: entity.width,
          height: entity.height,
        }).id,
        anchorable: true,
        sourceAnchorPageId: entity.pageAnchor?.pageId,
      }
    case 'drawing':
      return {
        id: createDrawingEntityInState({
          canvasX: x,
          canvasY: y,
          width: entity.width ?? 0,
          height: entity.height ?? 0,
          strokes: (entity.strokes ?? []).map((stroke) => ({
            ...stroke,
            id: `${stroke.id}_paste_${Math.random().toString(36).slice(2, 8)}`,
            points: stroke.points.map((point) => ({ x: point.x + x, y: point.y + y })),
          })),
          label: entity.label,
        }).id,
        anchorable: true,
        sourceAnchorPageId: entity.pageAnchor?.pageId,
      }
    default:
      return null
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
