import { shareStructure } from '../../shared/layout-structural-share'
import { applyRuntimePatchBatch, type RuntimePatchBatch } from '../../shared/runtime-patch'
import {
  snapshotToStore,
  storeToLayoutData,
  type RuntimeStore,
} from '../../shared/runtime-store'
import type { LayoutUpdateData } from '../../shared/types'
import { reportReconcileDrift } from './runtime-store-drift'

/**
 * The renderer's copy of main's runtime store.
 *
 * Two inputs, one value. `applyPatches` takes the per-change pushes;
 * `applySnapshot` takes the full `layoutUpdate`, which is main's truth at the
 * moment it was built. Both arrive in send order, so the later one wins and a
 * dropped patch heals on the next snapshot — the same bargain
 * `useSceneCameraTransform` strikes for pan.
 *
 * Identity is the product, not a nicety: layers subscribe through `useSlice`
 * and re-render only when the value they selected changes, so a snapshot that
 * repeats what is already held must not hand back new objects. Both inputs
 * reconcile through `shareStructure` for exactly that reason.
 */
export interface RuntimeStoreHandle {
  subscribe: (listener: () => void) => () => void
  read: () => RuntimeStore
  /** The store projected back into the flat snapshot shape, for the consumers
   *  that still read the whole payload. Identity-reconciled against the last
   *  projection, so memoized layers keep their bail-outs. */
  readLayoutData: () => LayoutUpdateData
  applySnapshot: (data: LayoutUpdateData) => void
  applyPatches: (batch: RuntimePatchBatch) => void
}

/**
 * `buildMs` times the pass, not the scene. `hover` has its own subscription
 * (`useHoveredEntityId`), so folding it into the projection's identity would
 * hand every `layoutData` consumer a new object on every pointer move — the
 * cost the patch bus exists to remove. Read either through `useSlice`.
 */
const PROJECTION_VOLATILE_KEYS = ['buildMs', 'hover']

export function createRuntimeStore(initial?: LayoutUpdateData): RuntimeStoreHandle {
  let store: RuntimeStore | null = initial ? snapshotToStore(initial) : null
  let projection: LayoutUpdateData | null = null
  let projectedFrom: RuntimeStore | null = null
  const listeners = new Set<() => void>()

  function read(): RuntimeStore {
    if (!store) throw new Error('runtime store read before its first snapshot')
    return store
  }

  function commit(next: RuntimeStore): void {
    if (store === next) return
    store = next
    for (const listener of listeners) listener()
  }

  return {
    subscribe(listener) {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    read,
    readLayoutData() {
      const current = read()
      if (projectedFrom === current && projection) return projection
      projection = shareStructure(
        projection,
        storeToLayoutData(current),
        PROJECTION_VOLATILE_KEYS,
      )
      projectedFrom = current
      return projection
    },
    applySnapshot(data) {
      const incoming = snapshotToStore(data)
      if (store) reportReconcileDrift(store, incoming)
      commit(store ? shareStructure(store, incoming) : incoming)
    },
    applyPatches(batch) {
      if (!store) return
      commit(applyRuntimePatchBatch(store, batch))
    },
  }
}

/** One store per renderer process, seeded from `getInitialData()` before the
 *  React root mounts. */
export const runtimeStore = createRuntimeStore()
