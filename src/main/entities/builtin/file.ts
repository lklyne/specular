/**
 * `file` entity kind — a file-on-disk projected onto the canvas (md /
 * wireframe / html / image / video; the renderer is picked by the
 * entity-renderer registry).
 *
 * This kind also claims long or structured *text*: the apply path routes a
 * `text` create whose content is long or markdown-ish to a `.md` note file
 * here, via `claimsAsNote`. This is the auto-route that used to live in
 * `entity-ops.ts` (`shouldRouteToFile` / `noteCreates`).
 */

import type { EntityCreateInput, EntityKindDefinition } from '../contract'
import type { PersistedFileEntity } from '../../../shared/types'
import type { JsonCanvasFileNode } from '../../../shared/json-canvas-types'
import {
  createFileEntity,
  deleteFileEntity,
  updateFileEntity,
} from '../../runtime/document-commands'
import {
  buildFileEntitySceneEntity,
  fileEntities,
  persistFileEntity,
  FILE_ENTITY_PERSISTED_FIELDS,
  type FileEntity,
} from '../../runtime/file-entity-state'
import { createNoteFile } from '../../runtime/note-assets'
import {
  htmlDefaultSize,
  imageSizeFromPath,
  videoSizeFromPath,
} from '../../runtime/image-sizing'
import {
  deserializeFileNodeToFile,
  serializeFileToFileNode,
} from '../../runtime/json-canvas-serializer'

const DEFAULT_FILE_SIZE = 200
const DEFAULT_NOTE_SIZE = 400
const LONG_TEXT_THRESHOLD = 300

/** A create item that should become a `.md` note rather than a `text` entity. */
function isNoteInput(input: EntityCreateInput): boolean {
  return typeof input.text === 'string' && input.file === undefined
}

function shouldRouteToFile(text: string): boolean {
  if (text.length > LONG_TEXT_THRESHOLD) return true
  return /^#{1,6}\s/m.test(text)
    || /^\|.+\|/m.test(text)
    || /```/.test(text)
}

function deriveNoteName(text: string): string {
  const heading = text.match(/^#{1,6}\s+(.+)/m)
  if (heading) return heading[1].slice(0, 60)
  return text.split('\n')[0].trim().slice(0, 60) || 'Note'
}

/**
 * Whether the `file` kind claims a `text` create. Mirrors the legacy
 * `entity-ops.ts` decision: long/structured content (or an explicit
 * `_forceFile`) becomes a note, unless `forceKind` pins it to a text entity.
 */
export function claimsAsNote(input: EntityCreateInput): boolean {
  return !input.id
    && input.kind === 'text'
    && !input.forceKind
    && typeof input.text === 'string'
    && (Boolean(input._forceFile) || shouldRouteToFile(input.text))
}

function resolveFileDimensions(file: string, width?: number, height?: number) {
  if (width != null && height != null) return { width, height }
  const detected = imageSizeFromPath(file) ?? videoSizeFromPath(file) ?? htmlDefaultSize(file)
  return detected ?? { width, height }
}

export const fileKind: EntityKindDefinition<'file'> = {
  kind: 'file',
  fields: FILE_ENTITY_PERSISTED_FIELDS,

  create(input) {
    const canvasX = (input.canvasX as number | undefined) ?? 0
    const canvasY = (input.canvasY as number | undefined) ?? 0

    if (isNoteInput(input)) {
      const text = input.text as string
      const filePath = createNoteFile(deriveNoteName(text), text)
      const entity = createFileEntity({
        canvasX,
        canvasY,
        file: filePath,
        width: (input.width as number | undefined) ?? DEFAULT_NOTE_SIZE,
        height: (input.height as number | undefined) ?? DEFAULT_NOTE_SIZE,
      })
      return entity.id
    }

    const dims = resolveFileDimensions(
      input.file as string,
      input.width as number | undefined,
      input.height as number | undefined,
    )
    const entity = createFileEntity({
      id: input.id as string | undefined,
      canvasX,
      canvasY,
      file: input.file as string,
      subpath: input.subpath as string | undefined,
      width: dims.width,
      height: dims.height,
    })
    return entity.id
  },

  update(id, patch) {
    updateFileEntity(id, {
      file: patch.file as string | undefined,
      subpath: patch.subpath as string | undefined,
      width: patch.width as number | undefined,
      height: patch.height as number | undefined,
      canvasX: patch.canvasX as number | undefined,
      canvasY: patch.canvasY as number | undefined,
    })
  },

  delete(id) {
    return deleteFileEntity(id)
  },

  serialize(entity) {
    return serializeFileToFileNode(entity as PersistedFileEntity)
  },

  deserialize(node) {
    return deserializeFileNodeToFile(node as JsonCanvasFileNode)
  },

  defaultSize(input) {
    const size = isNoteInput(input) ? DEFAULT_NOTE_SIZE : DEFAULT_FILE_SIZE
    return { width: size, height: size }
  },

  entities: () => fileEntities,

  restore(snapshots) {
    fileEntities.length = 0
    for (const snapshot of snapshots) {
      fileEntities.push(snapshot as unknown as FileEntity)
    }
  },

  buildSceneEntity: (entity, zoom, pan, origin) =>
    buildFileEntitySceneEntity(entity as FileEntity, zoom, pan, origin),

  persist: (entity) => persistFileEntity(entity as FileEntity),
}
