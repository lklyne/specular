/**
 * Wireframe structural-edit route (plan 3.4 — agent CLI parity).
 *
 * `specular wireframe <fileId|path> <verb> …` builds a `WireframeOp` and POSTs it
 * here. This route is the validating boundary: it resolves the target to a
 * file-entity, checks the op's node references against the current tree, then
 * applies it through `commitWireframeOp` — the same 3.0b apply path the canvas
 * gestures use. So an agent edit is one undoable Y.Doc transaction that projects
 * to `.wireframe.json`, exactly like a canvas edit.
 *
 * Errors surface as 4xx with a legible message: 404 for an unresolvable target,
 * 400 for a bad op (unknown node id, illegal prop, malformed content).
 */

import { basename } from 'path'
import { fileEntities } from '../runtime/file-entity-state'
import {
  findWireframeOpError,
  getWireframeContent,
  isWireframeFilePath,
  type WireframeOp,
} from '../runtime/wireframe-content-state'
import { commitWireframeOp } from '../runtime/wireframe-commands'
import { parseWireframeFile } from '../../shared/wireframe/wireframe-codec'
import { writeJson } from './http-helpers'
import type { Route } from './types'

/**
 * Resolve a `<fileId|path>` target to a wireframe file-entity id. Matches by
 * entity id, then exact file path, then basename — so an agent can name a
 * wireframe by its canvas id or by the path it wrote to disk.
 */
function resolveWireframeEntityId(target: string): string | null {
  const wireframes = fileEntities.filter((e) => isWireframeFilePath(e.file))
  const byId = wireframes.find((e) => e.id === target)
  if (byId) return byId.id
  const byPath = wireframes.find((e) => e.file === target)
  if (byPath) return byPath.id
  const targetBase = basename(target)
  const byBase = wireframes.find((e) => basename(e.file) === targetBase)
  if (byBase) return byBase.id
  return null
}

export const wireframeRoutes: Route[] = [
  {
    method: 'POST',
    pattern: '/wireframe/op',
    async handler({ response, body }) {
      const payload = body as { target?: unknown; op?: WireframeOp }
      if (typeof payload.target !== 'string' || !payload.op || typeof payload.op !== 'object') {
        writeJson(response, 400, { error: 'target (string) and op (object) are required' })
        return
      }

      const entityId = resolveWireframeEntityId(payload.target)
      if (!entityId) {
        writeJson(response, 404, { error: `No wireframe entity for: ${payload.target}` })
        return
      }

      // Validate node references against the current tree before applying. The
      // shared pure ops no-op on bad refs; here a bad id is a 4xx, not a silent
      // success. `replace` swaps the whole document, so it skips ref validation.
      if (payload.op.kind !== 'replace') {
        const current = getWireframeContent(entityId)
        if (current == null) {
          writeJson(response, 404, { error: `No wireframe content for: ${payload.target}` })
          return
        }
        let file
        try {
          file = parseWireframeFile(current)
        } catch (err) {
          writeJson(response, 400, { error: (err as Error).message })
          return
        }
        const refError = findWireframeOpError(file, payload.op)
        if (refError) {
          writeJson(response, 400, { error: refError })
          return
        }
      }

      const result = commitWireframeOp(entityId, payload.op)
      if (!result.ok) {
        writeJson(response, 400, { error: result.error })
        return
      }
      writeJson(response, 200, { ok: true, id: entityId, content: result.content })
    },
  },
]
