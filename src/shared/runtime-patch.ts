import type { CanvasHoverTarget } from './types'

/**
 * One slice of ephemeral runtime state, pushed on its own channel instead of
 * riding a full layout rebuild. Cost scales with what moved, not with scene
 * size.
 *
 * Every kind is also carried by the `layoutUpdate` snapshot, which stays the
 * reconcile baseline: a renderer that dropped or mis-applied a patch converges
 * on the next full pass rather than holding stale chrome. That is what lets
 * patches be lossy.
 */
export type HoverPatch = { kind: 'hover'; target: CanvasHoverTarget }

export type RuntimePatch = HoverPatch

/** Hover identity: a `{ kind, id }` ref, or the absence of one. */
export function sameHoverTarget(a: CanvasHoverTarget, b: CanvasHoverTarget): boolean {
  if (!a || !b) return a === b
  return a.id === b.id && a.kind === b.kind
}
