import type { RuntimeStore } from '../../../shared/runtime-store'
import { runtimeStore, type RuntimeStoreHandle } from '../runtime-store'
import { useSlice } from './useRuntimeStore'

const selectHoveredEntityId = (store: RuntimeStore): string | null =>
  store.slices.hover?.id ?? null

/**
 * Subscribe to the hovered item's id. Only the layers that paint hover chrome
 * call this, so a pointer move re-renders those and nothing else — the layout
 * projection deliberately leaves hover out of its identity.
 */
export function useHoveredEntityId(store: RuntimeStoreHandle = runtimeStore): string | null {
  return useSlice(selectHoveredEntityId, Object.is, store)
}
