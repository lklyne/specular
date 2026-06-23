/**
 * `drawing` entity kind — freeform ink captured as a bounded set of strokes.
 */

import type { AnnotationDrawingStroke, PersistedDrawingEntity } from '../../../shared/types'
import {
  createDrawingEntity,
  deleteDrawingEntity,
  updateDrawingEntity,
} from '../../runtime/document-commands'
import { serializeDrawingToDrawingNode } from '../../runtime/json-canvas-serializer'
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

  defaultSize() {
    return { width: DEFAULT_DRAWING_SIZE, height: DEFAULT_DRAWING_SIZE }
  },
}
