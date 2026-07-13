import type {
  Annotation,
  AnnotationCreateRequest,
  AnnotationMetadata,
  AnnotationReply,
  AnnotationStatus,
  AnnotationStatusFilter,
} from '../shared/types'
import { canonicalAnnotationUrl, isUnresolved } from '../shared/annotation-utils'
import type { PageAnchor } from '../shared/page-anchor'
import {
  findPageById,
  getComponentAncestryByNodeId,
  getComponentSourceLocationByNodeId,
} from './runtime/page-runtime'
import { markDirty } from './runtime/layout-dirty'
import { mutateWorkspace } from './runtime/mutate-workspace'
import { workspaceAnnotations } from './runtime/workspace-model'
import { scheduleWorkspaceAutosave } from './runtime/workspace-autosave'
import { makeId } from './workspace-utils'
import { VIEWPORT_PRESETS } from '../shared/constants'

function resolvePageName(pageId: string): string | undefined {
  const page = findPageById(pageId)
  if (!page) return undefined
  const preset = VIEWPORT_PRESETS[page.presetIndex]
  if (!preset) return undefined
  const label = page.name?.trim() || preset.label
  return `${label} ${preset.width}×${preset.height}`
}

/**
 * The page an annotation binds to, decided at creation. Element and page
 * anchors name their page structurally. A region binds iff its marquee
 * grabbed page content — some `regionComponents`/`regionElements` group has a
 * non-empty inner list (the region select emits a group per *intersecting*
 * page even when nothing was grabbed, so group presence alone is not a grab);
 * the first grabbing group is the page that contributed it. Canvas points
 * never bind.
 */
function annotationAnchorPageId(request: AnnotationCreateRequest): string | undefined {
  const anchor = request.anchor
  if (anchor.type === 'page' || anchor.type === 'element') return anchor.pageId
  if (anchor.type === 'region') {
    const grabbed =
      request.metadata?.regionComponents?.find((group) => group.components.length > 0) ??
      request.metadata?.regionElements?.find((group) => group.elements.length > 0)
    return grabbed?.pageId
  }
  return undefined
}

/** The anchor for a live page: its id plus the document URL it shows now. */
function pageAnchorForPage(pageId: string): PageAnchor | undefined {
  const page = findPageById(pageId)
  if (!page) return undefined
  const pageUrl = canonicalAnnotationUrl(page.url)
  return { pageId, ...(pageUrl ? { pageUrl } : {}) }
}

/** Display context: a human-readable page label for panel and prompt copy.
 *  The page *binding* lives in `Annotation.pageAnchor`, not in metadata. */
function withPageNameMetadata(
  metadata: AnnotationMetadata | undefined,
  pageId: string | undefined,
): AnnotationMetadata | undefined {
  const pageName = pageId ? resolvePageName(pageId) : undefined
  if (!pageName) return metadata
  return { ...(metadata ?? {}), pageName }
}

/** Element anchors: resolve React component ancestry and source location for
 *  the inspected node from the page's cached component tree. */
function withInspectEnrichment(
  request: AnnotationCreateRequest,
  metadata: AnnotationMetadata | undefined,
): AnnotationMetadata | undefined {
  if (request.anchor.type !== 'element') return metadata
  const inspectContext = metadata?.inspectContext
  if (!inspectContext?.nodeId) return metadata

  const reactComponents = getComponentAncestryByNodeId(
    request.anchor.pageId,
    inspectContext.nodeId,
  )
  const sourceLocation = getComponentSourceLocationByNodeId(
    request.anchor.pageId,
    inspectContext.nodeId,
  )
  if (!reactComponents.length && !sourceLocation) return metadata

  return {
    ...metadata,
    inspectContext: {
      ...inspectContext,
      ...(reactComponents.length ? { reactComponents } : {}),
      ...(sourceLocation ? { sourceLocation } : {}),
    },
  }
}

export function getAnnotations(filters?: {
  status?: AnnotationStatusFilter
  url?: string
  pageId?: string
}): Annotation[] {
  const targetUrl = canonicalAnnotationUrl(filters?.url)
  return workspaceAnnotations.filter((annotation) => {
    if (filters?.status && filters.status !== 'all') {
      if (filters.status === 'unresolved') {
        if (!isUnresolved(annotation.status)) return false
      } else if (annotation.status !== filters.status) {
        return false
      }
    }
    // Page binding reads `pageAnchor` only — annotations without one are
    // canvas-bound and match no page/url filter.
    if (filters?.pageId && annotation.pageAnchor?.pageId !== filters.pageId) {
      return false
    }
    if (targetUrl) {
      const annotationUrl = canonicalAnnotationUrl(annotation.pageAnchor?.pageUrl)
      if (!annotationUrl || annotationUrl !== targetUrl) return false
    }
    return true
  })
}

