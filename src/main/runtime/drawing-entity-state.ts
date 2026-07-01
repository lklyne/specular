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
  PersistedDrawingEntity,
} from '../../shared/types'
import { markDirty } from './layout-dirty'

export interface DrawingEntity {
  id: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  strokes: AnnotationDrawingStroke[]
  parentGroupId?: string
  label?: string
}

import { DRAWING_FEATURE_ENABLED } from '../../shared/featureFlags'

export const drawingEntities: DrawingEntity[] = []

export function drawingEntitiesForUi(): DrawingEntity[] {
  return DRAWING_FEATURE_ENABLED ? drawingEntities : []
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
  if (patch.canvasX !== undefined) entity.canvasX = patch.canvasX
  if (patch.canvasY !== undefined) entity.canvasY = patch.canvasY
  if (patch.width !== undefined) entity.width = patch.width
  if (patch.height !== undefined) entity.height = patch.height
  if (patch.strokes !== undefined) entity.strokes = patch.strokes
  if (patch.parentGroupId !== undefined) entity.parentGroupId = patch.parentGroupId
  if (patch.label !== undefined) entity.label = patch.label || undefined
  markDirty('canvas', 'sidebar')
  return entity
}

export function clearDrawingEntities(): void {
  drawingEntities.length = 0
}

export function buildDrawingEntitySceneEntity(entity: DrawingEntity): CanvasSceneDrawingEntity {
  return {
    kind: 'drawing',
    id: entity.id,
    canvasX: entity.canvasX,
    canvasY: entity.canvasY,
    width: entity.width,
    height: entity.height,
    strokes: entity.strokes,
    parentGroupId: entity.parentGroupId,
  }
}

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
  }
}
