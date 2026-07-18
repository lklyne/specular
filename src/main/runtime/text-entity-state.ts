/**
 * Text Entity State
 *
 * Manages the in-memory state of text entities on the canvas.
 * Text entities are lightweight canvas entities — no Electron views,
 * no browser runtime, no devtools. They just have position, size,
 * text, and color.
 */

import { randomUUID } from 'crypto'
import type {
  CanvasSceneTextEntity,
  PageAnchor,
  PersistedTextEntity,
  TextEntityStyle,
  TextWidthMode,
} from '../../shared/types'
import { markDirty } from './layout-dirty'
import { applyPatch } from './apply-patch'
import { pageAnchorScrollShift, pageAnchorElementShift } from './page-anchor-scroll'

export interface TextEntity {
  id: string
  text: string
  color: string
  textStyle: TextEntityStyle
  widthMode: TextWidthMode
  /** Per-entity text size in px. Optional — renderer defaults to 18. ADR 0013 §2. */
  textSize?: number
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  label?: string
  /** Present when the entity is hooked to a page (see shared/page-anchor.ts). */
  pageAnchor?: PageAnchor
}

/** Plain text starts auto-sized; sticky is always fixed. */
export function defaultWidthMode(textStyle: TextEntityStyle): TextWidthMode {
  return textStyle === 'plain' ? 'auto' : 'fixed'
}

export const DEFAULT_TEXT_WIDTH = 200
export const DEFAULT_TEXT_HEIGHT = 200

export const textEntities: TextEntity[] = []

export function createTextEntity(input: {
  canvasX: number
  canvasY: number
  text?: string
  color?: string
  textStyle?: TextEntityStyle
  widthMode?: TextWidthMode
  textSize?: number
  width?: number
  height?: number
  id?: string
  parentGroupId?: string
  label?: string
  pageAnchor?: PageAnchor
}): TextEntity {
  const textStyle = input.textStyle ?? 'sticky'
  const entity: TextEntity = {
    id: input.id ?? `text_${randomUUID()}`,
    text: input.text ?? '',
    // Color is stored raw — a preset number ('1'–'7'), the 'neutral'
    // sentinel, or a literal hex. The palette is resolved at render time.
    color: input.color ?? '3',
    textStyle,
    widthMode: input.widthMode ?? defaultWidthMode(textStyle),
    textSize: input.textSize,
    canvasX: input.canvasX,
    canvasY: input.canvasY,
    width: input.width ?? DEFAULT_TEXT_WIDTH,
    height: input.height ?? DEFAULT_TEXT_HEIGHT,
    parentGroupId: input.parentGroupId,
    label: input.label,
    pageAnchor: input.pageAnchor,
  }
  textEntities.push(entity)
  markDirty('canvas', 'sidebar')
  return entity
}

export function updateTextEntity(id: string, patch: Partial<Omit<TextEntity, 'id'>>): TextEntity | null {
  const entity = textEntities.find((n) => n.id === id)
  if (!entity) return null
  applyPatch(entity, patch, [
    'text', 'color', 'textStyle', 'widthMode', 'textSize',
    'canvasX', 'canvasY', 'width', 'height', 'parentGroupId', 'pageAnchor',
  ])
  if (patch.label !== undefined) entity.label = patch.label || undefined
  markDirty('canvas', 'sidebar')
  return entity
}

export function deleteTextEntity(id: string): boolean {
  const idx = textEntities.findIndex((n) => n.id === id)
  if (idx === -1) return false
  textEntities.splice(idx, 1)
  markDirty('canvas', 'sidebar')
  return true
}

export function clearTextEntities(): void {
  textEntities.length = 0
}

export function buildTextEntitySceneEntity(
  entity: TextEntity,
  zoom: number,
  pan: { x: number; y: number },
  canvasOrigin: { x: number; y: number },
): CanvasSceneTextEntity {
  // Scroll- and element-follow: project the apparent position — stored coords
  // shifted by the page's scroll and its reference element's movement since the
  // anchor was written (see shape builder).
  const scroll = pageAnchorScrollShift(entity.pageAnchor)
  const element = pageAnchorElementShift(entity.pageAnchor)
  const canvasX = entity.canvasX - scroll.x - element.x
  const canvasY = entity.canvasY - scroll.y - element.y
  const screenX = canvasOrigin.x + canvasX * zoom + pan.x
  const screenY = canvasOrigin.y + canvasY * zoom + pan.y
  return {
    kind: 'text',
    id: entity.id,
    text: entity.text,
    color: entity.color,
    textStyle: entity.textStyle,
    widthMode: entity.widthMode,
    textSize: entity.textSize,
    canvasX,
    canvasY,
    width: entity.width,
    height: entity.height,
    parentGroupId: entity.parentGroupId,
    pageAnchor: entity.pageAnchor,
    screenX,
    screenY,
    screenWidth: entity.width * zoom,
    screenHeight: entity.height * zoom,
  }
}

/**
 * Every key `persistTextEntity` writes to the doc's entity map — the single
 * field list both sync directions derive from (ADR 0024 §5). `satisfies`
 * keeps the set exhaustive against `PersistedTextEntity`; the persisted-fields
 * drift test keeps `persistTextEntity` on it.
 */
const TEXT_ENTITY_PERSISTED_FIELD_SET = {
  kind: true,
  id: true,
  text: true,
  color: true,
  textStyle: true,
  widthMode: true,
  textSize: true,
  canvasX: true,
  canvasY: true,
  width: true,
  height: true,
  parentGroupId: true,
  label: true,
  pageAnchor: true,
} as const satisfies Record<keyof PersistedTextEntity, true>

export const TEXT_ENTITY_PERSISTED_FIELDS: readonly string[] = Object.keys(
  TEXT_ENTITY_PERSISTED_FIELD_SET,
)

export function persistTextEntity(entity: TextEntity): PersistedTextEntity {
  return {
    kind: 'text',
    id: entity.id,
    text: entity.text,
    color: entity.color,
    textStyle: entity.textStyle,
    widthMode: entity.widthMode,
    textSize: entity.textSize,
    canvasX: entity.canvasX,
    canvasY: entity.canvasY,
    width: entity.width,
    height: entity.height,
    parentGroupId: entity.parentGroupId,
    label: entity.label,
    pageAnchor: entity.pageAnchor,
  }
}
