import type { CanvasHoverTarget } from '../../shared/types'
import { sameHoverTarget, type RuntimePatch } from '../../shared/runtime-patch'

/**
 * The hovered canvas item, held outside the layout snapshot so a pointer move
 * repaints only the chrome that draws hover.
 *
 * Two inputs, one value. `applyPatch` takes main's per-move push; `reconcile`
 * takes the hover a full `layoutUpdate` carries, which is main's truth at the
 * moment that snapshot was built. Patches and snapshots arrive in send order,
 * so the later one wins and a dropped patch heals on the next pass — the same
 * bargain `useSceneCameraTransform` strikes for pan.
 */
export type HoverStore = {
  subscribe: (listener: () => void) => () => void
  read: () => CanvasHoverTarget
  applyPatch: (patch: RuntimePatch) => void
  reconcile: (target: CanvasHoverTarget) => void
}

export function createHoverStore(initial: CanvasHoverTarget = null): HoverStore {
  let target = initial
  const listeners = new Set<() => void>()

  function set(next: CanvasHoverTarget): void {
    if (sameHoverTarget(target, next)) return
    target = next
    for (const listener of listeners) listener()
  }

  return {
    subscribe(listener) {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    read: () => target,
    applyPatch(patch) {
      if (patch.kind === 'hover') set(patch.target)
    },
    reconcile: set,
  }
}

export const hoverStore = createHoverStore()