export function getAnnotationById(id: string): Annotation | undefined {
  return workspaceAnnotations.find((a) => a.id === id)
}

let onAnnotationCreatedListener: ((annotation: Annotation) => void) | null = null

export function setOnAnnotationCreated(
  fn: ((annotation: Annotation) => void) | null,
): void {
  onAnnotationCreatedListener = fn
}

let onAnnotationReplyListener:
  | ((annotation: Annotation, reply: AnnotationReply) => void)
  | null = null

export function setOnAnnotationReply(
  fn: ((annotation: Annotation, reply: AnnotationReply) => void) | null,
): void {
  onAnnotationReplyListener = fn
}

export function createAnnotation(request: AnnotationCreateRequest): Annotation {
  return mutateWorkspace(() => createAnnotationInternal(request))
}

function createAnnotationInternal(request: AnnotationCreateRequest): Annotation {
  const elementName =
    request.anchor.type === 'element'
      ? request.elementName?.trim() || undefined
      : undefined
  const anchorPageId = annotationAnchorPageId(request)
  const pageAnchor = anchorPageId ? pageAnchorForPage(anchorPageId) : undefined
  const metadata = withInspectEnrichment(
    request,
    withPageNameMetadata(
      request.metadata ? { ...request.metadata } : undefined,
      anchorPageId,
    ),
  )
  const annotation: Annotation = {
    id: makeId('ann'),
    anchor: request.anchor,
    author: request.author ?? 'user',
    text: request.text,
    status: 'pending',
    replies: [],
    createdAt: new Date().toISOString(),
    ...(elementName ? { elementName } : {}),
    ...(pageAnchor ? { pageAnchor } : {}),
    metadata: metadata && Object.keys(metadata).length ? metadata : undefined,
  }
  workspaceAnnotations.push(annotation)
  markDirty('sidebar')
  if (onAnnotationCreatedListener) {
    try {
      onAnnotationCreatedListener(annotation)
    } catch (error) {
      console.error('onAnnotationCreated listener failed:', error)
    }
  }
  return annotation
}

export function updateAnnotationStatus(
  id: string,
  status: AnnotationStatus,
  reason?: string,
  resolvedBy?: 'user' | 'agent',
): Annotation | null {
  return mutateWorkspace(() => {
    const annotation = workspaceAnnotations.find((a) => a.id === id)
    if (!annotation) return null
    annotation.status = status
    markDirty('sidebar')
    const metadataPatch: AnnotationMetadata = { ...annotation.metadata }
    if (reason) {
      metadataPatch.dismissReason = reason
    } else if (status !== 'dismissed') {
      delete metadataPatch.dismissReason
    }
    if (status === 'resolved' && resolvedBy) {
      metadataPatch.resolvedBy = resolvedBy
    } else if (status !== 'resolved') {
      delete metadataPatch.resolvedBy
    }
    if (Object.keys(metadataPatch).length) {
      annotation.metadata = metadataPatch
    }
    return annotation
  }, { changed: (annotation) => annotation !== null })
}

export function setAnnotationFixSession(id: string, sessionId: string): void {
  const annotation = workspaceAnnotations.find((a) => a.id === id)
  if (!annotation) return
  if (annotation.metadata?.fixSessionId === sessionId) return
  annotation.metadata = { ...annotation.metadata, fixSessionId: sessionId }
  markDirty('canvas')
  scheduleWorkspaceAutosave()
}

export function addAnnotationReply(
  id: string,
  author: 'user' | 'agent',
  text: string,
): Annotation | null {
  return mutateWorkspace(() => {
    const annotation = workspaceAnnotations.find((a) => a.id === id)
    if (!annotation) return null
    const reply: AnnotationReply = { author, text, timestamp: new Date().toISOString() }
    annotation.replies = [...annotation.replies, reply]
    markDirty('sidebar')
    if (author === 'user' && annotation.status === 'resolved') {
      updateAnnotationStatus(id, 'pending')
    }
    if (onAnnotationReplyListener) {
      try {
        onAnnotationReplyListener(annotation, reply)
      } catch (error) {
        console.error('onAnnotationReply listener failed:', error)
      }
    }
    return annotation
  }, { changed: (annotation) => annotation !== null })
}

export function deleteAnnotation(id: string): boolean {
  return mutateWorkspace(() => {
    const idx = workspaceAnnotations.findIndex((a) => a.id === id)
    if (idx === -1) return false
    workspaceAnnotations.splice(idx, 1)
    markDirty('sidebar')
    return true
  }, { changed: (deleted) => deleted })
}
