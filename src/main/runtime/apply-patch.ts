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
