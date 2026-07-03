import type { Route } from './types'
import type { AnnotationAnchor, AnnotationStatusFilter } from '../../shared/types'
import {
  addAnnotationReply,
  createAnnotation,
  deleteAnnotation,
  getAnnotationById,
  getAnnotations,
  updateAnnotationStatus,
} from '../workspace-annotations'
import {
  fixAnnotation,
  fixPendingAnnotationsForOrigin,
} from '../agent-fix/fix-orchestrator'
import { findEntityPosition, movePresenceCursorTo } from '../presence-cursor'
import { writeJson } from './http-helpers'
import type { IncomingMessage } from 'http'

/** Resolve an annotation's anchor to a canvas position. Returns null for
 * page/element anchors where the page can't be found; those are rare
 * enough that a silent miss is fine. */
function annotationAnchorPosition(
  anchor: AnnotationAnchor,
): { x: number; y: number } | null {
  if (anchor.type === 'canvas') return { x: anchor.canvasX, y: anchor.canvasY }
  if (anchor.type === 'region') {
    return {
      x: anchor.canvasRect.x + anchor.canvasRect.width / 2,
      y: anchor.canvasRect.y + anchor.canvasRect.height / 2,
    }
  }
  if (anchor.type === 'page' || anchor.type === 'element') {
    return findEntityPosition(anchor.pageId)
  }
  return null
}

function moveCursorToAnnotation(request: IncomingMessage, id: string): void {
  const annotation = getAnnotationById(id)
  if (!annotation) return
  const pos = annotationAnchorPosition(annotation.anchor)
  if (pos) movePresenceCursorTo(request, pos.x, pos.y, null)
}

export const annotationRoutes: Route[] = [
  {
    method: 'GET',
    pattern: /^\/annotations(\/[^/?]+)?(\?.*)?$/,
    async handler({ response, url }) {
      const searchParams = new URL(url, 'http://localhost').searchParams
      const status = searchParams.get('status') as AnnotationStatusFilter | null
      const annotationUrl = searchParams.get('url') ?? undefined
      const pageId = searchParams.get('page_id') ?? undefined
      const id = url.match(/^\/annotations\/([^/?]+)$/)?.[1]
      if (id) {
        const annotation = getAnnotationById(id)
        if (!annotation) {
          writeJson(response, 404, { error: `Annotation not found: ${id}` })
          return
        }
        writeJson(response, 200, annotation)
        return
      }
      writeJson(
        response,
        200,
        {
          annotations: getAnnotations({
            status: status ?? undefined,
            url: annotationUrl,
            pageId,
          }),
        },
      )
    },
  },
  {
    method: 'POST',
    pattern: '/annotations',
    async handler({ request, response, body }) {
      const payload = body as {
        anchor?: unknown
        author?: 'user' | 'agent'
        text?: string
        kind?: string
        drawing?: unknown
        metadata?: Record<string, unknown>
      }
      if (!payload.anchor || typeof payload.text !== 'string') {
        writeJson(response, 400, { error: 'anchor and text are required' })
        return
      }
      const pos = annotationAnchorPosition(payload.anchor as AnnotationAnchor)
      if (pos) movePresenceCursorTo(request, pos.x, pos.y, null)
      writeJson(
        response,
        200,
        createAnnotation(
          payload as {
            anchor: any
            author?: 'user' | 'agent'
            text: string
            kind?: any
            drawing?: any
            metadata?: Record<string, unknown>
          },
        ),
      )
    },
  },
  {
    method: 'POST',
    pattern: /^\/annotations\/([^/]+)\/acknowledge$/,
    async handler({ request, response, params }) {
      const id = params[0]
      moveCursorToAnnotation(request, id)
      const result = updateAnnotationStatus(id, 'acknowledged')
      if (!result) {
        writeJson(response, 404, { error: `Annotation not found: ${id}` })
        return
      }
      writeJson(response, 200, result)
    },
  },
  {
    method: 'POST',
    pattern: /^\/annotations\/([^/]+)\/dismiss$/,
    async handler({ request, response, body, params }) {
      const id = params[0]
      const payload = body as { reason?: string }
      moveCursorToAnnotation(request, id)
      const result = updateAnnotationStatus(id, 'dismissed', payload.reason)
      if (!result) {
        writeJson(response, 404, { error: `Annotation not found: ${id}` })
        return
      }
      writeJson(response, 200, result)
    },
  },
  {
    method: 'POST',
    pattern: /^\/annotations\/([^/]+)\/resolve$/,
    async handler({ request, response, params }) {
      const id = params[0]
      moveCursorToAnnotation(request, id)
      const result = updateAnnotationStatus(id, 'resolved')
      if (!result) {
        writeJson(response, 404, { error: `Annotation not found: ${id}` })
        return
      }
      writeJson(response, 200, result)
    },
  },
  {
    method: 'POST',
    pattern: /^\/annotations\/([^/]+)\/reply$/,
    async handler({ request, response, body, params }) {
      const id = params[0]
      const payload = body as { author?: string; text?: string }
      if (!payload.text) {
        writeJson(response, 400, { error: 'text is required' })
        return
      }
      moveCursorToAnnotation(request, id)
      const result = addAnnotationReply(id, (payload.author as 'user' | 'agent') ?? 'agent', payload.text)
      if (!result) {
        writeJson(response, 404, { error: `Annotation not found: ${id}` })
        return
      }
      writeJson(response, 200, result)
    },
  },
  {
    method: 'DELETE',
    pattern: /^\/annotations\/([^/]+)$/,
    async handler({ request, response, params }) {
      const id = params[0]
      moveCursorToAnnotation(request, id)
      const deleted = deleteAnnotation(id)
      if (!deleted) {
        writeJson(response, 404, { error: `Annotation not found: ${id}` })
        return
      }
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/annotations/fix',
    async handler({ response, body }) {
      const payload = body as { origin?: string; annotationId?: string }
      if (payload.annotationId) {
        const ok = fixAnnotation(payload.annotationId)
        writeJson(response, 200, { ok, queued: ok ? 1 : 0 })
        return
      }
      if (payload.origin) {
        const queued = fixPendingAnnotationsForOrigin(payload.origin)
        writeJson(response, 200, { ok: true, queued })
        return
      }
      writeJson(response, 400, { error: 'origin or annotationId is required' })
    },
  },
]
