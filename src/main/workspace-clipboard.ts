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
import { cloneMetadata } from './workspace-utils'

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
  const entityIds = withPageAnchoredEntityIds(ids)

  const entities: ClipboardEntityPayload[] = []

  // Collect all positionable entities for bounding box
  const allPositions: { canvasX: number; canvasY: number }[] = []

  for (const id of entityIds) {
    const page = findPageById(id)
    if (page) {
      allPositions.push({ canvasX: page.canvasX, canvasY: page.canvasY })
      continue
    }
    const note = textEntities.find((n) => n.id === id)
    if (note) {
      allPositions.push({ canvasX: note.canvasX, canvasY: note.canvasY })
      continue
    }
    const file = fileEntities.find((f) => f.id === id)
    if (file) {
      allPositions.push({ canvasX: file.canvasX, canvasY: file.canvasY })
      continue
    }
    const shape = shapeEntities.find((s) => s.id === id)
    if (shape) {
      allPositions.push({ canvasX: shape.canvasX, canvasY: shape.canvasY })
      continue
    }
    const drawing = drawingEntities.find((d) => d.id === id)
    if (drawing) {
      allPositions.push({ canvasX: drawing.canvasX, canvasY: drawing.canvasY })
    }
  }

  if (!allPositions.length) return null

  const minX = Math.min(...allPositions.map((p) => p.canvasX))
  const minY = Math.min(...allPositions.map((p) => p.canvasY))

  for (const id of entityIds) {
    const page = findPageById(id)
    if (page) {
      entities.push({
        kind: 'page',
        sourceId: page.id,
        url: page.pageView.webContents.getURL() || 'about:blank',
        presetIndex: page.presetIndex,
        metadata: cloneMetadata(page.metadata) as Record<string, unknown> | undefined,
        dx: page.canvasX - minX,
        dy: page.canvasY - minY,
        colorScheme: page.colorScheme,
      })
      continue
    }
    const note = textEntities.find((n) => n.id === id)
    if (note) {
      entities.push({
        kind: 'text',
        text: note.text,
        color: note.color,
        textStyle: note.textStyle,
        textSize: note.textSize,
        width: note.width,
        height: note.height,
        dx: note.canvasX - minX,
        dy: note.canvasY - minY,
        ...payloadAnchor(note.pageAnchor),
      })
      continue
    }
    const file = fileEntities.find((f) => f.id === id)
    if (file) {
      entities.push({
        kind: 'file',
        file: file.file,
        subpath: file.subpath,
        width: file.width,
        height: file.height,
        dx: file.canvasX - minX,
        dy: file.canvasY - minY,
        presetIndex: file.presetIndex,
        metadata: file.metadata,
        objectFit: file.objectFit,
      })
      continue
    }
    const shape = shapeEntities.find((s) => s.id === id)
    if (shape) {
      entities.push({
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
        dx: shape.canvasX - minX,
        dy: shape.canvasY - minY,
        ...payloadAnchor(shape.pageAnchor),
      })
      continue
    }
    const drawing = drawingEntities.find((d) => d.id === id)
    if (drawing) {
      entities.push({
        kind: 'drawing',
        width: drawing.width,
        height: drawing.height,
        strokes: drawing.strokes.map((stroke) => ({
          ...stroke,
          points: stroke.points.map((point) => ({
            x: point.x - drawing.canvasX,
            y: point.y - drawing.canvasY,
          })),
        })),
        label: drawing.label,
        dx: drawing.canvasX - minX,
        dy: drawing.canvasY - minY,
        ...payloadAnchor(drawing.pageAnchor),
      })
    }
  }

  if (!entities.length) return null

  return { version: 2, entities }
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

function pasteEntitiesInternal(input: {
  payload: ClipboardEntitySelectionPayload
  canvasX: number
  canvasY: number
}): { entityIds: string[] } {
  const entityIds: string[] = []
  /** Copied page id → its clone's id, for re-attaching anchored items. */
  const clonedPageIds = new Map<string, string>()
  const anchorables: Array<{ id: string; sourceAnchorPageId?: string }> = []

  for (const entity of input.payload.entities) {
    if (!Number.isFinite(entity.dx) || !Number.isFinite(entity.dy)) continue

    if (entity.kind === 'page') {
      if (
        !Number.isFinite(entity.presetIndex) ||
        typeof entity.url !== 'string' ||
        !entity.url?.trim().length
      ) continue

      const pasteMetadata = entity.metadata
        ? { ...entity.metadata, createdFrom: 'paste' }
        : { createdFrom: 'paste' }
      const page = createPage({
        url: entity.url!,
        presetIndex: entity.presetIndex!,
        syncId: null,
        canvasX: input.canvasX + entity.dx,
        canvasY: input.canvasY + entity.dy,
        source: 'manual',
        metadata: pasteMetadata,
        colorScheme: entity.colorScheme,
      })
      if (entity.sourceId) clonedPageIds.set(entity.sourceId, page.id)
      entityIds.push(page.id)
    } else if (entity.kind === 'text') {
      const note = createTextEntityInState({
        canvasX: input.canvasX + entity.dx,
        canvasY: input.canvasY + entity.dy,
        text: entity.text,
        color: entity.color,
        textStyle: entity.textStyle,
        textSize: entity.textSize,
        width: entity.width,
        height: entity.height,
      })
      anchorables.push({ id: note.id, sourceAnchorPageId: entity.pageAnchor?.pageId })
      entityIds.push(note.id)
    } else if (entity.kind === 'file') {
      if (typeof entity.file !== 'string' || !entity.file.trim().length) continue
      const file = createFileEntityInState({
        canvasX: input.canvasX + entity.dx,
        canvasY: input.canvasY + entity.dy,
        file: entity.file,
        subpath: entity.subpath,
        width: entity.width,
        height: entity.height,
        presetIndex: entity.presetIndex,
        metadata: entity.metadata ? { ...entity.metadata } : undefined,
        objectFit: entity.objectFit,
      })
      entityIds.push(file.id)
    } else if (entity.kind === 'shape') {
      const shape = createShapeEntityInState({
        canvasX: input.canvasX + entity.dx,
        canvasY: input.canvasY + entity.dy,
        shapeKind: entity.shapeKind,
        text: entity.text,
        color: entity.color,
        strokeWidth: entity.strokeWidth,
        textSize: entity.textSize,
        theme: entity.theme,
        label: entity.label,
        width: entity.width,
        height: entity.height,
      })
      anchorables.push({ id: shape.id, sourceAnchorPageId: entity.pageAnchor?.pageId })
      entityIds.push(shape.id)
    } else if (entity.kind === 'drawing') {
      const canvasX = input.canvasX + entity.dx
      const canvasY = input.canvasY + entity.dy
      const drawing = createDrawingEntityInState({
        canvasX,
        canvasY,
        width: entity.width ?? 0,
        height: entity.height ?? 0,
        strokes: (entity.strokes ?? []).map((stroke) => ({
          ...stroke,
          id: `${stroke.id}_paste_${Math.random().toString(36).slice(2, 8)}`,
          points: stroke.points.map((point) => ({
            x: point.x + canvasX,
            y: point.y + canvasY,
          })),
        })),
        label: entity.label,
      })
      anchorables.push({ id: drawing.id, sourceAnchorPageId: entity.pageAnchor?.pageId })
      entityIds.push(drawing.id)
    }
  }

  if (!entityIds.length) return { entityIds: [] }

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
