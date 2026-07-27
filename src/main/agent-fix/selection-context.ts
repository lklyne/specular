/**
 * Resolve a selection annotation's context bundle: what the user had selected
 * when they commented, and what feedback already sits on those items.
 *
 * Membership is what the user picked (a selected group standing in for its
 * descendants), never everything the region happens to overlap — the region is
 * only the frame the comment was drawn over.
 */

import type { Annotation } from '../../shared/types'
import { isUnresolved } from '../../shared/annotation-utils'
import {
  entityBoundsById,
  entityKindById,
  expandSelectedGroups,
  groupById,
} from '../workspace-entities'
import { getAnnotations } from '../workspace-annotations'
import { textEntities } from '../runtime/text-entity-state'
import { drawingEntities } from '../runtime/drawing-entity-state'
import { shapeEntities } from '../runtime/shape-entity-state'
import { fileEntities } from '../runtime/file-entity-state'
import { findPageById } from '../runtime/page-runtime'
import { pageDisplayLabel } from '../runtime/runtime-serialization'
import type {
  PriorFeedbackSummary,
  SelectionMemberSummary,
  SelectionPromptContext,
} from './prompt-builder'

function describeMember(entityId: string): SelectionMemberSummary | null {
  const kind = entityKindById(entityId)
  if (!kind) return null
  const bounds = entityBoundsById(entityId) ?? undefined
  const base = { id: entityId, kind, bounds }
  switch (kind) {
    case 'text': {
      const entity = textEntities.find((e) => e.id === entityId)
      return { ...base, text: entity?.text, textStyle: entity?.textStyle, label: entity?.label }
    }
    case 'drawing': {
      const entity = drawingEntities.find((e) => e.id === entityId)
      return { ...base, label: entity?.label }
    }
    case 'shape': {
      const entity = shapeEntities.find((e) => e.id === entityId)
      return { ...base, shapeKind: entity?.shapeKind, text: entity?.text, label: entity?.label }
    }
    case 'file': {
      const entity = fileEntities.find((e) => e.id === entityId)
      return { ...base, filePath: entity?.file }
    }
    case 'page': {
      const page = findPageById(entityId)
      if (!page) return base
      return { ...base, url: page.url, pageName: pageDisplayLabel(page) }
    }
    case 'group':
      return { ...base, label: groupById(entityId)?.label }
    default:
      return base
  }
}

/** Selector / component / element name for a comment already on a page. */
function elementDescription(annotation: Annotation): string | undefined {
  const inspect = annotation.metadata?.inspectContext
  if (inspect?.name) return inspect.name
  if (inspect?.elementPath) return inspect.elementPath
  if (annotation.anchor.type === 'element') return annotation.anchor.selector
  return undefined
}

function priorFeedbackFor(annotation: Annotation, memberIds: Set<string>): PriorFeedbackSummary[] {
  return getAnnotations()
    .filter((candidate) => {
      if (candidate.id === annotation.id) return false
      if (!isUnresolved(candidate.status)) return false
      const anchorPageId =
        candidate.anchor.type === 'element' || candidate.anchor.type === 'page'
          ? candidate.anchor.pageId
          : undefined
      const boundPageId = candidate.pageAnchor?.pageId
      return (
        (anchorPageId != null && memberIds.has(anchorPageId)) ||
        (boundPageId != null && memberIds.has(boundPageId))
      )
    })
    .map((candidate) => ({
      text: candidate.text,
      status: candidate.status,
      element: elementDescription(candidate),
      pageName: candidate.metadata?.pageName,
    }))
}

export function resolveSelectionContext(annotation: Annotation): SelectionPromptContext | null {
  const selectionEntityIds = annotation.metadata?.selectionEntityIds
  if (!selectionEntityIds?.length) return null
  const memberIds = expandSelectedGroups(selectionEntityIds, { keepGroupId: true })
  return {
    members: memberIds
      .map(describeMember)
      .filter((member): member is SelectionMemberSummary => member !== null),
    priorFeedback: priorFeedbackFor(annotation, new Set(memberIds)),
  }
}
