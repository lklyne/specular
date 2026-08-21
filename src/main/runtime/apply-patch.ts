/**
 * Copy the defined keys of a patch onto a live runtime entity in place — the
 * mechanical field-copy every `*-entity-state.ts` `update()` shares. Only keys
 * listed in `keys` are considered, and only when present (`!== undefined`), so
 * a partial patch leaves untouched fields alone. Keys needing a transform
 * (empty-string → undefined, metadata cloning) stay explicit at the call site.
 */
export function applyPatch<T extends object>(
  entity: T,
  patch: Partial<T>,
  keys: readonly (keyof T)[],
): void {
  for (const key of keys) {
    const value = patch[key]
    if (value !== undefined) entity[key] = value as T[keyof T]
  }
}

/**
 * The patchable subset of a kind's declared persisted-field list: every field
 * except `kind`/`id` (never patchable — id is the lookup key, kind is fixed at
 * create) and any the caller excludes because `applyPatch`'s blind copy can't
 * express them (a transform like empty-string-to-undefined, or a field with a
 * side effect beyond itself). Deriving `update`'s key list from the same
 * declaration `persist()` projects from is what keeps a newly persisted field
 * wired into `update` without a second hand-list that can fall out of sync
 * with the first (docs/plans/entity-field-drift.md, Step C).
 */
export function patchableFields<T extends object>(
  persistedFields: readonly string[],
  exclude: readonly string[] = [],
): readonly (keyof T)[] {
  const skip = new Set<string>(['kind', 'id', ...exclude])
  return persistedFields.filter((field) => !skip.has(field)) as (keyof T)[]
}
