/**
 * Entity-kind registry (internal, v1).
 *
 * One self-describing handler per `CanvasEntityKind` owns that kind's headless
 * mutation behavior: create, update, serialize, default size, and the update
 * fields it honors. This mirrors the entity-renderer registry
 * (`src/main/plugins/registry.ts`) — a renderer is one claim file, an entity
 * kind is one handler file — and gives the canvas-as-document apply path a
 * single owner per kind instead of the per-kind sprawl of routes and buckets it
 * replaces (ADR 0019).
 *
 * The registry is mutated only by built-in kinds via
 * `registerBuiltInEntityKinds()` in `src/main/entities/index.ts`. It lives in
 * `src/main/`; renderers never import it (layer rule).
 */

import type { CanvasEntityKind, PersistedCanvasEntity } from '../../shared/types'
import type { JsonCanvasNode } from '../../shared/json-canvas-types'

export type EntityId = string

/**
 * A create item from a canvas patch. Positions (`canvasX`/`canvasY`) are
 * resolved by the caller (a verb or `upsertEntities`) before the patch reaches
 * the apply path; a handler reads the per-kind fields it understands and
 * ignores the rest.
 */
export type EntityCreateInput = Record<string, unknown>

/** An update patch from a canvas patch — a partial set of an entity's fields. */
export type EntityPatch = Record<string, unknown>

/**
 * Ambient services a handler may use while mutating. Empty today; reserved as a
 * single signature seam so the contract can grow (id minting, placement, …)
 * without rewriting every handler.
 */
export interface MutationContext {
  // intentionally empty for v1
}

export interface EntityKindDefinition<K extends CanvasEntityKind = CanvasEntityKind> {
  /** The kind this handler owns. Doubles as its registry key. */
  kind: K
  /** Create one entity from a positioned patch item; returns its new id. */
  create(input: EntityCreateInput, ctx: MutationContext): EntityId
  /** Apply a partial patch to an existing entity in place. */
  update(id: EntityId, patch: EntityPatch, ctx: MutationContext): void
  /** Remove an entity by id; returns whether it existed. */
  delete(id: EntityId, ctx: MutationContext): boolean
  /** Project a persisted entity to its JSON Canvas node (disk shape). */
  serialize(entity: PersistedCanvasEntity): JsonCanvasNode
  /** Rebuild a persisted entity from its JSON Canvas node (disk shape). */
  deserialize(node: JsonCanvasNode): PersistedCanvasEntity
  /** Footprint used when a create item omits an explicit size. */
  defaultSize(input: EntityCreateInput): { width: number; height: number }
  /** The update fields this kind actually honors — the rest are dropped. */
  fields: readonly string[]
}

const registry = new Map<CanvasEntityKind, EntityKindDefinition>()

/**
 * Register one kind's handler. Throws on a duplicate kind, exactly like
 * `registerEntityRenderer` rejects a duplicate id — a second owner for one kind
 * is always a bug.
 */
export function registerEntityKind<K extends CanvasEntityKind>(
  def: EntityKindDefinition<K>,
): void {
  if (registry.has(def.kind)) {
    throw new Error(`entity kind already registered: ${def.kind}`)
  }
  registry.set(def.kind, def as EntityKindDefinition)
}

/** Look up a kind's handler. Throws when the kind has no registered owner. */
export function getEntityKind(kind: CanvasEntityKind): EntityKindDefinition {
  const def = registry.get(kind)
  if (!def) throw new Error(`no entity kind registered: ${kind}`)
  return def
}

/** Whether a kind has a registered handler (without throwing). */
export function hasEntityKind(kind: CanvasEntityKind): boolean {
  return registry.has(kind)
}

/** Snapshot for debugging; not part of any IPC contract. */
export function listRegisteredEntityKinds(): readonly EntityKindDefinition[] {
  return [...registry.values()]
}

/** Test-only: drop all registrations. */
export function __resetEntityRegistryForTests(): void {
  registry.clear()
}
