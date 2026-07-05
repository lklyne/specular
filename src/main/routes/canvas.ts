/**
 * Canvas document routes (ADR 0019).
 *
 * `GET /canvas` serializes the whole doc to a JSON Canvas document (the read
 * shape; `specular workspace` reads it).
 *
 * `POST /canvas/apply` is the HTTP transport for the single mutation door —
 * the patch semantics live in `src/main/canvas-apply.ts` (shared with the
 * integration suite).
 */

import type { Route } from './types'
import { applyCanvasPatch, CanvasPatchError, type CanvasPatch } from '../canvas-apply'
import { workspaceSnapshot } from '../runtime/workspace-tabs'
import { serializeToJsonCanvas } from '../runtime/json-canvas-serializer'
import { writeJson } from './http-helpers'

export const canvasRoutes: Route[] = [
  {
    method: 'GET',
    pattern: '/canvas',
    async handler({ response }) {
      writeJson(response, 200, serializeToJsonCanvas(workspaceSnapshot()))
    },
  },
  {
    method: 'POST',
    pattern: '/canvas/apply',
    async handler({ response, body }) {
      try {
        const result = applyCanvasPatch(body as CanvasPatch)
        writeJson(response, 200, result)
      } catch (error) {
        if (error instanceof CanvasPatchError) {
          writeJson(response, 400, { error: error.message })
          return
        }
        throw error
      }
    },
  },
]
