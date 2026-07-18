import type { PageAnchor } from '../../shared/page-anchor'
import { drawingEntities } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { textEntities } from './text-entity-state'

export interface AnchorableEntity {
  id: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  pageAnchor?: PageAnchor
}

export function anchorableEntities(): AnchorableEntity[] {
  return [...textEntities, ...drawingEntities, ...shapeEntities]
}

export function findAnchorableEntity(id: string): AnchorableEntity | undefined {
  return (
    textEntities.find((entity) => entity.id === id) ??
    drawingEntities.find((entity) => entity.id === id) ??
    shapeEntities.find((entity) => entity.id === id)
  )
}
