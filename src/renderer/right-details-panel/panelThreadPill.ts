import type { DevtoolsPanelData } from '../../shared/types'
import {
  resolveThreadPill,
  type ThreadPill,
  type ThreadPillInput,
  type ThreadWriteTarget,
} from '../../shared/agent-thread'
import { annotationOrigin } from '../../shared/annotation-utils'
import { fileEntityLabel } from './rightDetailsPanelHelpers'
import { getInspectDetailState } from './rightDetailsPanelSelectors'

export function threadPillFromPanelData(data: DevtoolsPanelData): ThreadPill {
  return resolveThreadPill(threadPillInputFromPanelData(data))
}

export function threadWriteTargetFromPanel(
  data: DevtoolsPanelData,
  pill: ThreadPill,
): ThreadWriteTarget {
  const origin = originFromPill(data, pill)
  if (!origin) return { kind: 'space' }
  const binding = data.originBindings?.[origin]
  if (binding) return { kind: 'repo', origin, repoPath: binding.repoPath }
  const inferred = (data.pages ?? []).find((page) => {
    try {
      return new URL(page.url).origin === origin && Boolean(page.boundRepoPath)
    } catch {
      return false
    }
  })?.boundRepoPath
  if (inferred) return { kind: 'repo', origin, repoPath: inferred }
  return { kind: 'space' }
}

function originFromPill(data: DevtoolsPanelData, pill: ThreadPill): string | null {
  if (pill.kind === 'dom') return pill.origin
  if (pill.kind === 'annotation') {
    const annotation = (data.annotations ?? []).find((item) => item.id === pill.annotationId)
    return annotation ? annotationOrigin(annotation) : null
  }
  const page = data.selection
  if (!page?.url) return null
  try {
    return new URL(page.url).origin
  } catch {
    return null
  }
}

/**
 * What the composer chip calls a single selected entity. Names it the way its
 * pane header does — a file by its filename, a group by its label, a note by
 * its opening words — and falls back to the kind for entities with no name.
 */
function selectedEntityLabel(data: DevtoolsPanelData, kind: string): string {
  switch (kind) {
    case 'file':
      return data.fileEntity ? fileEntityLabel(data.fileEntity.file) : kind
    case 'group':
      return data.groupEntity?.label.trim() || kind
    case 'text':
      return firstWords(data.textEntity?.text) || kind
    case 'shape':
      return firstWords(data.shapeEntity?.text) || kind
    default:
      return kind
  }
}

function firstWords(text: string | undefined): string {
  const flat = (text ?? '').replace(/\s+/g, ' ').trim()
  return flat.length > 40 ? `${flat.slice(0, 39)}…` : flat
}

function threadPillInputFromPanelData(data: DevtoolsPanelData): ThreadPillInput {
  const inspect = data.inspect
  const { selectedDetail } = inspect ? getInspectDetailState(inspect) : {}
  const page = data.pages?.find((item) => item.id === selectedDetail?.pageId)
  let origin: string | null = null
  if (page?.url) {
    try {
      origin = new URL(page.url).origin
    } catch {
      origin = null
    }
  }

  const focused = data.focusedAnnotationId
    ? (data.annotations ?? []).find((item) => item.id === data.focusedAnnotationId)
    : undefined

  const mode = data.panelMode
  let canvasSelection: ThreadPillInput['canvasSelection'] = null
  if (mode.kind === 'multi') {
    canvasSelection = {
      count: mode.entityIds.length,
      label: `${mode.entityIds.length} items`,
      entityIds: [...mode.entityIds],
    }
  } else if (mode.kind !== 'document' && mode.kind !== 'page') {
    canvasSelection = {
      count: 1,
      label: selectedEntityLabel(data, mode.kind),
      entityIds: [mode.entityId],
    }
  } else if (mode.kind === 'page') {
    canvasSelection = {
      count: 1,
      label: data.selection?.pageTitle || 'page',
      entityIds: [mode.entityId],
    }
  }

  return {
    inspectNode: selectedDetail
      ? {
          name: selectedDetail.name,
          tagName: selectedDetail.tagName,
          origin,
          pageId: selectedDetail.pageId,
        }
      : null,
    focusedAnnotation: focused
      ? {
          id: focused.id,
          text: focused.text,
          elementName: focused.elementName,
          anchorType: focused.anchor.type,
        }
      : null,
    canvasSelection,
  }
}
