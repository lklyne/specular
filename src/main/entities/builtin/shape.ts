/**
 * `shape` entity kind — a vector primitive (rectangle / ellipse / diamond)
 * with an optional inner label.
 */

import type {
  PersistedShapeEntity,
  ShapeBorderStyle,
  ShapeFillStyle,
  ShapeKind,
  ShapeTextAlign,
  ShapeTextVerticalAlign,
} from '../../../shared/types'
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
  SHAPE_ENTITY_PERSISTED_FIELDS,
  type ShapeEntity,
} from '../../runtime/shape-entity-state'
import {
  deserializeShapeNodeToShape,
  serializeShapeToShapeNode,
} from '../../runtime/json-canvas-serializer'
import type { EntityKindDefinition } from '../contract'

export const shapeKind: EntityKindDefinition<'shape'> = {
  kind: 'shape',
  fields: SHAPE_ENTITY_PERSISTED_FIELDS,

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
      fillStyle: input.fillStyle as ShapeFillStyle | undefined,
      strokeWidth: input.strokeWidth as number | undefined,
      borderStyle: input.borderStyle as ShapeBorderStyle | undefined,
      borderColor: input.borderColor as string | undefined,
      textSize: input.textSize as number | undefined,
      textAlign: input.textAlign as ShapeTextAlign | undefined,
      textVerticalAlign: input.textVerticalAlign as ShapeTextVerticalAlign | undefined,
    })
    return entity.id
  },

  update(id, patch) {
    // Forward the whole patch — `updateShapeEntity` already copies every
    // field in `SHAPE_ENTITY_PERSISTED_FIELDS` (minus id/kind), so hand-picking
    // a subset here would just be a second field list that can drift from the
    // first (docs/plans/entity-field-drift.md, Step C).
    updateShapeEntity(id, patch as Partial<Omit<ShapeEntity, 'id'>>)
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

  restore(snapshots) {
    shapeEntities.length = 0
    for (const snapshot of snapshots) {
      shapeEntities.push(snapshot as unknown as ShapeEntity)
    }
  },

  buildSceneEntity: (entity, zoom, pan, origin) =>
    buildShapeEntitySceneEntity(entity as ShapeEntity, zoom, pan, origin),

  persist: (entity) => persistShapeEntity(entity as ShapeEntity),
}
