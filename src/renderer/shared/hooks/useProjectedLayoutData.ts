import { useSyncExternalStore } from 'react'
import type { ProjectedLayoutData } from '../../../shared/scene-projection'
import { runtimeStore, type RuntimeStoreHandle } from '../runtime-store'
import { projectLayoutData } from '../scene-projection'

/**
 * The payload with its scene projected from the camera this renderer holds.
 *
 * `zoom`, `pan` and `canvasOrigin` are the `camera` and `chrome` slices of the
 * same store commit the entities came from, so a camera patch on its own
 * re-projects every entity — which is the whole of what a pan or zoom does.
 *
 * Identity-stable per store commit (`projectLayoutData` caches on the payload),
 * so the memoized layers keep their bail-outs.
 */
export function useProjectedLayoutData(
  store: RuntimeStoreHandle = runtimeStore,
): ProjectedLayoutData {
  return useSyncExternalStore(store.subscribe, () => projectLayoutData(store.readLayoutData()))
}
