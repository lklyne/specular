import { useMemo, useSyncExternalStore } from 'react'
import type { RuntimeStore } from '../../../shared/runtime-store'
import type { LayoutUpdateData } from '../../../shared/types'
import { runtimeStore, type RuntimeStoreHandle } from '../runtime-store'

/**
 * Subscribe to one derived value of the runtime store. The component re-renders
 * when that value changes, not when the store does — which is the whole point
 * of the store: a hover moves one slice, and only the layers that selected it
 * repaint.
 *
 * `selector` must be stable across renders (module scope or `useCallback`).
 * Selectors that build a fresh object each call need an `isEqual` that can see
 * past identity; the default `Object.is` is right for the slices themselves,
 * which the store keeps identity-stable.
 */
export function useSlice<T>(
  selector: (store: RuntimeStore) => T,
  isEqual: (a: T, b: T) => boolean = Object.is,
  store: RuntimeStoreHandle = runtimeStore,
): T {
  const getSnapshot = useMemo(
    () => createSliceReader(selector, isEqual, store),
    [selector, isEqual, store],
  )
  return useSyncExternalStore(store.subscribe, getSnapshot)
}

/** The whole store projected back into the flat snapshot shape. The bridge for
 *  consumers not yet split into slices. */
export function useLayoutData(store: RuntimeStoreHandle = runtimeStore): LayoutUpdateData {
  return useSyncExternalStore(store.subscribe, store.readLayoutData)
}

/**
 * `useSyncExternalStore` re-reads on every render and compares by identity, so
 * a selector that derives a value has to return the same reference until the
 * derivation actually changes. The reader caches against the store it read
 * from, then falls back to `isEqual` for selectors that rebuild their result.
 */
function createSliceReader<T>(
  selector: (store: RuntimeStore) => T,
  isEqual: (a: T, b: T) => boolean,
  store: RuntimeStoreHandle,
): () => T {
  let cache: { from: RuntimeStore; value: T } | null = null
  return () => {
    const current = store.read()
    if (cache && cache.from === current) return cache.value
    const next = selector(current)
    const value = cache && isEqual(cache.value, next) ? cache.value : next
    cache = { from: current, value }
    return value
  }
}
