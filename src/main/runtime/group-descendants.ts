// fallow-ignore-file circular-dependencies
// Suppressed: see #141. page-runtime → runtime-core → selection-controller import back
import { drawingEntities } from './drawing-entity-state'
import { fileEntities } from './file-entity-state'
import { pages } from './page-runtime'
import { shapeEntities } from './shape-entity-state'
import { textEntities } from './text-entity-state'
import { workspaceGroups } from './workspace-model'

export function descendantEntityIdsForGroup(groupId: string): string[] {
  const ids: string[] = []

  const visit = (parentId: string) => {
    const childGroupIds = workspaceGroups
      .filter((group) => group.parentGroupId === parentId)
      .map((group) => group.id)

    ids.push(
      ...pages.filter((page) => page.parentGroupId === parentId).map((page) => page.id),
      ...textEntities.filter((entity) => entity.parentGroupId === parentId).map((entity) => entity.id),
      ...fileEntities.filter((entity) => entity.parentGroupId === parentId).map((entity) => entity.id),
      ...drawingEntities.filter((entity) => entity.parentGroupId === parentId).map((entity) => entity.id),
      ...shapeEntities.filter((entity) => entity.parentGroupId === parentId).map((entity) => entity.id),
      ...childGroupIds,
    )

    childGroupIds.forEach(visit)
  }

  visit(groupId)
  return ids
}

