import { selectedCanvasTargets as uiSelectedCanvasTargets } from '../ui-state'
import { getEntityKind } from '../entities/contract'
import { deleteEdges } from '../workspace-edges'
import { deletePages } from '../workspace-entities'
import { deleteGroups } from '../workspace-groups'
import { requestLayout } from './viewport-control'
import { interactivePageId } from './runtime-context'

export function deleteSelection(): void {
  const targets = uiSelectedCanvasTargets()
  if (!targets.length) return

  // Guard (#124): an entered/focused page owns its web content. Delete fires
  // even from page focus (firesFromPageFocus), so drop the interactive page —
  // the user must exit to selected-only before Delete removes the frame.
  const enteredPageId = interactivePageId()

  const edgeIds: string[] = []
  const pageIds: string[] = []
  const groupIds: string[] = []
  // text/file/drawing/shape delete identically through their registry handler
  // (the same per-kind command this used to bucket). Page and group keep their
  // batch entry points — page for the single-pass WebContentsView close + edge/
  // empty-group cleanup, group for member-page dissolution — and edges are not
  // a registered kind.
  const entityTargets: { kind: 'text' | 'file' | 'drawing' | 'shape'; id: string }[] = []

  for (const target of targets) {
    switch (target.kind) {
      case 'edge':
        edgeIds.push(target.id)
        break
      case 'page':
        if (target.id === enteredPageId) break
        pageIds.push(target.id)
        break
      case 'group':
        groupIds.push(target.id)
        break
      case 'text':
      case 'file':
      case 'drawing':
      case 'shape':
        entityTargets.push({ kind: target.kind, id: target.id })
        break
    }
  }

  if (edgeIds.length) deleteEdges({ edgeIds })
  if (pageIds.length) deletePages({ pageIds })
  for (const { kind, id } of entityTargets) getEntityKind(kind).delete(id, {})
  if (groupIds.length) deleteGroups({ groupIds })

  const deletedEntityCount = pageIds.length + entityTargets.length + groupIds.length
  if (!deletedEntityCount) requestLayout()
}
