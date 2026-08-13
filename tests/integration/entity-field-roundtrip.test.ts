/**
 * Entity field round-trip net (ADR 0024 §5).
 *
 * A canvas item's fields are copied by several paths — forward sync to the
 * Y.Doc, disk save/load, snapshot hydrate, copy/paste, duplicate. Each path
 * that hand-lists fields can silently fall behind a kind's declared list, and
 * the failure is invisible: the value is simply gone after a relaunch or a
 * paste. See docs/plans/entity-field-drift.md.
 *
 * This file asserts that a FULLY-populated entity survives each path with
 * every declared field intact. The samples live in `entity-field-fixtures.ts`;
 * the "samples are complete" cases below pin each sample against its kind's
 * declared field list, so a newly declared field fails here until its sample
 * sets it — otherwise the round trips would keep passing while covering one
 * field less. (`tests/` is outside both typecheck projects, so the samples'
 * `satisfies Required<Persisted…Entity>` helps in an editor but is not
 * gate-enforced; the runtime assertion is what actually holds.)
 *
 * Coverage grows one path at a time; each path lands green alongside the fix
 * that makes it pass. Covered so far:
 *   - runtime → Y.Doc → runtime (forward sync + restore)
 *
 * Mutation-verified two ways: dropping `textAlign` from `shapeCoreFields`
 * (src/main/runtime/shape-entity-state.ts) fails the shape round trip with the
 * field missing from the Y.Doc record; adding a field to
 * `SHAPE_ENTITY_PERSISTED_FIELD_SET` fails "shape sets every declared
 * persisted field".
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { getEntityKind } from '../../src/main/entities/contract'
import { DOC_MAP_ENTITIES } from '../../src/main/runtime/space-doc'
import { scheduleSpaceAutosave } from '../../src/main/runtime/space-autosave'
import { MAP_BACKED_SAMPLES, type MapBackedKind } from './entity-field-fixtures'

let harness: WorkspaceHarness

const KINDS = Object.keys(MAP_BACKED_SAMPLES) as MapBackedKind[]

/** Seed the runtime store directly from the sample.
 *
 *  Deliberately not `create` + `update`: those honor only the fields each
 *  handler casts, and closing that gap is its own step. Seeding the store is
 *  the state a live entity is in by the time any copy path reads it. */
function seedRuntime(kind: MapBackedKind): Record<string, unknown> {
  const sample = MAP_BACKED_SAMPLES[kind] as unknown as Record<string, unknown>
  getEntityKind(kind).restore([sample])
  return sample
}

/** Every declared field of `kind`, read off a copied record. */
function declaredFields(kind: MapBackedKind): readonly string[] {
  return getEntityKind(kind).fields
}

describe('entity field round-trip', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  // The net is only as good as the samples. This is what forces a newly
  // declared field to be added to its sample — without it, the round-trip
  // cases below would keep passing while quietly covering one field less.
  describe('samples are complete', () => {
    for (const kind of KINDS) {
      it(`${kind} sets every declared persisted field`, () => {
        const sample = MAP_BACKED_SAMPLES[kind] as unknown as Record<string, unknown>
        expect(Object.keys(sample).sort()).toEqual([...declaredFields(kind)].sort())
      })
    }
  })

  describe('runtime → Y.Doc → runtime', () => {
    for (const kind of KINDS) {
      it(`${kind} carries every declared field into the Y.Doc and back`, async () => {
        const sample = seedRuntime(kind)
        scheduleSpaceAutosave()
        await settleSync()

        const entities = harness.doc.getMap<Record<string, unknown>>(DOC_MAP_ENTITIES)
        const stored = entities.get(sample.id as string)
        expect(stored, `${kind} is missing from the Y.Doc entity map`).toBeDefined()

        const docRecord =
          stored instanceof Object && 'toJSON' in stored
            ? (stored as { toJSON(): Record<string, unknown> }).toJSON()
            : (stored as Record<string, unknown>)

        for (const field of declaredFields(kind)) {
          expect(docRecord, `${kind}.${field} did not reach the Y.Doc`).toHaveProperty(field)
          expect(docRecord[field], `${kind}.${field} changed value on the way to the Y.Doc`)
            .toEqual(sample[field])
        }

        // ...and back out, which is the path undo takes.
        getEntityKind(kind).restore([docRecord])
        const restored = getEntityKind(kind)
          .entities()
          .find((entity) => (entity as { id: string }).id === sample.id) as
          | Record<string, unknown>
          | undefined
        expect(restored, `${kind} did not restore from its Y.Doc record`).toBeDefined()

        for (const field of declaredFields(kind)) {
          if (field === 'kind') continue
          expect(restored?.[field], `${kind}.${field} was lost restoring from the Y.Doc`)
            .toEqual(sample[field])
        }
      })
    }
  })
})
