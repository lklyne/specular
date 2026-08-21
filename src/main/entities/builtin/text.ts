/**
 * `text` entity kind — a sticky/plain text note on the canvas (ADR 0004).
 *
 * Long or structured text does NOT create a `text` entity — the apply path
 * routes it to the `file` kind (see `file.ts#claimsAsNote`), matching the
 * legacy `entity-ops.ts` auto-route.
 */

import type { PersistedTextEntity, TextEntityStyle, TextWidthMode } from '../../../shared/types'
import type { JsonCanvasTextNode } from '../../../shared/json-canvas-types'
import {
  createTextEntity,
  deleteTextEntity,
  updateTextEntity,
} from '../../runtime/document-commands'
import {
  buildTextEntitySceneEntity,
  persistTextEntity,
  textEntities,
  TEXT_ENTITY_PERSISTED_FIELDS,
  type TextEntity,
} from '../../runtime/text-entity-state'
import {
  deserializeTextNodeToText,
  serializeTextToTextNode,
} from '../../runtime/json-canvas-serializer'
import type { EntityKindDefinition } from '../contract'

const DEFAULT_TEXT_SIZE = 200

export const textKind: EntityKindDefinition<'text'> = {
  kind: 'text',
  fields: TEXT_ENTITY_PERSISTED_FIELDS,

  create(input) {
    const entity = createTextEntity({
      id: input.id as string | undefined,
      canvasX: (input.canvasX as number | undefined) ?? 0,
      canvasY: (input.canvasY as number | undefined) ?? 0,
      text: input.text as string | undefined,
      color: input.color as string | undefined,
      textStyle: input.textStyle as TextEntityStyle | undefined,
      widthMode: input.widthMode as TextWidthMode | undefined,
      textSize: input.textSize as number | undefined,
      width: input.width as number | undefined,
      height: input.height as number | undefined,
    })
    return entity.id
  },

  update(id, patch) {
    // Forward the whole patch — `updateTextEntity` already copies every field
    // in `TEXT_ENTITY_PERSISTED_FIELDS` (minus id/kind), so hand-picking a
    // subset here would just be a second field list that can drift from the
    // first (docs/plans/entity-field-drift.md, Step C).
    updateTextEntity(id, patch as Partial<Omit<TextEntity, 'id'>>)
  },

  delete(id) {
    return deleteTextEntity(id)
  },

  serialize(entity) {
    return serializeTextToTextNode(entity as PersistedTextEntity)
  },

  deserialize(node) {
    return deserializeTextNodeToText(node as JsonCanvasTextNode)
  },

  defaultSize() {
    return { width: DEFAULT_TEXT_SIZE, height: DEFAULT_TEXT_SIZE }
  },

  entities: () => textEntities,

  restore(snapshots) {
    textEntities.length = 0
    for (const snapshot of snapshots) {
      textEntities.push(snapshot as unknown as TextEntity)
    }
  },

  buildSceneEntity: (entity, zoom, pan, origin) =>
    buildTextEntitySceneEntity(entity as TextEntity, zoom, pan, origin),

  persist: (entity) => persistTextEntity(entity as TextEntity),
}
