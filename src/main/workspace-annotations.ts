import type {
  Annotation,
  AnnotationCreateRequest,
  AnnotationMetadata,
  AnnotationReply,
  AnnotationStatus,
  AnnotationStatusFilter,
} from '../shared/types'
import { canonicalAnnotationUrl, isUnresolved } from '../shared/annotation-utils'
import {
  findPageById,
  getComponentAncestryByNodeId,
  getComponentSourceLocationByNodeId,
} from './runtime/page-runtime'
import { markDirty } from './runtime/layout-dirty'
import { mutateWorkspace } from './runtime/mutate-workspace'
import { requestLayout } from './runtime/viewport-control'
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

function resolvePageUrl(pageId: string): string | undefined {
  const page = findPageById(pageId)
  if (!page) return undefined
  return canonicalAnnotationUrl(page.pageView.webContents.getURL())
}

function regionPrimaryPageId(
  metadata: AnnotationMetadata | undefined,
): string | undefined {
  return (
    metadata?.regionComponents?.[0]?.pageId ??
    metadata?.regionElements?.[0]?.pageId
  )
}

function enrichedAnnotationMetadata(
  request: AnnotationCreateRequest,
): AnnotationMetadata | undefined {
  const anchor = request.anchor
  const contextPageId =
    anchor.type === 'page' || anchor.type === 'element'
      ? anchor.pageId
      : anchor.type === 'region'
        ? regionPrimaryPageId(request.metadata)
        : undefined
  const pageName = contextPageId ? resolvePageName(contextPageId) : undefined
  const pageUrl = contextPageId ? resolvePageUrl(contextPageId) : undefined

  const metadata = request.metadata ? { ...request.metadata } : undefined
  const metadataWithContext: AnnotationMetadata = {
    ...(metadata ?? {}),
    ...(pageName ? { pageName } : {}),
    ...(pageUrl ? { pageUrl } : {}),
  }

  // For non-element anchors, just attach pageName if available
  if (anchor.type !== 'element') {
    return Object.keys(metadataWithContext).length ? metadataWithContext : undefined
  }

  const inspectContext = metadataWithContext.inspectContext
  if (!inspectContext?.nodeId) {
    return Object.keys(metadataWithContext).length ? metadataWithContext : undefined
  }

  const reactComponents = getComponentAncestryByNodeId(
    anchor.pageId,
    inspectContext.nodeId,
  )
  const sourceLocation = getComponentSourceLocationByNodeId(
    anchor.pageId,
    inspectContext.nodeId,
  )

  return {
    ...metadataWithContext,
    ...((reactComponents.length || sourceLocation)
      ? {
          inspectContext: {
            ...inspectContext,
            ...(reactComponents.length ? { reactComponents } : {}),
            ...(sourceLocation ? { sourceLocation } : {}),
          },
        }
      : {}),
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
    if (filters?.pageId) {
      if (annotation.anchor.type === 'canvas') return false
      if (annotation.anchor.type === 'region') {
        const match = annotation.metadata?.regionComponents?.some(
          (g) => g.pageId === filters.pageId,
        ) ?? false
        if (!match) return false
      } else if (annotation.anchor.pageId !== filters.pageId) {
        return false
      }
    }
    if (targetUrl) {
      const annotationUrl = canonicalAnnotationUrl(annotation.metadata?.pageUrl)
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
  const annotation: Annotation = {
    id: makeId('ann'),
    anchor: request.anchor,
    author: request.author ?? 'user',
    text: request.text,
    status: 'pending',
    replies: [],
    createdAt: new Date().toISOString(),
    ...(elementName ? { elementName } : {}),
    metadata: enrichedAnnotationMetadata(request),
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
