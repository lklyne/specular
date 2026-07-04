/**
 * `group` entity kind — a container that owns a set of child entities.
 *
 * A group is created *around* existing entities, so a headless create reads
 * `entityIds` (and an optional `label`) rather than a position/size; the
 * group's geometry is derived from its children.
 */

import type { PersistedGroupEntity, WorkspaceGroup } from '../../../shared/types'
import type { JsonCanvasGroupNode } from '../../../shared/json-canvas-types'
import { createUserGroup } from '../../workspace-groups'
import { deleteGroupEntity, updateGroupEntity } from '../../runtime/document-commands'
import { WORKSPACE_GROUP_PERSISTED_FIELDS } from '../../runtime/group-entity-state'
import { workspaceGroups } from '../../runtime/workspace-model'
import {
  deserializeGroupNodeToGroup,
  serializeGroupEntityToGroupNode,
} from '../../runtime/json-canvas-serializer'
import type { EntityKindDefinition } from '../contract'

const DEFAULT_GROUP_SIZE = 200

export const groupKind: EntityKindDefinition<'group'> = {
  kind: 'group',
  fields: WORKSPACE_GROUP_PERSISTED_FIELDS,

  create(input) {
    const entityIds = (input.entityIds as string[] | undefined) ?? []
    const group = createUserGroup(entityIds, input.label as string | undefined)
    return group.id
  },

  update(id, patch) {
    updateGroupEntity(id, {
      canvasX: patch.canvasX as number | undefined,
      canvasY: patch.canvasY as number | undefined,
      width: patch.width as number | undefined,
      height: patch.height as number | undefined,
      label: patch.label as string | undefined,
      color: patch.color as string | undefined,
    })
  },

  // ponytail: removes the group container only; children keep their geometry
  // and un-parent. Deleting members is `delete <childId…>`, not this.
  delete(id) {
    return deleteGroupEntity(id)
  },

  serialize(entity) {
    return serializeGroupEntityToGroupNode(entity as PersistedGroupEntity)
  },

  deserialize(node) {
    return deserializeGroupNodeToGroup(node as JsonCanvasGroupNode)
  },

  defaultSize() {
    return { width: DEFAULT_GROUP_SIZE, height: DEFAULT_GROUP_SIZE }
  },

  entities: () => workspaceGroups,

  restore(snapshots) {
    workspaceGroups.length = 0
    for (const snapshot of snapshots) {
      workspaceGroups.push(snapshot as unknown as WorkspaceGroup)
    }
  },
}
