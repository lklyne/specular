/**
 * Dirty-flag system for layout IPC sends.
 *
 * Native view bounds are recomputed unconditionally every pass. These three
 * flags gate the payloads that ride along with it, and each means something
 * narrower than "something changed":
 *
 * - `canvas` — a geometry pass is needed: `buildCanvasLayoutData` rebuilds the
 *   scene and the bus diffs it. Set it only for a change to entity canvas
 *   geometry, entity membership, or z-order. Everything else the scene carries
 *   has a slice and a patch producer (see `src/main/runtime/CLAUDE.md`).
 * - `sidebar` — the left sidebar's payload, which is not on the scene bus.
 * - `toolbar` — the toolbar's zoom readout, selection summary, and presence
 *   count, likewise off the scene bus.
 *
 * Mutation sites call `markDirty()` with the surfaces they affect;
 * `layoutAllViews()` checks `consumeDirty()` per surface and only sends IPC for
 * surfaces that were actually dirtied.
 */

export type DirtySurface = 'canvas' | 'sidebar' | 'toolbar'

const ALL_SURFACES: DirtySurface[] = ['canvas', 'sidebar', 'toolbar']

const dirtyFlags = new Set<DirtySurface>()

export function markDirty(...surfaces: DirtySurface[]): void {
  for (const s of surfaces) dirtyFlags.add(s)
}

export function markAllDirty(): void {
  for (const s of ALL_SURFACES) dirtyFlags.add(s)
}

export function isDirty(surface: DirtySurface): boolean {
  return dirtyFlags.has(surface)
}

/** Returns true if the surface was dirty, and clears it. */
export function consumeDirty(surface: DirtySurface): boolean {
  if (!dirtyFlags.has(surface)) return false
  dirtyFlags.delete(surface)
  return true
}

export function clearAllDirty(): void {
  dirtyFlags.clear()
}
