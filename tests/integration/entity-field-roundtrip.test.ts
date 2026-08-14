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
 *   - runtime → .canvas → runtime (save + reload)
 *   - runtime → .canvas → runtime for group and page (Step A: these two kinds
 *     were rebuilt from disk through space-restore.ts's hand-written page and
 *     group arms rather than the registry)
 *   - tab switch (Step A: space-tabs.ts's spaceSnapshot() hand-listed five
 *     per-kind persist loops; space-restore.ts's restoreWorkspaceSnapshot
 *     hand-listed the page arm twice and the group arm once)
 *
 * Mutation-verified two ways: dropping `textAlign` from `shapeCoreFields`
 * (src/main/runtime/shape-entity-state.ts) fails the shape round trip with the
 * field missing from the Y.Doc record; adding a field to
 * `SHAPE_ENTITY_PERSISTED_FIELD_SET` fails "shape sets every declared
 * persisted field".
 *
 * The group/page disk-round-trip and tab-switch cases below were
 * mutation-verified against the pre-Step-A hand-written code: temporarily
 * dropping `colorScheme` from both page-rebuild call sites in
 * `restoreWorkspaceSnapshot` (src/main/runtime/space-restore.ts) failed the
 * page cases in both describes; temporarily dropping `sourceTaskId` from the
 * hand-built group object in the same function's entities loop failed the
 * group cases; temporarily deleting the `shape` persist loop in
 * `spaceSnapshot()` (src/main/runtime/space-tabs.ts) failed the tab-switch
 * shape case (the entity vanished from the tab's snapshot entirely). All
 * reverted before landing the collapse to the registry intake door.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { getEntityKind } from '../../src/main/entities/contract'
import { DOC_MAP_ENTITIES } from '../../src/main/runtime/space-doc'
import { scheduleSpaceAutosave } from '../../src/main/runtime/space-autosave'
import { reloadWorkspaceDataFromCurrentSpace } from '../../src/main/runtime/space-change'
import { createSpaceTab, setActiveSpaceTab } from '../../src/main/runtime/space-tab-operations'
import { activeSpaceTabId } from '../../src/main/runtime/space-model'
import {
  MAP_BACKED_SAMPLES,
  SAMPLE_GROUP,
  SAMPLE_PAGE,
  type MapBackedKind,
} from './entity-field-fixtures'

/**
 * `group`'s declared field list (`getEntityKind('group').fields`) includes
 * `pageIds`/`entityIds`, but `persistGroupEntity` doesn't project them and
 * `PersistedGroupEntity` doesn't carry them — a pre-existing gap the plan
 * (docs/plans/entity-field-drift.md, "Deliberately not here") calls out as
 * its own decision, not part of Step A. Check every field the sample
 * declares instead of the kind's full field list.
 */
const GROUP_FIELDS_TO_CHECK = Object.keys(SAMPLE_GROUP).filter(
  (field) => field !== 'kind' && field !== 'id',
) as (keyof typeof SAMPLE_GROUP)[]

/**
 * `page` is excluded from the generic map-backed loop (see
 * entity-field-fixtures.ts): its declared fields
 * (`getEntityKind('page').fields`) omit `kind` (implicit in the pages map)
 * and `groupId` (folded into `parentGroupId` on write). Check the sample's
 * own fields minus those two plus `id`, which is used for lookup rather than
 * asserted.
 */
const PAGE_FIELDS_TO_CHECK = Object.keys(SAMPLE_PAGE).filter(
  (field) => field !== 'kind' && field !== 'id' && field !== 'groupId',
) as (keyof typeof SAMPLE_PAGE)[]

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

  describe('runtime → .canvas → runtime', () => {
    for (const kind of KINDS) {
      it(`${kind} carries every declared field through a save and reload`, async () => {
        const sample = seedRuntime(kind)
        scheduleSpaceAutosave()
        await settleSync()
        harness.flush()

        reloadWorkspaceDataFromCurrentSpace()
        await settleSync()

        const reloaded = getEntityKind(kind)
          .entities()
          .find((entity) => (entity as { id: string }).id === sample.id) as
          | Record<string, unknown>
          | undefined
        expect(reloaded, `${kind} did not survive the save/reload at all`).toBeDefined()

        for (const field of declaredFields(kind)) {
          if (field === 'kind') continue
          expect(reloaded?.[field], `${kind}.${field} was lost saving to or loading from .canvas`)
            .toEqual(sample[field])
        }
      })
    }
  })

  // `page` and `group` are exercised on their own here rather than folded
  // into the generic loop above (see the field-list comments near the top of
  // this file for why their declared-field lists don't line up 1:1 with
  // their samples). Both kinds go through `restoreWorkspaceSnapshot`'s
  // hand-listed page/group arms on the way back from disk — this is the path
  // Step A collapses into the registry's `restore`.
  describe('runtime → .canvas → runtime (group and page)', () => {
    it('group carries every declared field through a save and reload', async () => {
      getEntityKind('group').restore([SAMPLE_GROUP])
      scheduleSpaceAutosave()
      await settleSync()
      harness.flush()

      reloadWorkspaceDataFromCurrentSpace()
      await settleSync()

      const reloaded = getEntityKind('group')
        .entities()
        .find((entity) => entity.id === SAMPLE_GROUP.id) as Record<string, unknown> | undefined
      expect(reloaded, 'group did not survive the save/reload at all').toBeDefined()

      for (const field of GROUP_FIELDS_TO_CHECK) {
        expect(reloaded?.[field], `group.${field} was lost saving to or loading from .canvas`)
          .toEqual(SAMPLE_GROUP[field])
      }
    })

    it('page carries every declared field through a save and reload', async () => {
      getEntityKind('page').restore([SAMPLE_PAGE])
      scheduleSpaceAutosave()
      await settleSync()
      harness.flush()

      reloadWorkspaceDataFromCurrentSpace()
      await settleSync()

      const reloaded = getEntityKind('page')
        .entities()
        .find((entity) => entity.id === SAMPLE_PAGE.id) as Record<string, unknown> | undefined
      expect(reloaded, 'page did not survive the save/reload at all').toBeDefined()

      for (const field of PAGE_FIELDS_TO_CHECK) {
        expect(reloaded?.[field], `page.${field} was lost saving to or loading from .canvas`)
          .toEqual(SAMPLE_PAGE[field])
      }
    })
  })

  // Tab switch swaps a tab's entities out to its `WorkspaceSnapshot` (via
  // `spaceSnapshot()` in space-tabs.ts) and back in (via
  // `restoreWorkspaceSnapshot` in space-restore.ts) without touching disk —
  // both hand-listed paths Step A collapses.
  describe('tab switch', () => {
    it('carries every declared field of every kind across a switch away and back', async () => {
      const originalTabId = activeSpaceTabId
      if (!originalTabId) throw new Error('harness has no active tab')

      for (const kind of KINDS) seedRuntime(kind)
      getEntityKind('group').restore([SAMPLE_GROUP])
      getEntityKind('page').restore([SAMPLE_PAGE])

      // createSpaceTab() syncs the current (original) tab's runtime state
      // into its snapshot via spaceSnapshot() before switching — the forward
      // half of the path under test — then switches to the new, empty tab.
      const otherTabId = createSpaceTab('other')
      expect(otherTabId).not.toBe(originalTabId)
      for (const kind of KINDS) {
        expect(getEntityKind(kind).entities(), `${kind} leaked into the new tab`).toHaveLength(0)
      }

      // Switching back rebuilds runtime state from the original tab's stored
      // snapshot via restoreWorkspaceSnapshot() — the reverse half.
      expect(setActiveSpaceTab(originalTabId)).toBe(true)

      for (const kind of KINDS) {
        const sample = MAP_BACKED_SAMPLES[kind] as unknown as Record<string, unknown>
        const restored = getEntityKind(kind)
          .entities()
          .find((entity) => entity.id === sample.id) as Record<string, unknown> | undefined
        expect(restored, `${kind} did not survive the tab switch at all`).toBeDefined()
        for (const field of declaredFields(kind)) {
          if (field === 'kind') continue
          expect(restored?.[field], `${kind}.${field} was lost switching tabs away and back`)
            .toEqual(sample[field])
        }
      }

      const restoredGroup = getEntityKind('group')
        .entities()
        .find((entity) => entity.id === SAMPLE_GROUP.id) as Record<string, unknown> | undefined
      expect(restoredGroup, 'group did not survive the tab switch at all').toBeDefined()
      for (const field of GROUP_FIELDS_TO_CHECK) {
        expect(restoredGroup?.[field], `group.${field} was lost switching tabs away and back`)
          .toEqual(SAMPLE_GROUP[field])
      }

      const restoredPage = getEntityKind('page')
        .entities()
        .find((entity) => entity.id === SAMPLE_PAGE.id) as Record<string, unknown> | undefined
      expect(restoredPage, 'page did not survive the tab switch at all').toBeDefined()
      for (const field of PAGE_FIELDS_TO_CHECK) {
        expect(restoredPage?.[field], `page.${field} was lost switching tabs away and back`)
          .toEqual(SAMPLE_PAGE[field])
      }
    })
  })
})
