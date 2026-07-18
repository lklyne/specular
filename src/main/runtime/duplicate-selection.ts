import {
  entityKindById,
  groupBoundsForEntityIds,
} from '../workspace-entities'
import { duplicateGroup } from '../workspace-groups'
import { findDuplicatePlacement } from '../workspace-placement'
import { duplicatePageFromSource } from '../workspace-pages'
import {
  copyableSelectionPayload,
  pasteEntitiesFromClipboard,
} from '../workspace-clipboard'
import { getSelectedEntityIds } from './runtime-core'

export function duplicateSelection(): void {
  const entityIds = getSelectedEntityIds()
  if (!entityIds.length) return

  if (entityIds.length === 1) {
    const id = entityIds[0]
    const kind = entityKindById(id)
    if (kind === 'group') {
      duplicateGroup({ groupId: id, focus: true })
      return
    }
    if (kind === 'page') {
      duplicatePageFromSource({ sourcePageId: id, focus: true })
      return
    }
  }

  // Text/file/shape/drawing entities (single or multi-selected) all clone
  // through copy/paste machinery, so duplicates retain their relative
  // layout and cmd-D behaves like copy + paste-in-place with the usual
  // duplicate offset.
  const payload = copyableSelectionPayload()
  if (!payload) return
  const bounds = groupBoundsForEntityIds(entityIds)
  if (!bounds) return
  const placement = findDuplicatePlacement(bounds)
  pasteEntitiesFromClipboard({
    payload,
    canvasX: placement.canvasX,
    canvasY: placement.canvasY,
  })
}
