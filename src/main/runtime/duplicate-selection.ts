import {
  entityKindById,
  groupBoundsForEntityIds,
} from '../workspace-entities'
import { duplicateGroup } from '../workspace-groups'
import { findDuplicatePlacement } from '../workspace-placement'
import {
  copyableSelectionPayload,
  pasteEntitiesFromClipboard,
} from '../workspace-clipboard'
import { getSelectedEntityIds } from './runtime-core'

export function duplicateSelection(): void {
  const entityIds = getSelectedEntityIds()
  if (!entityIds.length) return

  if (entityIds.length === 1 && entityKindById(entityIds[0]) === 'group') {
    duplicateGroup({ groupId: entityIds[0], focus: true })
    return
  }

  // Everything else (pages, text/file/shape/drawing — single or
  // multi-selected) clones through copy/paste machinery, so duplicates
  // retain their relative layout, a page carries its anchored items (which
  // re-attach to the cloned page), and cmd-D behaves like copy +
  // paste-in-place with the usual duplicate offset.
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
