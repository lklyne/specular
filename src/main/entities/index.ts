/**
 * Boot-time entry point for built-in entity kinds.
 *
 * Called once from `src/main/index.ts` after `app.whenReady`, alongside
 * `registerBuiltInPlugins()`. Idempotent so repeated calls (e.g. dev
 * hot-reload) don't throw on the registry's dup-kind guard.
 */

import { registerEntityKind, __resetEntityRegistryForTests } from './contract'
import { pageKind } from './builtin/page'
import { textKind } from './builtin/text'
import { fileKind } from './builtin/file'
import { groupKind } from './builtin/group'
import { drawingKind } from './builtin/drawing'
import { shapeKind } from './builtin/shape'

// Registration order is the canonical kind order the fan-out sites enumerate
// (`allEntities()` / `forEachEntityKind()` walk it): page, text, file, drawing,
// shape, group — group trails the map-projectable kinds, matching scene z-order
// and `defaultEntityOrder`. Lookups (`getEntityKind`) are order-independent.
const builtIns = [pageKind, textKind, fileKind, drawingKind, shapeKind, groupKind]

let registered = false

export function registerBuiltInEntityKinds(): void {
  if (registered) return
  for (const def of builtIns) registerEntityKind(def)
  registered = true
}

/** Test-only: drop all registrations so a fresh round can run. */
export function __unregisterBuiltInEntityKindsForTests(): void {
  if (!registered) return
  __resetEntityRegistryForTests()
  registered = false
}
