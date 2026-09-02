/**
 * Hover moves with the pointer, so it is the one runtime slice that cannot
 * afford a scene rebuild per change. It rides a patch to the chrome that draws
 * it; `buildCanvasLayoutData` still reads `hoverTarget` for the snapshot, so a
 * pass triggered by anything else carries the current value.
 *
 * Every write goes through here. A caller that sets the runtime variable
 * directly leaves the outline on screen until the next second's snapshot.
 */

import type { CanvasHoverTarget } from '../../shared/types'
import { sameHoverTarget } from '../../shared/runtime-patch'
import { hoverTarget, setHoverTarget } from './runtime-context'
import { broadcastRuntimePatch } from './runtime-patch-broadcast'

export function commitHoverTarget(next: CanvasHoverTarget): void {
  if (sameHoverTarget(hoverTarget, next)) return
  setHoverTarget(next)
  broadcastRuntimePatch({ kind: 'slice', slice: 'hover', value: next })
}

export function clearHoverTarget(): void {
  commitHoverTarget(null)
}
