/**
 * Canvas document routes (ADR 0019).
 *
 * `POST /canvas/apply` is the single create/update path: it walks a patch's
 * `entities`, looks up each item's handler in the entity-kind registry by
 * `kind`, and runs the whole patch inside ONE Y.Doc transaction
 * (`commitAsOneTransaction`) so a batch collapses to a single undo step. Every
 * verb and `upsertEntities` compile to this patch shape.
 *
 * An item with no `id` is a create; an item with an `id` is an update. Long /
 * structured `text` creates are routed to the `file` kind as `.md` notes
 * (`claimsAsNote`). Positions are resolved by the caller before the patch
 * arrives. (Delete folds into this route with the registry's generic delete in
 * a later ADR-0019 slice; today it stays on `/entities/delete`.)
 */

import type { Route } from './types'
import type { CanvasEntityKind } from '../../shared/types'
import { getEntityKind, hasEntityKind } from '../entities/contract'
import { claimsAsNote } from '../entities/builtin/file'
import { commitAsOneTransaction } from '../runtime/workspace-observers'
import { writeJson } from './http-helpers'

interface CanvasPatch {
  entities?: Array<Record<string, unknown>>
}

/** Resolve the handler kind for a patch item, honoring the text→note route. */
function resolveKind(item: Record<string, unknown>): CanvasEntityKind | null {
  if (!item.id && claimsAsNote(item)) return 'file'
  const kind = item.kind
  return typeof kind === 'string' && hasEntityKind(kind as CanvasEntityKind)
    ? (kind as CanvasEntityKind)
    : null
}

export const canvasRoutes: Route[] = [
  {
    method: 'POST',
    pattern: '/canvas/apply',
    async handler({ response, body }) {
      const patch = body as CanvasPatch
      const entities = patch.entities ?? []

      // Validate every item resolves to a registered kind before mutating, so a
      // bad item can't leave a half-applied transaction behind.
      for (let i = 0; i < entities.length; i++) {
        if (!resolveKind(entities[i])) {
          writeJson(response, 400, {
            error: `entities[${i}]: missing or unknown kind`,
          })
          return
        }
      }

      const created: string[] = []
      const updated: string[] = []
      const ctx = {}

      commitAsOneTransaction(() => {
        for (const item of entities) {
          const kind = resolveKind(item)!
          const handler = getEntityKind(kind)
          if (item.id) {
            handler.update(item.id as string, item, ctx)
            updated.push(item.id as string)
          } else {
            created.push(handler.create(item, ctx))
          }
        }
      })

      writeJson(response, 200, { created, updated })
    },
  },
]
