import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { runtimeStore } from './runtime-store'

/**
 * Point the scene bus at this renderer's store, for the life of the process.
 *
 * Ordering, not tidiness, is why this is not a hook. Main starts sending the
 * moment the bootstrap request is answered, and a React effect subscribes a
 * mount later — everything sent in between is dropped, leaving the renderer
 * behind main until the next snapshot heals it. Connecting at module scope,
 * before the bootstrap request goes out, closes that window: patches that
 * arrive before the seed find no store and are discarded, which is correct
 * because the seed is newer than any of them.
 */
export function connectRuntimeStore(
  api: Pick<CanvasBgElectronAPI, 'onLayoutUpdate' | 'onRuntimePatch'>,
): void {
  api.onLayoutUpdate((data) => runtimeStore.applySnapshot(data))
  api.onRuntimePatch((batch) => runtimeStore.applyPatches(batch))
}
