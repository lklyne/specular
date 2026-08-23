/**
 * Drawing Entity State
 *
 * Manages in-memory state of drawing entities on the canvas.
 * Drawing entities are purely visual overlays — SVG strokes
 * positioned in canvas coordinates with no browser views.
 */

import { randomUUID } from 'crypto'
import type {
  AnnotationDrawingStroke,
  CanvasSceneDrawingEntity,
  PageAnchor,
  PersistedDrawingEntity,
} from '../../shared/types'
import { markDirty } from './layout-dirty'
import { applyPatch, patchableFields } from './apply-patch'
import { pageAnchorScrollShift, pageAnchorElementShift } from './page-anchor-scroll'

export interface DrawingEntity {
  id: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  strokes: AnnotationDrawingStroke[]
  parentGroupId?: string
  label?: string
  /** Present when the entity is hooked to a page (see shared/page-anchor.ts). */
  pageAnchor?: PageAnchor
}

export const drawingEntities: DrawingEntity[] = []

export function drawingEntitiesForUi(): DrawingEntity[] {
  return drawingEntities
}

export function createDrawingEntity(input: {
  canvasX: number
  canvasY: number
  width: number
  height: number
  strokes: AnnotationDrawingStroke[]
  id?: string
  parentGroupId?: string
  label?: string
  pageAnchor?: PageAnchor
}): DrawingEntity {
  const entity: DrawingEntity = {
    id: input.id ?? `drawing_${randomUUID()}`,
    canvasX: input.canvasX,
    canvasY: input.canvasY,
    width: input.width,
    height: input.height,
    strokes: input.strokes,
    parentGroupId: input.parentGroupId,
    label: input.label,
    pageAnchor: input.pageAnchor,
  }
  drawingEntities.push(entity)
  markDirty('canvas', 'sidebar')
  return entity
}

export function deleteDrawingEntity(id: string): boolean {
  const idx = drawingEntities.findIndex((d) => d.id === id)
  if (idx === -1) return false
  drawingEntities.splice(idx, 1)
  markDirty('canvas', 'sidebar')
  return true
}

export function updateDrawingEntity(
  id: string,
  patch: Partial<Omit<DrawingEntity, 'id'>>,
): DrawingEntity | null {
  const entity = drawingEntities.find((candidate) => candidate.id === id)
  if (!entity) return null
  applyPatch(entity, patch, DRAWING_ENTITY_PATCHABLE_FIELDS)
  if (patch.label !== undefined) entity.label = patch.label || undefined
  markDirty('canvas', 'sidebar')
  return entity
}

export function clearDrawingEntities(): void {
  drawingEntities.length = 0
}

export function buildDrawingEntitySceneEntity(
  entity: DrawingEntity,
): CanvasSceneDrawingEntity {
  // Scroll- and element-follow: project the apparent position — stored coords
  // shifted by the page's scroll and its reference element's movement since the
  // anchor was written (see shape builder). Strokes are absolute canvas coords,
  // so the combined shift applies to every point.
  const scroll = pageAnchorScrollShift(entity.pageAnchor)
  const element = pageAnchorElementShift(entity.pageAnchor)
  const shiftX = scroll.x + element.x
  const shiftY = scroll.y + element.y
  const canvasX = entity.canvasX - shiftX
  const canvasY = entity.canvasY - shiftY
  const strokes =
    shiftX || shiftY
      ? entity.strokes.map((stroke) => ({
          ...stroke,
          points: stroke.points.map((point) => ({ x: point.x - shiftX, y: point.y - shiftY })),
        }))
      : entity.strokes
  return {
    kind: 'drawing',
    id: entity.id,
    canvasX,
    canvasY,
    width: entity.width,
    height: entity.height,
    strokes,
    parentGroupId: entity.parentGroupId,
    pageAnchor: entity.pageAnchor,
  }
}

/**
 * Every key `persistDrawingEntity` writes to the doc's entity map — the single
 * field list both sync directions derive from (ADR 0024 §5). `satisfies`
 * keeps the set exhaustive against `PersistedDrawingEntity`; the
 * persisted-fields drift test keeps `persistDrawingEntity` on it.
 */
const DRAWING_ENTITY_PERSISTED_FIELD_SET = {
  kind: true,
  id: true,
  canvasX: true,
  canvasY: true,
  width: true,
  height: true,
  strokes: true,
  parentGroupId: true,
  label: true,
  pageAnchor: true,
} as const satisfies Record<keyof PersistedDrawingEntity, true>

export const DRAWING_ENTITY_PERSISTED_FIELDS: readonly string[] = Object.keys(
  DRAWING_ENTITY_PERSISTED_FIELD_SET,
)

/**
 * `updateDrawingEntity`'s field-copy list, derived from the same declaration
 * `persist()` uses. `label` is excluded — it needs the empty-string-to-undefined
 * normalization applied explicitly above, not `applyPatch`'s blind copy.
 */
const DRAWING_ENTITY_PATCHABLE_FIELDS = patchableFields<Omit<DrawingEntity, 'id'>>(
  DRAWING_ENTITY_PERSISTED_FIELDS,
  ['label'],
)

export function persistDrawingEntity(entity: DrawingEntity): PersistedDrawingEntity {
  return {
    kind: 'drawing',
    id: entity.id,
    canvasX: entity.canvasX,
    canvasY: entity.canvasY,
    width: entity.width,
    height: entity.height,
    strokes: entity.strokes,
    parentGroupId: entity.parentGroupId,
    label: entity.label,
    pageAnchor: entity.pageAnchor,
  }
}
