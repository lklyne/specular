/**
 * Annotate a multi-selection: one region annotation over the selection's union
 * bounds, carrying the selected entity ids and the artifact the request is
 * about. The single main-side door — the HTTP route, the CLI verb, and the
 * selection popup's Annotate button all land here (ADR 0019).
 */

import { isAbsolute, resolve } from 'path'
import type { Annotation, AnnotationSelectionTarget } from '../../shared/types'
import { entityKindById, groupBoundsForEntityIds, groupDescendantIds } from '../workspace-entities'
import { selectedEntityIds } from '../ui-state'
import { fileEntities } from './file-entity-state'
import { findPageById } from './page-runtime'
import { executeRegionSelect } from './region-select'

export interface AnnotateSelectionInput {
  /** Omitted or empty → the current canvas selection. */
  entityIds?: string[]
  text: string
}

/**
 * The ids the target is derived from: the selection with every selected group
 * replaced by its descendants, so selecting the group that holds one page
 * reads the same as selecting that page.
 */
function targetCandidateIds(entityIds: string[]): string[] {
  const expanded: string[] = []
  for (const entityId of entityIds) {
    if (entityKindById(entityId) === 'group') {
      expanded.push(...groupDescendantIds(entityId))
    } else {
      expanded.push(entityId)
    }
  }
  return [...new Set(expanded)]
}

/** File entity paths are stored as written; a relative one resolves against cwd. */
function fileEntityPath(file: string): string {
  return isAbsolute(file) ? file : resolve(file)
}

/**
 * Exactly one page in the selection → that page; else exactly one file entity →
 * that file; else no target (the request spans several artifacts, so naming one
 * would be a guess).
 */
export function selectionTargetFor(entityIds: string[]): AnnotationSelectionTarget | undefined {
  const candidates = targetCandidateIds(entityIds)
  const pageIds = candidates.filter((id) => entityKindById(id) === 'page')
  if (pageIds.length === 1) {
    const page = findPageById(pageIds[0])
    return {
      entityId: pageIds[0],
      kind: 'page',
      ...(page?.url ? { url: page.url } : {}),
    }
  }
  if (pageIds.length > 1) return undefined

  const fileIds = candidates.filter((id) => entityKindById(id) === 'file')
  if (fileIds.length !== 1) return undefined
  const entity = fileEntities.find((e) => e.id === fileIds[0])
  return {
    entityId: fileIds[0],
    kind: 'file',
    ...(entity?.file ? { filePath: fileEntityPath(entity.file) } : {}),
  }
}

export async function annotateSelectionRegion(
  input: AnnotateSelectionInput,
): Promise<Annotation> {
  const entityIds = input.entityIds?.length ? [...new Set(input.entityIds)] : selectedEntityIds()
  if (!entityIds.length) {
    throw new Error('No entities selected')
  }
  const canvasRect = groupBoundsForEntityIds(entityIds)
  if (!canvasRect) {
    throw new Error('Selection has no bounds')
  }
  const selectionTarget = selectionTargetFor(entityIds)
  return executeRegionSelect(canvasRect, input.text, {
    metadata: {
      selectionEntityIds: entityIds,
      ...(selectionTarget ? { selectionTarget } : {}),
    },
  })
}
