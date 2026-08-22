import type { CanvasHoverTarget, CanvasSceneEntity } from './types'
import type { RuntimeSliceKey, RuntimeStore, RuntimeStoreSlices } from './runtime-store'

/**
 * One addressed change to the runtime store, pushed on its own channel instead
 * of riding a full layout rebuild. Cost scales with what moved, not with scene
 * size.
 *
 * Two addresses, because the store has two axes: a slice patch replaces one of
 * the small top-level slices wholesale, an entity patch replaces (or removes,
 * with `entity: null`) one entry of the entity map.
 *
 * Every value a patch carries is also carried by the `layoutUpdate` snapshot,
 * which stays the reconcile baseline: a renderer that dropped or mis-applied a
 * patch converges on the next full pass rather than holding stale chrome. That
 * is what lets patches be lossy.
 */
export type RuntimeSlicePatch = {
  [K in RuntimeSliceKey]: { kind: 'slice'; slice: K; value: RuntimeStoreSlices[K] }
}[RuntimeSliceKey]

export type RuntimeEntityPatch = {
  kind: 'entity'
  id: string
  entity: CanvasSceneEntity | null
}

export type RuntimePatch = RuntimeSlicePatch | RuntimeEntityPatch

/**
 * What one send carries. A layout pass produces every patch it produces at
 * once, so they cross the wire together and apply together — a renderer never
 * paints a half-applied pass.
 *
 * `buildMs` times the pass rather than describing the scene, so it rides the
 * envelope instead of a patch (see `RuntimeStore.buildMs`).
 */
export interface RuntimePatchBatch {
  patches: RuntimePatch[]
  buildMs?: number
}

export function applyRuntimePatch(store: RuntimeStore, patch: RuntimePatch): RuntimeStore {
  if (patch.kind === 'entity') {
    const current = store.entities[patch.id]
    if (patch.entity === null) {
      if (current === undefined) return store
      const entities = { ...store.entities }
      delete entities[patch.id]
      return { ...store, entities }
    }
    if (current === patch.entity) return store
    return { ...store, entities: { ...store.entities, [patch.id]: patch.entity } }
  }
  if (store.slices[patch.slice] === patch.value) return store
  return { ...store, slices: { ...store.slices, [patch.slice]: patch.value } }
}

export function applyRuntimePatchBatch(
  store: RuntimeStore,
  batch: RuntimePatchBatch,
): RuntimeStore {
  let next = store
  for (const patch of batch.patches) next = applyRuntimePatch(next, patch)
  if (batch.buildMs != null && batch.buildMs !== next.buildMs) {
    next = { ...next, buildMs: batch.buildMs }
  }
  return next
}

/** Hover identity: a `{ kind, id }` ref, or the absence of one. */
export function sameHoverTarget(a: CanvasHoverTarget, b: CanvasHoverTarget): boolean {
  if (!a || !b) return a === b
  return a.id === b.id && a.kind === b.kind
}
