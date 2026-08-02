/**
 * Canvas document routes (ADR 0019).
 *
 * `GET /canvas` serializes the whole doc to a JSON Canvas document (the read
 * shape; `specular workspace` reads it).
 *
 * `POST /canvas/apply` is the HTTP transport for the single mutation door —
 * the patch semantics live in `src/main/canvas-apply.ts` (shared with the
 * integration suite).
 *
 * Both honor `--tab` (issue #360 §3), resolved into `targetTab` by the server.
 * A read of a background tab serializes its record directly — no swap, no
 * focus change. A write of one runs the normal spine inside `withTabContext`.
 */

import type { Route } from './types'
import type { PersistedWorkspaceTab } from '../../shared/types'
import { applyCanvasPatch, CanvasPatchError, type CanvasPatch } from '../canvas-apply'
import { spaceSnapshot, spaceTabIdentity } from '../runtime/space-tabs'
import { activeSpaceTabId } from '../runtime/space-model'
import { withTabContext } from '../runtime/space-tab-context'
import { serializeToJsonCanvas } from '../runtime/json-canvas-serializer'
import { writeJson } from './http-helpers'
import type { JsonCanvasDocument } from '../../shared/json-canvas-types'

/**
 * The live read carries tab identity so a reader knows which canvas answered.
 * It is added here rather than in the serializer: a `.canvas` file on disk
 * describes one tab and has nothing to say about its siblings.
 *
 * A background tab is read from its own record. `spaceTabIdentity()` syncs
 * the active tab first, so when the target IS the active tab the record is
 * already current and the two paths agree.
 */
export function readCanvasDocument(targetTab?: PersistedWorkspaceTab): JsonCanvasDocument {
  const identity = spaceTabIdentity()
  const doc =
    targetTab && targetTab.id !== activeSpaceTabId
      ? serializeToJsonCanvas(targetTab.snapshot, targetTab.annotations)
      : serializeToJsonCanvas(spaceSnapshot())
  if (doc.appState) {
    doc.appState.activeTab = identity.activeTab ?? undefined
    doc.appState.tabs = identity.tabs
  }
  return doc
}

export const canvasRoutes: Route[] = [
  {
    method: 'GET',
    pattern: '/canvas',
    tabScoped: true,
    async handler({ response, targetTab }) {
      writeJson(response, 200, readCanvasDocument(targetTab))
    },
  },
  {
    method: 'POST',
    pattern: '/canvas/apply',
    tabScoped: true,
    async handler({ response, body, targetTab }) {
      try {
        const patch = body as CanvasPatch
        const result = targetTab
          ? withTabContext(targetTab.id, () => applyCanvasPatch(patch))
          : applyCanvasPatch(patch)
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
