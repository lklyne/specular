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

import type { CanvasEntityKind, CanvasSceneEntity, PersistedCanvasEntity } from '../../shared/types'
import type { JsonCanvasNode } from '../../shared/json-canvas-types'

export type EntityId = string

/**
 * The minimal shape of a live runtime entity — enough for iteration, id
 * lookup, and group membership. Each kind's handler returns its own concrete
 * entities (a page, a text note, a group…); consumers that need kind-specific
 * fields narrow by kind.
 */
export interface RuntimeEntity {
  id: EntityId
  parentGroupId?: string
}

type ScenePoint = { x: number; y: number }

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
  /**
   * The authoritative persisted field list: every key this kind writes to its
   * Y.Doc map (the shared entity map; the pages/groups maps for page/group).
   * Both sync directions derive from it — the persist projection's output
   * keys must match it (drift-tested per kind) and `restore` must account for
   * every field on undo.
   */
  fields: readonly string[]
  /**
   * Reconcile this kind's live runtime store from its persisted doc snapshots
   * after undo/redo reverts the Y.Doc. Runs inside `withSuppressedDocSync`;
   * the observer passes every snapshot of this kind (an empty array when the
   * doc holds none). The map-backed kinds replace their array wholesale; page
   * patches its live WebContentsView-backed objects field by field (see
   * `runtime/page-doc-projection.ts`).
   */
  restore(snapshots: readonly Record<string, unknown>[]): void
  /**
   * This kind's live runtime entities — the raw store its persist and scene
   * projections read. For `drawing` this is the raw store, deliberately
   * distinct from the UI-filtered `drawingEntitiesForUi()` view the sidebar and
   * canvas scene read; persistence and stack order must see the raw store.
   */
  entities(): readonly RuntimeEntity[]
  /**
   * Project one runtime entity to its on-canvas scene entity. Present only on
   * the map-projectable kinds (text, file, drawing, shape); page (a
   * WebContents-backed overlay) and group (needs its child ids) build their
   * scene entities through bespoke call-site paths.
   */
  buildSceneEntity?(entity: RuntimeEntity): CanvasSceneEntity
  /**
   * Project one runtime entity to its persisted shape. Present only on the
   * map-projectable kinds; page mirrors to the pages map via `serializePage`
   * and group mirrors to the groups map, both outside the entity map.
   */
  persist?(entity: RuntimeEntity): PersistedCanvasEntity
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

/**
 * Run `fn` for each registered kind in registration order. Registration order
 * is the canonical kind order the fan-out sites enumerate (page, text, file,
 * drawing, shape, group); edges are not a registered kind.
 */
export function forEachEntityKind(fn: (def: EntityKindDefinition) => void): void {
  for (const def of registry.values()) fn(def)
}

/**
 * Every registered kind's live entities, each tagged with its kind, flattened
 * in registration order. The one door for "walk all canvas entities" —
 * replacing the copy-pasted `[...textEntities, ...fileEntities, …]` fan-out.
 * Edges are never included (edges do not register).
 */
export function allEntities(): { kind: CanvasEntityKind; entity: RuntimeEntity }[] {
  const out: { kind: CanvasEntityKind; entity: RuntimeEntity }[] = []
  for (const def of registry.values()) {
    for (const entity of def.entities()) out.push({ kind: def.kind, entity })
  }
  return out
}

/** Test-only: drop all registrations. */
export function __resetEntityRegistryForTests(): void {
  registry.clear()
}
