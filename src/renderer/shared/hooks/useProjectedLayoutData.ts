import { useSyncExternalStore } from 'react'
import type { LayoutUpdateData } from '../../../shared/types'
import { runtimeStore, type RuntimeStoreHandle } from '../runtime-store'
import { projectLayoutData } from '../scene-projection'

/**
 * The payload with its scene projected from the camera this renderer holds.
 *
 * `zoom`, `pan` and `canvasOrigin` are the `camera` and `chrome` slices of the
 * same store commit the entities came from, so a layer reading a projected
 * entity is reading one consistent camera rather than whichever of main's
 * `screen*` fields or the renderer's camera the store happened to receive last.
 *
 * Identity-stable per store commit (`projectLayoutData` caches on the payload),
 * so the memoized layers keep their bail-outs.
 */
export function useProjectedLayoutData(
  store: RuntimeStoreHandle = runtimeStore,
): LayoutUpdateData {
  return useSyncExternalStore(store.subscribe, () => projectLayoutData(store.readLayoutData()))
}
