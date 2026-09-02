import { shareStructure } from './layout-structural-share'
import { RUNTIME_SLICE_KEYS, type RuntimeStore } from './runtime-store'
import type { RuntimePatch } from './runtime-patch'

/**
 * `shareStructure` returns its `previous` argument whenever the two values are
 * deep-equal, so identity of the result is the equality answer. Reusing it
 * keeps one deep-comparison rule in the codebase instead of two that can drift.
 */
function deepEqual(previous: unknown, next: unknown): boolean {
  return Object.is(shareStructure(previous, next), previous)
}

/**
 * The minimal set of patches that turns `previous` into `next`.
 *
 * Slices are compared whole — they are small, and a consumer subscribed to one
 * re-reads it entirely anyway. Entities are compared one at a time, so moving a
 * single node does not re-send the scene. `buildMs` is not compared: it differs
 * on every pass and describes the pass, not the scene (see `RuntimeStore`).
 */
export function diffRuntimeStores(previous: RuntimeStore, next: RuntimeStore): RuntimePatch[] {
  const patches: RuntimePatch[] = []

  for (const slice of RUNTIME_SLICE_KEYS) {
    const before = previous.slices[slice]
    const after = next.slices[slice]
    if (deepEqual(before, after)) continue
    patches.push({ kind: 'slice', slice, value: after } as RuntimePatch)
  }

  for (const id of Object.keys(next.entities)) {
    const entity = next.entities[id]
    if (deepEqual(previous.entities[id], entity)) continue
    patches.push({ kind: 'entity', id, entity })
  }
  for (const id of Object.keys(previous.entities)) {
    if (id in next.entities) continue
    patches.push({ kind: 'entity', id, entity: null })
  }

  return patches
}
