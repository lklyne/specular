/**
 * The single canvas mutation door (ADR 0019).
 *
 * `applyCanvasPatch` walks a patch and runs the whole thing inside ONE Y.Doc
 * transaction (`commitAsOneTransaction`) so a batch collapses to a single
 * undo step. The patch shape every verb compiles to:
 *
 *   { entities: [ {kind,…}|{id,…} ], edges: [ {from,to} ], delete: ["id"] }
 *
 * An entity with no `id` is a create; an entity with an `id` is an update.
 * Long / structured `text` creates are routed to the `file` kind as `.md`
 * notes (`claimsAsNote`). Each `delete` id is resolved to its kind from the
 * doc (never an id prefix) and dispatched to that kind's handler, or to the
 * edge store when it names an edge. Positions are resolved by the caller
 * before the patch arrives.
 *
 * The HTTP route (`POST /canvas/apply`) and the integration suite both call
 * this function, so the tested path IS the shipping path.
 */

import type { CanvasEntityKind, CreateEdgesRequest } from '../shared/types'
import { getEntityKind, hasEntityKind } from './entities/contract'
import { claimsAsNote } from './entities/builtin/file'
import { entityKindById } from './workspace-entities'
import { createEdges, edgeExists } from './workspace-edges'
import { deleteEdge, updateEdge } from './runtime/document-commands'
import { commitAsOneTransaction } from './runtime/space-observers'
import { activeBackgroundTabContext } from './runtime/space-tab-context'

export interface CanvasPatch {
  entities?: Array<Record<string, unknown>>
  edges?: Array<Record<string, unknown> & { id?: string }>
  delete?: string[]
}

export interface CanvasApplyResult {
  created: string[]
  updated: string[]
  deleted: string[]
  edges: string[]
}

export class CanvasPatchError extends Error {}

// Caller-facing kind sugar that normalizes to a registered kind, so the patch
// door speaks the same vocabulary as the `add` verb (`add note` → text/file).
const KIND_ALIASES: Record<string, CanvasEntityKind> = { note: 'text' }

/**
 * Resolve the handler kind for a patch item. Updates (id present) read their
 * kind from the doc — never an id prefix or a caller hint — so the CLI never
 * sniffs prefixes (ADR 0019 §4). Creates honor the text→note route, then the
 * declared kind.
 */
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

/**
 * Pages are `WebContentsView`-backed, and a background-tab write carries the
 * target's pages through as snapshot data rather than instantiating them
 * (issue #360 §5). Every page item in such a patch would therefore either mint
 * a view onto a canvas nobody is looking at, or silently no-op against an
 * empty runtime array — so refuse the whole patch up front and say why.
 * Deferred webview instantiation is a separate problem.
 */
function rejectPageWorkOnBackgroundTab(
  patch: CanvasPatch,
  entities: Array<Record<string, unknown>>,
): void {
  const context = activeBackgroundTabContext()
  if (!context) return
  const refuse = (what: string): never => {
    throw new CanvasPatchError(
      `${what}: pages cannot be created, edited, or deleted on a background tab — ` +
        'switch to it first (`specular tab switch <ref>`), then retry without --tab',
    )
  }
  for (let i = 0; i < entities.length; i++) {
    const item = entities[i]
    const id = typeof item.id === 'string' ? item.id : null
    if (id ? context.pageIds.has(id) : item.kind === 'page') refuse(`entities[${i}]`)
  }
  for (const id of patch.delete ?? []) {
    if (context.pageIds.has(id)) refuse(`delete '${id}'`)
  }
}

/**
 * Apply a patch as one transaction / one undo step.
 * Throws `CanvasPatchError` before mutating anything if an item doesn't
 * resolve to a registered kind, so a bad item can't leave a half-applied
 * transaction behind.
 */
export function applyCanvasPatch(patch: CanvasPatch): CanvasApplyResult {
  const entities = patch.entities ?? []

  // Before kind resolution: a page id on a background tab resolves to no kind
  // at all (its runtime entity was never instantiated), which would surface as
  // a misleading "unknown kind".
  rejectPageWorkOnBackgroundTab(patch, entities)

  for (let i = 0; i < entities.length; i++) {
    if (!resolveKind(entities[i])) {
      throw new CanvasPatchError(`entities[${i}]: missing or unknown kind`)
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
      const removed = kind ? getEntityKind(kind).delete(id, ctx) : deleteEdge(id)
      if (removed) deleted.push(id)
    }
  })

  return { created, updated, deleted, edges: edgeIds }
}
