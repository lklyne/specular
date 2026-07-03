/**
 * `drawing` entity kind — freeform ink captured as a bounded set of strokes.
 */

import type { AnnotationDrawingStroke, PersistedDrawingEntity } from '../../../shared/types'
import type { JsonCanvasDrawingNode } from '../../../shared/json-canvas-types'
import {
  createDrawingEntity,
  deleteDrawingEntity,
  updateDrawingEntity,
} from '../../runtime/document-commands'
import {
  buildDrawingEntitySceneEntity,
  drawingEntities,
  persistDrawingEntity,
  type DrawingEntity,
} from '../../runtime/drawing-entity-state'
import {
  deserializeDrawingNodeToDrawing,
  serializeDrawingToDrawingNode,
} from '../../runtime/json-canvas-serializer'
import type { EntityKindDefinition } from '../contract'

const DEFAULT_DRAWING_SIZE = 200

export const drawingKind: EntityKindDefinition<'drawing'> = {
  kind: 'drawing',
  fields: ['canvasX', 'canvasY', 'width', 'height', 'strokes'],

  create(input) {
    const entity = createDrawingEntity({
      id: input.id as string | undefined,
      canvasX: (input.canvasX as number | undefined) ?? 0,
      canvasY: (input.canvasY as number | undefined) ?? 0,
      width: (input.width as number | undefined) ?? DEFAULT_DRAWING_SIZE,
      height: (input.height as number | undefined) ?? DEFAULT_DRAWING_SIZE,
      strokes: (input.strokes as AnnotationDrawingStroke[] | undefined) ?? [],
    })
    return entity.id
  },

  update(id, patch) {
    updateDrawingEntity(id, {
      canvasX: patch.canvasX as number | undefined,
      canvasY: patch.canvasY as number | undefined,
      width: patch.width as number | undefined,
      height: patch.height as number | undefined,
      strokes: patch.strokes as AnnotationDrawingStroke[] | undefined,
    })
  },

  delete(id) {
    return deleteDrawingEntity(id)
  },

  serialize(entity) {
    return serializeDrawingToDrawingNode(entity as PersistedDrawingEntity)
  },

  deserialize(node) {
    return deserializeDrawingNodeToDrawing(node as JsonCanvasDrawingNode)
  },

  defaultSize() {
    return { width: DEFAULT_DRAWING_SIZE, height: DEFAULT_DRAWING_SIZE }
  },

  // The raw store — persistence and stack order read this. The sidebar and
  // canvas scene read the UI-filtered `drawingEntitiesForUi()` view instead.
  entities: () => drawingEntities,

  buildSceneEntity: (entity, zoom, pan, origin) =>
    buildDrawingEntitySceneEntity(entity as DrawingEntity, zoom, pan, origin),

  persist: (entity) => persistDrawingEntity(entity as DrawingEntity),
}
