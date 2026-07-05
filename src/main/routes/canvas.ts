/**
 * Canvas document routes (ADR 0019).
 *
 * `GET /canvas` serializes the whole doc to a JSON Canvas document (the read
 * shape; `specular workspace` reads it).
 *
 * `POST /canvas/apply` is the single mutation door: it walks a patch and runs
 * the whole thing inside ONE Y.Doc transaction (`commitAsOneTransaction`) so a
 * batch collapses to a single undo step. The patch shape every verb compiles to:
 *
 *   { entities: [ {kind,…}|{id,…} ], edges: [ {from,to} ], delete: ["id"] }
 *
 * An entity with no `id` is a create; an entity with an `id` is an update. Long /
 * structured `text` creates are routed to the `file` kind as `.md` notes
 * (`claimsAsNote`). Each `delete` id is resolved to its kind from the doc (never
 * an id prefix) and dispatched to that kind's handler, or to the edge store when
 * it names an edge. Positions are resolved by the caller before the patch
 * arrives.
 */

import type { Route } from './types'
import type { CanvasEntityKind } from '../../shared/types'
import type { CreateEdgesRequest } from '../../shared/types'
import { getEntityKind, hasEntityKind } from '../entities/contract'
import { claimsAsNote } from '../entities/builtin/file'
import { entityKindById } from '../workspace-entities'
import { createEdges, edgeExists } from '../workspace-edges'
import { deleteEdge, updateEdge } from '../runtime/document-commands'
import { workspaceSnapshot } from '../runtime/workspace-tabs'
import { serializeToJsonCanvas } from '../runtime/json-canvas-serializer'
import { commitAsOneTransaction } from '../runtime/workspace-observers'
import { writeJson } from './http-helpers'

interface CanvasPatch {
  entities?: Array<Record<string, unknown>>
  edges?: Array<Record<string, unknown> & { id?: string }>
  delete?: string[]
}

/**
 * Resolve the handler kind for a patch item. Updates (id present) read their
 * kind from the doc — never an id prefix or a caller hint — so the CLI never
 * sniffs prefixes (ADR 0019 §4). Creates honor the text→note route, then the
 * declared kind.
 */
// Caller-facing kind sugar that normalizes to a registered kind, so the patch
// door speaks the same vocabulary as the `add` verb (`add note` → text/file).
const KIND_ALIASES: Record<string, CanvasEntityKind> = { note: 'text' }

function resolveKind(item: Record<string, unknown>): CanvasEntityKind | null {
  if (item.id) return entityKindById(item.id as string)
  // Normalize aliases before the note-route + registry checks (claimsAsNote
  // requires kind === 'text', so this must run first).
  if (typeof item.kind === 'string' && KIND_ALIASES[item.kind]) {
    item.kind = KIND_ALIASES[item.kind]
  }
  if (claimsAsNote(item)) return 'file'
  const kind = item.kind
  return typeof kind === 'string' && hasEntityKind(kind as CanvasEntityKind)
    ? (kind as CanvasEntityKind)
    : null
}

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
      const patch = body as CanvasPatch
      const entities = patch.entities ?? []

      // Validate every create/update item resolves to a registered kind before
      // mutating, so a bad item can't leave a half-applied transaction behind.
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
      const deleted: string[] = []
      let edgeIds: string[] = []
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

        // Edges follow the same upsert contract as entities: an id that
        // resolves to an existing edge patches it in place; anything else
        // (no id, or an id the doc doesn't know) creates.
        const edgeCreates: CreateEdgesRequest['edges'] = []
        const edgeUpdateIds: string[] = []
        for (const item of patch.edges ?? []) {
          const id = typeof item.id === 'string' ? item.id : undefined
          if (id && edgeExists(id)) {
            updateEdge(id, item as Parameters<typeof updateEdge>[1])
            edgeUpdateIds.push(id)
          } else {
            edgeCreates.push(item as CreateEdgesRequest['edges'][number])
          }
        }
        if (edgeCreates.length) {
          edgeIds = createEdges({ edges: edgeCreates }).edgeIds
        }
        edgeIds = edgeIds.concat(edgeUpdateIds)

        for (const id of patch.delete ?? []) {
          // Kind is resolved from the doc, never an id prefix. An id the doc
          // doesn't know as an entity is tried as an edge.
          const kind = entityKindById(id)
          const removed = kind
            ? getEntityKind(kind).delete(id, ctx)
            : deleteEdge(id)
          if (removed) deleted.push(id)
        }
      })

      writeJson(response, 200, { created, updated, deleted, edges: edgeIds })
    },
  },
]
