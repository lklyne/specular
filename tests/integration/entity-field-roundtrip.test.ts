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
 *   - copy/paste, duplicate, and group-duplicate (Step B: workspace-clipboard.ts
 *     and workspace-groups.ts each hand-listed a clone's fields per kind and had
 *     drifted — copying a styled shape lost its border style, fill, and text
 *     alignment)
 *   - update (Step C: each registry `update` handler cast patch fields onto its
 *     state mutator one by one and had drifted — `textStyle`/`parentGroupId`/
 *     `label`/`pageAnchor` on text, `label`/`parentGroupId`/`pageAnchor` on
 *     shape, `parentGroupId`/`label`/`pageAnchor` on drawing, and
 *     `parentGroupId` on file all persisted and loaded correctly but did
 *     nothing when patched through `getEntityKind(kind).update()`)
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
  copyableEntityPayload,
  pasteEntitiesFromClipboard,
} from '../../src/main/workspace-clipboard'
import { duplicateEntity } from '../../src/main/workspace-pages'
import { duplicateGroup } from '../../src/main/workspace-groups'
import type { WorkspaceGroup } from '../../src/shared/types'
import {
  MAP_BACKED_SAMPLES,
  MAP_BACKED_UPDATE_PATCHES,
  SAMPLE_GROUP,
  SAMPLE_PAGE,
  type MapBackedKind,
} from './entity-field-fixtures'

/**
 * Fields a clone path (copy/paste, duplicate, group-duplicate) is expected to
 * recompute rather than carry verbatim: `id` is always new, `canvasX`/
 * `canvasY` move by the paste/duplicate offset, `pageAnchor` is re-resolved
 * against live pages rather than copied (ADR 0031 — page-anchor re-targeting
 * on paste is placement logic, not field copying; docs/plans/
 * entity-field-drift.md, Step B), and plain copy/paste (not group-duplicate)
 * intentionally drops `parentGroupId` — pasting does not imply rejoining a
 * group. `strokes` is checked separately for `drawing` because its points
 * are placement data embedded in the field, not a scalar.
 */
const PLACEMENT_FIELDS = new Set(['kind', 'id', 'canvasX', 'canvasY', 'pageAnchor', 'strokes'])

function nonPlacementFields(kind: MapBackedKind): string[] {
  return declaredFields(kind).filter((field) => !PLACEMENT_FIELDS.has(field))
}

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
 *  Deliberately not `create`: create's defaulting logic isn't what this net
 *  covers, and seeding the store is the state a live entity is in by the time
 *  any copy or update path reads it. Restores a deep clone, not the shared
 *  `SAMPLE_*` object itself — `restore` pushes its argument by reference, and
 *  the `update` cases below mutate the seeded entity in place; without the
 *  clone, updating a seeded `text` would permanently overwrite the module-level
 *  `SAMPLE_TEXT` fixture for every test that runs afterward in this file. */
