/**
 * `shape` entity kind — a vector primitive (rectangle / ellipse / diamond)
 * with an optional inner label.
 */

import type { PersistedShapeEntity, ShapeKind } from '../../../shared/types'
import type { JsonCanvasShapeNode } from '../../../shared/json-canvas-types'
import {
  createShapeEntity,
  deleteShapeEntity,
  updateShapeEntity,
} from '../../runtime/document-commands'
import {
  buildShapeEntitySceneEntity,
  DEFAULT_SHAPE_HEIGHT,
  DEFAULT_SHAPE_WIDTH,
  persistShapeEntity,
  shapeEntities,
  type ShapeEntity,
} from '../../runtime/shape-entity-state'
import {
  deserializeShapeNodeToShape,
  serializeShapeToShapeNode,
} from '../../runtime/json-canvas-serializer'
import type { EntityKindDefinition } from '../contract'

export const shapeKind: EntityKindDefinition<'shape'> = {
  kind: 'shape',
  fields: ['canvasX', 'canvasY', 'width', 'height', 'shapeKind', 'text', 'color', 'strokeWidth', 'textSize'],

  create(input) {
    const entity = createShapeEntity({
      id: input.id as string | undefined,
      canvasX: (input.canvasX as number | undefined) ?? 0,
      canvasY: (input.canvasY as number | undefined) ?? 0,
      shapeKind: input.shapeKind as ShapeKind | undefined,
      width: input.width as number | undefined,
      height: input.height as number | undefined,
      text: input.text as string | undefined,
      color: input.color as string | undefined,
      strokeWidth: input.strokeWidth as number | undefined,
      textSize: input.textSize as number | undefined,
    })
    return entity.id
  },

  update(id, patch) {
    updateShapeEntity(id, {
      canvasX: patch.canvasX as number | undefined,
      canvasY: patch.canvasY as number | undefined,
      width: patch.width as number | undefined,
      height: patch.height as number | undefined,
      shapeKind: patch.shapeKind as ShapeKind | undefined,
      text: patch.text as string | undefined,
      color: patch.color as string | undefined,
      strokeWidth: patch.strokeWidth as number | undefined,
      textSize: patch.textSize as number | undefined,
    })
  },

  delete(id) {
    return deleteShapeEntity(id)
  },

  serialize(entity) {
    return serializeShapeToShapeNode(entity as PersistedShapeEntity)
  },

  deserialize(node) {
    return deserializeShapeNodeToShape(node as JsonCanvasShapeNode)
  },

  defaultSize() {
    return { width: DEFAULT_SHAPE_WIDTH, height: DEFAULT_SHAPE_HEIGHT }
  },

  entities: () => shapeEntities,

  buildSceneEntity: (entity, zoom, pan, origin) =>
    buildShapeEntitySceneEntity(entity as ShapeEntity, zoom, pan, origin),

  persist: (entity) => persistShapeEntity(entity as ShapeEntity),
}
