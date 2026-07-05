import type { Route } from './types'
import {
  getDrawingEntities,
  getFileEntities,
  getTextEntities,
} from '../runtime/document-commands'
import { animateCursorScan, allEntityPositions } from '../presence-cursor'
import { getFileReloadVersion } from '../runtime/local-file-watcher'
import { writeJson } from './http-helpers'

/**
 * Per-kind entity READ routes. The create / update / delete routes that used to
 * live here collapsed into `POST /canvas/apply` (ADR 0019 slice 4); the registry
 * handlers own those mutations now. These GET shapes remain for inspection and
 * are consumed by smoke tests and the MCP read tools.
 */
export const entityRoutes: Route[] = [
  {
    method: 'GET',
    pattern: '/text-entities',
    async handler({ request, response }) {
      animateCursorScan(request, allEntityPositions(), 'read_content')
      writeJson(response, 200, { textEntities: getTextEntities() })
    },
  },
  {
    method: 'GET',
    pattern: '/file-entities',
    async handler({ request, response }) {
      animateCursorScan(request, allEntityPositions(), 'read_content')
      const entities = getFileEntities().map((e) => ({
        ...e,
        fileReloadVersion: getFileReloadVersion(e.id),
      }))
      writeJson(response, 200, { fileEntities: entities })
    },
  },
  {
    method: 'GET',
    pattern: '/drawing-entities',
    async handler({ response }) {
      writeJson(response, 200, { drawingEntities: getDrawingEntities() })
    },
  },
]
