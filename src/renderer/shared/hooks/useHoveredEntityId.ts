import { useSyncExternalStore } from 'react'
import { hoverStore, type HoverStore } from '../hover-store'

/**
 * Subscribe to the hovered item's id. Only the layers that paint hover chrome
 * call this, so a pointer move re-renders those and nothing else — the layout
 * snapshot in `useState` no longer moves for a hover.
 */
export function useHoveredEntityId(store: HoverStore = hoverStore): string | null {
  return useSyncExternalStore(store.subscribe, () => store.read()?.id ?? null)
}
