import { randomUUID } from 'crypto'
import type { Route } from './types'
import {
  createDrawingEntity,
  createFileEntity,
  createTextEntity,
  deleteDrawingEntity,
  deleteFileEntity,
  deleteTextEntity,
  getDrawingEntities,
  getFileEntities,
  getTextEntities,
  updateDrawingEntity,
  updateFileEntity,
  updateTextEntity,
} from '../runtime/document-commands'
import { createNoteFile } from '../runtime/note-assets'
import { createPages } from '../workspace-pages'
import { deletePages } from '../workspace-entities'
import { findPageById } from '../runtime/runtime-context'
import { htmlDefaultSize, imageSizeFromPath, videoSizeFromPath } from '../runtime/image-sizing'
import {
  animateCursorScan,
  allEntityPositions,
  movePresenceCursorTo,
  staggerOperation,
} from '../presence-manager'
import { writeJson } from './http-helpers'

export const entityRoutes: Route[] = [
  // --- Text entities ---
  {
    method: 'GET',
    pattern: '/text-entities',
    async handler({ request, response }) {
      animateCursorScan(request, allEntityPositions(), 'read_content')
      writeJson(response, 200, { textEntities: getTextEntities() })
    },
  },
  {
    method: 'POST',
    pattern: '/text-entities/create',
    async handler({ request, response, body }) {
      const payload = body as {
        canvasX?: number
        canvasY?: number
        text?: string
        color?: string
        width?: number
        height?: number
        id?: string
        items?: Array<{ canvasX: number; canvasY: number; text?: string; color?: string; width?: number; height?: number; id?: string }>
      }
      const items = payload.items ?? [payload]
      if (items.length <= 1) {
        const item = items[0]
        if (typeof item?.canvasX !== 'number' || typeof item?.canvasY !== 'number') {
          writeJson(response, 400, { error: 'canvasX and canvasY are required numbers' })
          return
        }
        movePresenceCursorTo(request, item.canvasX, item.canvasY, null)
        writeJson(
          response,
          200,
          createTextEntity({
            id: item.id,
            canvasX: item.canvasX,
            canvasY: item.canvasY,
            text: item.text,
            color: item.color,
            width: item.width,
            height: item.height,
          }),
        )
        return
      }
      const validItems = items.filter(
        (item): item is typeof item & { canvasX: number; canvasY: number } =>
          typeof item.canvasX === 'number' && typeof item.canvasY === 'number',
      )
      validItems.forEach((item) => { item.id = item.id ?? `text_${randomUUID()}` })
      writeJson(response, 200, { items: validItems.map((item) => ({ id: item.id })) })
      staggerOperation(
        request,
        validItems.map((item) => ({ x: item.canvasX, y: item.canvasY })),
        null,
        (i) => createTextEntity(validItems[i]),
      )
    },
  },
  {
    method: 'POST',
    pattern: '/text-entities/update',
    async handler({ request, response, body }) {
      const payload = body as {
        id?: string
        patch?: { text?: string; color?: string; width?: number; height?: number; canvasX?: number; canvasY?: number }
        items?: Array<{ id: string; patch: { text?: string; color?: string; width?: number; height?: number; canvasX?: number; canvasY?: number } }>
      }
      const items = payload.items ?? [{ id: payload.id!, patch: payload.patch! }]
      const results: unknown[] = []
      const positions: Array<{ x: number; y: number }> = []
      for (const item of items) {
        if (!item.id || !item.patch) continue
        const existing = getTextEntities().find((e) => e.id === item.id)
        if (existing) positions.push({ x: existing.canvasX, y: existing.canvasY })
        const entity = updateTextEntity(item.id, item.patch)
        if (entity) results.push(entity)
      }
      if (positions.length === 1) movePresenceCursorTo(request, positions[0].x, positions[0].y, null)
      else if (positions.length > 1) animateCursorScan(request, positions, null)
      writeJson(response, 200, items.length === 1 && !payload.items ? results[0] ?? { error: 'not found' } : { items: results })
    },
  },
  {
    method: 'POST',
    pattern: '/text-entities/delete',
    async handler({ request, response, body }) {
      const payload = body as {
        id?: string
        ids?: string[]
      }
      const ids = payload.ids ?? [payload.id!]
      if (ids.length <= 1) {
        const id = ids[0]
        if (id) {
          const existing = getTextEntities().find((e) => e.id === id)
          if (existing) movePresenceCursorTo(request, existing.canvasX, existing.canvasY, null)
        }
        const deleted = id ? deleteTextEntity(id) : false
        writeJson(response, 200, !payload.ids ? { ok: deleted } : { deleted: deleted ? ids : [] })
        return
      }
      const itemsToDelete = ids
        .filter((id): id is string => Boolean(id))
        .map((id) => { const e = getTextEntities().find((te) => te.id === id); return e ? { id, x: e.canvasX, y: e.canvasY } : null })
        .filter((p): p is NonNullable<typeof p> => p !== null)
      writeJson(response, 200, { deleted: itemsToDelete.map((i) => i.id) })
      staggerOperation(request, itemsToDelete, null, (i) => deleteTextEntity(itemsToDelete[i].id))
    },
  },
  // --- File entities ---
  {
    method: 'GET',
    pattern: '/file-entities',
    async handler({ request, response }) {
      animateCursorScan(request, allEntityPositions(), 'read_content')
      writeJson(response, 200, { fileEntities: getFileEntities() })
    },
  },
  {
    method: 'POST',
    pattern: '/file-entities/create',
    async handler({ request, response, body }) {
      const payload = body as {
        canvasX?: number
        canvasY?: number
        file?: string
        subpath?: string
        width?: number
        height?: number
        id?: string
        items?: Array<{ canvasX: number; canvasY: number; file: string; subpath?: string; width?: number; height?: number; id?: string }>
      }
      const resolveFileDimensions = (item: { file: string; width?: number; height?: number }) => {
        if (item.width != null && item.height != null) return { width: item.width, height: item.height }
        const detected = imageSizeFromPath(item.file) ?? videoSizeFromPath(item.file) ?? htmlDefaultSize(item.file)
        return detected ?? { width: item.width, height: item.height }
      }
      const items = payload.items ?? [payload]
      if (items.length <= 1) {
        const item = items[0]
        if (typeof item?.canvasX !== 'number' || typeof item?.canvasY !== 'number' || typeof item?.file !== 'string') {
          writeJson(response, 400, { error: 'canvasX, canvasY, and file are required' })
          return
        }
        const dims = resolveFileDimensions(item as { file: string; width?: number; height?: number })
        const id = item.id ?? `file_${randomUUID()}`
        movePresenceCursorTo(request, item.canvasX, item.canvasY, null)
        createFileEntity({
          id,
          canvasX: item.canvasX,
          canvasY: item.canvasY,
          file: item.file,
          subpath: item.subpath,
          width: dims.width,
          height: dims.height,
        })
        writeJson(response, 200, { id })
        return
      }
      const validItems = items.filter(
        (item): item is typeof item & { canvasX: number; canvasY: number; file: string } =>
          typeof item.canvasX === 'number' &&
          typeof item.canvasY === 'number' &&
          typeof item.file === 'string',
      )
      validItems.forEach((item) => { item.id = item.id ?? `file_${randomUUID()}` })
      writeJson(response, 200, { items: validItems.map((item) => ({ id: item.id })) })
      staggerOperation(
        request,
        validItems.map((item) => ({ x: item.canvasX, y: item.canvasY })),
        null,
        (i) => {
          const item = validItems[i]
          const dims = resolveFileDimensions(item)
          createFileEntity({ ...item, width: dims.width, height: dims.height })
        },
      )
    },
  },
  {
    method: 'POST',
    pattern: '/file-entities/update',
    async handler({ request, response, body }) {
      const payload = body as {
        id?: string
        patch?: { file?: string; subpath?: string; width?: number; height?: number; canvasX?: number; canvasY?: number }
        items?: Array<{ id: string; patch: { file?: string; subpath?: string; width?: number; height?: number; canvasX?: number; canvasY?: number } }>
      }
      const items = payload.items ?? [{ id: payload.id!, patch: payload.patch! }]
      const results: unknown[] = []
      const positions: Array<{ x: number; y: number }> = []
      for (const item of items) {
        if (!item.id || !item.patch) continue
        const existing = getFileEntities().find((e) => e.id === item.id)
        if (existing) positions.push({ x: existing.canvasX, y: existing.canvasY })
        const entity = updateFileEntity(item.id, item.patch)
        if (entity) results.push(entity)
      }
      if (positions.length === 1) movePresenceCursorTo(request, positions[0].x, positions[0].y, null)
      else if (positions.length > 1) animateCursorScan(request, positions, null)
      writeJson(response, 200, items.length === 1 && !payload.items ? results[0] ?? { error: 'not found' } : { items: results })
    },
  },
  {
    method: 'POST',
    pattern: '/file-entities/delete',
    async handler({ request, response, body }) {
      const payload = body as {
        id?: string
        ids?: string[]
      }
      const ids = payload.ids ?? [payload.id!]
      if (ids.length <= 1) {
        const id = ids[0]
        if (id) {
          const existing = getFileEntities().find((e) => e.id === id)
          if (existing) movePresenceCursorTo(request, existing.canvasX, existing.canvasY, null)
        }
        const deleted = id ? deleteFileEntity(id) : false
        writeJson(response, 200, !payload.ids ? { ok: deleted } : { deleted: deleted ? ids : [] })
        return
      }
      const itemsToDelete = ids
        .filter((id): id is string => Boolean(id))
        .map((id) => { const e = getFileEntities().find((fe) => fe.id === id); return e ? { id, x: e.canvasX, y: e.canvasY } : null })
        .filter((p): p is NonNullable<typeof p> => p !== null)
      writeJson(response, 200, { deleted: itemsToDelete.map((i) => i.id) })
      staggerOperation(request, itemsToDelete, null, (i) => deleteFileEntity(itemsToDelete[i].id))
    },
  },
  // --- Note entities ---
  {
    method: 'POST',
    pattern: '/note-entities/create',
    async handler({ request, response, body }) {
      const payload = body as {
        canvasX?: number
        canvasY?: number
        name?: string
        content?: string
        width?: number
        height?: number
      }
      if (typeof payload.canvasX !== 'number' || typeof payload.canvasY !== 'number') {
        writeJson(response, 400, { error: 'canvasX and canvasY are required' })
        return
      }
      const filePath = createNoteFile(payload.name, payload.content)
      const entity = createFileEntity({
        canvasX: payload.canvasX,
        canvasY: payload.canvasY,
        file: filePath,
        width: payload.width ?? 300,
        height: payload.height ?? 300,
      })
      animateCursorScan(request, [{ x: payload.canvasX, y: payload.canvasY }], null)
      writeJson(response, 200, { id: entity.id, file: filePath })
    },
  },
  // --- Mixed entity batch ---
  {
    method: 'POST',
    pattern: '/entities/create',
    async handler({ request, response, body }) {
      const payload = body as {
        items: Array<{
          kind: 'page' | 'text' | 'file'
          canvasX: number
          canvasY: number
          url?: string
          presetIndex?: number
          linked?: boolean
          groupId?: string
          text?: string
          color?: string
          file?: string
          subpath?: string
          width?: number
          height?: number
        }>
      }
      if (!Array.isArray(payload.items)) {
        writeJson(response, 400, { error: 'items array is required' })
        return
      }
      const itemsWithIds = payload.items.map((item) => {
        const prefix = item.kind === 'page' ? 'page' : item.kind === 'text' ? 'text' : 'file'
        return { ...item, id: `${prefix}_${randomUUID()}` }
      })
      writeJson(response, 200, { items: itemsWithIds.map((item) => ({ kind: item.kind, id: item.id })) })
      staggerOperation(
        request,
        itemsWithIds.map((item) => ({ x: item.canvasX, y: item.canvasY })),
        'create_page',
        (i) => {
          const item = itemsWithIds[i]
          if (item.kind === 'page') {
            createPages({ pages: [{ id: item.id, url: item.url!, presetIndex: item.presetIndex ?? 0, canvasX: item.canvasX, canvasY: item.canvasY, linked: item.linked, groupId: item.groupId }] })
          } else if (item.kind === 'text') {
            createTextEntity({ id: item.id, canvasX: item.canvasX, canvasY: item.canvasY, text: item.text, color: item.color, width: item.width, height: item.height })
          } else if (item.kind === 'file') {
            createFileEntity({ id: item.id, canvasX: item.canvasX, canvasY: item.canvasY, file: item.file!, subpath: item.subpath, width: item.width, height: item.height })
          }
        },
      )
    },
  },
  // --- Drawing entities ---
  {
    method: 'GET',
    pattern: '/drawing-entities',
    async handler({ response }) {
      writeJson(response, 200, { drawingEntities: getDrawingEntities() })
    },
  },
  {
    method: 'POST',
    pattern: '/drawing-entities/create',
    async handler({ response, body }) {
      const payload = body as {
        canvasX?: number
        canvasY?: number
        width?: number
        height?: number
        strokes?: import('../../shared/types').AnnotationDrawingStroke[]
        id?: string
      }
      if (
        typeof payload.canvasX !== 'number' ||
        typeof payload.canvasY !== 'number' ||
        typeof payload.width !== 'number' ||
        typeof payload.height !== 'number' ||
        !Array.isArray(payload.strokes)
      ) {
        writeJson(response, 400, { error: 'canvasX, canvasY, width, height, and strokes are required' })
        return
      }
      const entity = createDrawingEntity({
        id: payload.id,
        canvasX: payload.canvasX,
        canvasY: payload.canvasY,
        width: payload.width,
        height: payload.height,
        strokes: payload.strokes,
      })
      writeJson(response, 200, entity)
    },
  },
  {
    method: 'POST',
    pattern: '/drawing-entities/update',
    async handler({ response, body }) {
      const payload = body as {
        id?: string
        patch?: {
          canvasX?: number
          canvasY?: number
          width?: number
          height?: number
          strokes?: import('../../shared/types').AnnotationDrawingStroke[]
        }
      }
      if (!payload.id || !payload.patch) {
        writeJson(response, 400, { error: 'id and patch are required' })
        return
      }
      const entity = updateDrawingEntity(payload.id, payload.patch)
      writeJson(response, 200, entity ?? { error: 'not found' })
    },
  },
  {
    method: 'POST',
    pattern: '/drawing-entities/delete',
    async handler({ response, body }) {
      const payload = body as { id?: string; ids?: string[] }
      const ids = payload.ids ?? (payload.id ? [payload.id] : [])
      const deleted: string[] = []
      for (const id of ids) {
        if (deleteDrawingEntity(id)) deleted.push(id)
      }
      writeJson(response, 200, { deleted })
    },
  },
  {
    method: 'POST',
    pattern: '/entities/delete',
    async handler({ request, response, body }) {
      const payload = body as {
        items: Array<{ kind: 'page' | 'text' | 'file'; id: string }>
      }
      if (!Array.isArray(payload.items)) {
        writeJson(response, 400, { error: 'items array is required' })
        return
      }
      const itemsToDelete: Array<{ kind: string; id: string; x: number; y: number }> = []
      for (const item of payload.items) {
        let pos: { x: number; y: number } | null = null
        if (item.kind === 'page') {
          const page = findPageById(item.id)
          if (page) pos = { x: page.canvasX, y: page.canvasY }
        } else if (item.kind === 'text') {
          const te = getTextEntities().find((e) => e.id === item.id)
          if (te) pos = { x: te.canvasX, y: te.canvasY }
        } else if (item.kind === 'file') {
          const fe = getFileEntities().find((e) => e.id === item.id)
          if (fe) pos = { x: fe.canvasX, y: fe.canvasY }
        }
        if (pos) itemsToDelete.push({ kind: item.kind, id: item.id, ...pos })
      }
      writeJson(response, 200, { items: payload.items.map((item) => ({ kind: item.kind, id: item.id, deleted: true })) })
      staggerOperation(request, itemsToDelete, null, (i) => {
        const item = itemsToDelete[i]
        if (item.kind === 'page') deletePages({ pageIds: [item.id] })
        else if (item.kind === 'text') deleteTextEntity(item.id)
        else if (item.kind === 'file') deleteFileEntity(item.id)
      })
    },
  },
]