function seedRuntime(kind: MapBackedKind): Record<string, unknown> {
  const sample = structuredClone(MAP_BACKED_SAMPLES[kind]) as unknown as Record<string, unknown>
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

  /**
   * The mutation-side flavor of field drift (docs/plans/entity-field-drift.md,
   * Step C): each registry `update` handler cast patch fields onto its state
   * mutator one by one instead of forwarding the whole patch, so a field could
   * persist and load correctly yet do nothing when an agent or the details
   * panel set it through `getEntityKind(kind).update()`. Unlike the copy paths
   * above, `update` legitimately changes `parentGroupId` and `pageAnchor` (an
   * agent moving an entity into a group, or a details-panel edit re-anchoring
   * it), so — unlike `nonPlacementFields` — every declared field except
   * `kind`/`id` is checked here.
   *
   * Mutation-verified: reverting `textKind.update` (src/main/entities/builtin/
   * text.ts) to its pre-fix hand-picked field list (dropping `textStyle`,
   * `parentGroupId`, `label`, `pageAnchor`) fails the text case below with
   * those four fields unchanged from the seed value; reverting
   * `shapeKind.update` similarly (dropping `label`, `parentGroupId`,
   * `pageAnchor`) fails the shape case; reverting `drawingKind.update`
   * (dropping `parentGroupId`, `label`, `pageAnchor`) fails the drawing case;
   * reverting `fileKind.update` (dropping `parentGroupId`) fails the file
   * case.
   */
  describe('update', () => {
    for (const kind of KINDS) {
      it(`${kind} update patches every declared field except id/kind`, () => {
        const sample = seedRuntime(kind)
        const patch = MAP_BACKED_UPDATE_PATCHES[kind] as unknown as Record<string, unknown>

        getEntityKind(kind).update(sample.id as string, patch, {})

        const updated = getEntityKind(kind)
          .entities()
          .find((entity) => entity.id === sample.id) as Record<string, unknown> | undefined
        expect(updated, `${kind} disappeared after update`).toBeDefined()

        for (const field of declaredFields(kind)) {
          if (field === 'kind' || field === 'id') continue
          expect(updated?.[field], `${kind}.${field} did not take effect via update`)
            .toEqual(patch[field])
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

  /**
   * Copy/paste clones a source entity from its own persisted record
   * (`getEntityKind(kind).persist()`) via `cloneMapBackedEntity`
   * (src/main/runtime/entity-clone.ts), reused by `pasteEntitiesFromClipboard`
   * (src/main/workspace-clipboard.ts). Before that refactor each kind's
   * clipboard payload hand-listed its own fields and had drifted: copying a
   * styled shape lost `borderStyle`, `fillStyle`, `borderColor`, `textAlign`,
   * and `textVerticalAlign` (docs/plans/entity-field-drift.md, Step B).
   *
   * `parentGroupId` and `pageAnchor` are deliberately excluded from the
   * declared-field check — paste does not rejoin the source's group (that's
   * group-duplicate, covered below) and re-resolves the anchor against live
   * pages rather than copying it (ADR 0031 placement logic, left as-is).
   *
   * Mutation-verified: reverting `mapBackedPayload` in workspace-clipboard.ts
   * to hand-list shape's old field set (`shapeKind`, `text`, `color`,
   * `strokeWidth`, `textSize`, `theme`, `label` — omitting `fillStyle`,
   * `borderStyle`, `borderColor`, `textAlign`, `textVerticalAlign`) fails the
   * shape case below with those five fields `undefined` on the pasted clone.
   */
  describe('copy/paste', () => {
    for (const kind of KINDS) {
      it(`${kind} copy/paste carries every declared field except placement`, () => {
        const sample = seedRuntime(kind)
        const payload = copyableEntityPayload([sample.id as string])
        expect(payload, `${kind} did not produce a clipboard payload`).toBeTruthy()

        const { entityIds } = pasteEntitiesFromClipboard({
          payload: payload!,
          canvasX: 5000,
          canvasY: 5000,
        })
        expect(entityIds).toHaveLength(1)
        const pastedId = entityIds[0]
        expect(pastedId).not.toBe(sample.id)

        const pasted = getEntityKind(kind)
          .entities()
          .find((entity) => entity.id === pastedId) as Record<string, unknown> | undefined
        expect(pasted, `${kind} was not created by paste`).toBeDefined()

        expect(pasted?.parentGroupId, `${kind} paste unexpectedly kept its source group`)
          .toBeUndefined()

        for (const field of nonPlacementFields(kind)) {
          if (field === 'parentGroupId') continue
          expect(pasted?.[field], `${kind}.${field} was lost pasting from the clipboard`)
            .toEqual(sample[field])
        }

        if (kind === 'drawing') {
          assertDrawingStrokesShifted(
            sample,
            pasted!,
            5000 - (sample.canvasX as number),
            5000 - (sample.canvasY as number),
          )
        }
      })
    }
  })

  /**
   * Duplicate (cmd-D on a single entity) reuses the exact same clone path as
   * copy/paste — `duplicateEntity` (src/main/workspace-pages.ts) builds a
   * clipboard payload via `copyableEntityPayload` and pastes it via
   * `pasteEntitiesFromClipboard` — so it earns coverage for the same reason.
   */
  describe('duplicate', () => {
    for (const kind of KINDS) {
      it(`${kind} duplicate carries every declared field except placement`, () => {
        const sample = seedRuntime(kind)
        const { entityId } = duplicateEntity({ entityId: sample.id as string })
        expect(entityId).not.toBe(sample.id)

        const duplicated = getEntityKind(kind)
          .entities()
          .find((entity) => entity.id === entityId) as Record<string, unknown> | undefined
        expect(duplicated, `${kind} was not created by duplicate`).toBeDefined()

        for (const field of nonPlacementFields(kind)) {
          if (field === 'parentGroupId') continue
          expect(duplicated?.[field], `${kind}.${field} was lost duplicating`)
            .toEqual(sample[field])
        }
      })
    }
  })

  /**
   * Group-duplicate clones a group's children through the same
   * persist→re-id→offset→restore path as copy/paste (`cloneMapBackedEntity`,
   * reused by `duplicateGroupInternal` in src/main/workspace-groups.ts)
   * rather than its own hand-listed `create*EntityInState` call per child
   * kind. Unlike plain copy/paste, a group-duplicated child DOES rejoin its
   * (cloned) group — that's the point of duplicating a group.
   *
   * Mutation-verified: reverting the group-child clone loop in
   * workspace-groups.ts to hand-list shape's old field set (dropping
   * `fillStyle`/`borderStyle`/`borderColor`/`textAlign`/`textVerticalAlign`,
   * as the pre-Step-B `createShapeEntityInState` call did) fails the shape
   * case below the same way copy/paste's mutation does.
   */
  describe('group duplicate', () => {
    it('carries every declared field of every map-backed child kind except placement', () => {
      const testGroup: WorkspaceGroup = {
        id: 'group_sample',
        kind: 'group',
        label: 'Test group',
        canvasX: 0,
        canvasY: 0,
        width: 800,
        height: 600,
        layoutMode: 'freeform',
        managedLayout: false,
      }
      getEntityKind('group').restore([testGroup])
      // Every MAP_BACKED_SAMPLES fixture already declares
      // `parentGroupId: 'group_sample'` (entity-field-fixtures.ts), so
      // seeding them here makes them children of `testGroup`.
      for (const kind of KINDS) seedRuntime(kind)

      const { groupId, entityIds } = duplicateGroup({ groupId: 'group_sample' })
      expect(groupId).not.toBe('group_sample')
      expect(entityIds).toHaveLength(KINDS.length)

      const clonedGroup = getEntityKind('group')
        .entities()
        .find((entity) => entity.id === groupId) as Record<string, unknown> | undefined
      expect(clonedGroup, 'the duplicated group itself is missing').toBeDefined()
      const offsetX = (clonedGroup?.canvasX as number) - testGroup.canvasX
      const offsetY = (clonedGroup?.canvasY as number) - testGroup.canvasY

      for (const kind of KINDS) {
        const sample = MAP_BACKED_SAMPLES[kind] as unknown as Record<string, unknown>
        const clones = getEntityKind(kind)
          .entities()
          .filter((entity) => entity.id !== sample.id) as unknown as Record<string, unknown>[]
        expect(clones, `${kind} was not duplicated with its group`).toHaveLength(1)
        const clone = clones[0]

        expect(clone.parentGroupId, `${kind} clone did not join the duplicated group`).toBe(groupId)

        for (const field of nonPlacementFields(kind)) {
          if (field === 'parentGroupId') continue
          expect(clone[field], `${kind}.${field} was lost duplicating its group`)
            .toEqual(sample[field])
        }

        if (kind === 'drawing') {
          assertDrawingStrokesShifted(sample, clone, offsetX, offsetY)
        }
      }
    })
  })
})

/** A drawing sample's strokes, typed loosely for the shift assertion below. */
type SampleStroke = {
  color: string
  width: number
  brushType: string
  points: { x: number; y: number }[]
}

/**
 * Asserts a cloned drawing's strokes kept every non-positional field and
 * shifted every point by exactly `(deltaX, deltaY)` — the one field whose
 * position data lives inside an array rather than being a scalar `canvasX`/
 * `canvasY`, so it needs its own check instead of a plain `toEqual`.
 */
function assertDrawingStrokesShifted(
  source: Record<string, unknown>,
  clone: Record<string, unknown>,
  deltaX: number,
  deltaY: number,
): void {
  const sourceStrokes = source.strokes as SampleStroke[]
  const cloneStrokes = clone.strokes as SampleStroke[]
  expect(cloneStrokes, 'drawing clone lost its strokes').toHaveLength(sourceStrokes.length)
  sourceStrokes.forEach((sourceStroke, i) => {
    const cloneStroke = cloneStrokes[i]
    expect(cloneStroke.color).toBe(sourceStroke.color)
    expect(cloneStroke.width).toBe(sourceStroke.width)
    expect(cloneStroke.brushType).toBe(sourceStroke.brushType)
    sourceStroke.points.forEach((point, j) => {
      expect(cloneStroke.points[j].x).toBeCloseTo(point.x + deltaX)
      expect(cloneStroke.points[j].y).toBeCloseTo(point.y + deltaY)
    })
  })
}
