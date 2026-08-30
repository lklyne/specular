/**
 * Fully-populated sample entities, one per canvas-item kind.
 *
 * These exist to catch field drift in the paths that COPY entity fields —
 * disk save/load, snapshot hydrate, copy/paste, duplicate. The declaration
 * itself is already guarded (`entity-kind-persisted-fields.test.ts` pins each
 * persist projection against its declared list, and the `satisfies Record<keyof
 * Persisted…, true>` field sets pin the list against the type). What was
 * unguarded is whether each copy path carries the fields it declares.
 *
 * Every sample is typed `satisfies Required<Persisted…Entity>`, so declaring a
 * new persisted field is a COMPILE ERROR here until the sample sets it. That
 * is the property that makes this a net rather than one more list to
 * remember: a runtime builder derived from `fields` could not do it, having no
 * way to invent a valid `strokes` array or `file` path.
 *
 * Values are deliberately distinctive (no defaults, no empty strings) so a
 * path that drops a field to its default is caught by value, not just by key.
 */

import type {
  PersistedDrawingEntity,
  PersistedFileEntity,
  PersistedGroupEntity,
  PersistedPageEntity,
  PersistedShapeEntity,
  PersistedTextEntity,
} from '../../src/shared/types'

const pageAnchor = { pageId: 'page_sample', pageUrl: 'https://example.test/doc' }

const SAMPLE_TEXT = {
  kind: 'text',
  id: 'sample_text',
  text: 'sample text body',
  color: '5',
  canvasX: 120,
  canvasY: 240,
  width: 260,
  height: 180,
  textStyle: 'plain',
  widthMode: 'auto',
  textSize: 24,
  textFont: 'hand',
  label: 'Sample text label',
  parentGroupId: 'group_sample',
  pageAnchor,
} satisfies Required<PersistedTextEntity>

const SAMPLE_FILE = {
  kind: 'file',
  id: 'sample_file',
  file: 'notes/sample.md',
  subpath: '#heading',
  canvasX: 300,
  canvasY: 420,
  width: 320,
  height: 240,
  objectFit: 'cover',
  presetIndex: 2,
  metadata: { sampleKey: 'sampleValue' },
  parentGroupId: 'group_sample',
} satisfies Required<PersistedFileEntity>

const SAMPLE_DRAWING = {
  kind: 'drawing',
  id: 'sample_drawing',
  canvasX: 60,
  canvasY: 80,
  width: 140,
  height: 120,
  strokes: [
    {
      id: 'stroke_sample',
      color: '2',
      width: 4,
      points: [{ x: 1, y: 2 }, { x: 3, y: 4 }],
      brushType: 'pen',
    },
  ] as PersistedDrawingEntity['strokes'],
  label: 'Sample drawing label',
  parentGroupId: 'group_sample',
  pageAnchor,
} satisfies Required<PersistedDrawingEntity>

const SAMPLE_SHAPE = {
  kind: 'shape',
  id: 'sample_shape',
  shapeKind: 'rounded',
  text: 'sample shape label',
  color: '6',
  fillStyle: 'none',
  strokeWidth: 3,
  borderStyle: 'dashed',
  borderColor: '4',
  textSize: 18,
  textAlign: 'left',
  textVerticalAlign: 'top',
  theme: 'dark',
  canvasX: 500,
  canvasY: 640,
  width: 220,
  height: 160,
  label: 'Sample shape label',
  parentGroupId: 'group_sample',
  pageAnchor,
} satisfies Required<PersistedShapeEntity>

export const SAMPLE_GROUP = {
  kind: 'group',
  id: 'sample_group',
  label: 'Sample group',
  color: '1',
  canvasX: 20,
  canvasY: 40,
  width: 800,
  height: 600,
  layoutMode: 'row',
  managedLayout: true,
  layoutGap: 40,
  sourceTaskId: 'task_sample',
  metadata: { sampleKey: 'sampleValue' },
  parentGroupId: 'group_outer',
} satisfies Required<PersistedGroupEntity>

export const SAMPLE_PAGE = {
  kind: 'page',
  id: 'sample_page',
  name: 'Sample page',
  url: 'https://example.test/page',
  presetIndex: 3,
  canvasX: 900,
  canvasY: 100,
  syncId: 'sync_sample',
  source: 'manual',
  groupId: 'group_sample',
  parentGroupId: 'group_sample',
  metadata: { sampleKey: 'sampleValue' },
  colorScheme: 'dark',
} satisfies Required<PersistedPageEntity>

/**
 * Keyed by kind. `page` is excluded from the map-backed round trips: it is
 * WebContentsView-backed and mirrors to the pages map through `persistPage`,
 * so it is exercised by its own cases rather than the generic loop.
 */
export const MAP_BACKED_SAMPLES = {
  text: SAMPLE_TEXT,
  file: SAMPLE_FILE,
  drawing: SAMPLE_DRAWING,
  shape: SAMPLE_SHAPE,
} as const

export type MapBackedKind = keyof typeof MAP_BACKED_SAMPLES

/**
 * Second, fully-distinct value for every declared field of each map-backed
 * kind — a patch to apply over `MAP_BACKED_SAMPLES` via
 * `getEntityKind(kind).update()`. Exists to catch the mutation-side flavor of
 * field drift (docs/plans/entity-field-drift.md, Step C): a registry `update`
 * handler that casts patch fields one by one silently ignores any field it
 * forgot to name, even though the field persists and loads correctly. Every
 * value differs from the matching `SAMPLE_*` value (including `id`, so a path
 * that accidentally re-keys is also visible) so the round-trip check can tell
 * "field took the patch value" apart from "field kept its old value".
 */
const updatedPageAnchor = { pageId: 'page_updated', pageUrl: 'https://example.test/updated' }

// `update` grid-snaps canvasX/canvasY/width/height (src/main/runtime/
// document-commands.ts, snapGeometryPatch — GRID_SIZE is 20), so every
// geometry value here is a multiple of 20. Otherwise the snap would round the
// patched value away from what the test asserts, which is a snapping quirk,
// not the field-drift bug this net is for.
const SAMPLE_TEXT_UPDATE = {
  text: 'updated text body',
  color: '2',
  canvasX: 1000,
  canvasY: 900,
  width: 320,
  height: 200,
  textStyle: 'sticky',
  widthMode: 'fixed',
  textSize: 32,
  textFont: 'mono',
  label: 'Updated text label',
  parentGroupId: 'group_updated',
  pageAnchor: updatedPageAnchor,
} satisfies Omit<Required<PersistedTextEntity>, 'kind' | 'id'>

const SAMPLE_FILE_UPDATE = {
  file: 'notes/updated.md',
  subpath: '#updated-heading',
  canvasX: 780,
  canvasY: 660,
  // File entities snap position only, not size (updateFileEntity passes an
  // explicit snapKeys of just canvasX/canvasY) — width/height need no
  // grid-alignment.
  width: 555,
  height: 444,
  objectFit: 'contain',
  // Deliberately out of `VIEWPORT_PRESETS` range: a valid index would make
  // `setFilePreset` (the richer device-preset verb `fileKind.update` calls
  // after the raw field copy) overwrite width/height with the device's
  // viewport size, which would fail the width/height checks above for a
  // reason unrelated to what this case tests — whether the raw `presetIndex`
  // field itself reaches the entity.
  presetIndex: 999,
  metadata: { updatedKey: 'updatedValue' },
  parentGroupId: 'group_updated',
} satisfies Omit<Required<PersistedFileEntity>, 'kind' | 'id'>

const SAMPLE_DRAWING_UPDATE = {
  canvasX: 120,
  canvasY: 220,
  width: 340,
  height: 440,
  strokes: [
    {
      id: 'stroke_updated',
      color: '7',
      width: 8,
      points: [{ x: 9, y: 10 }, { x: 11, y: 12 }],
      brushType: 'marker',
    },
  ] as PersistedDrawingEntity['strokes'],
  label: 'Updated drawing label',
  parentGroupId: 'group_updated',
  pageAnchor: updatedPageAnchor,
} satisfies Omit<Required<PersistedDrawingEntity>, 'kind' | 'id'>

const SAMPLE_SHAPE_UPDATE = {
  shapeKind: 'ellipse',
  text: 'updated shape label',
  color: '3',
  fillStyle: 'solid',
  strokeWidth: 6,
  borderStyle: 'solid',
  borderColor: '2',
  textSize: 22,
  textAlign: 'right',
  textVerticalAlign: 'bottom',
  theme: 'light',
  canvasX: 1200,
  canvasY: 1300,
  width: 400,
  height: 300,
  label: 'Updated shape label',
  parentGroupId: 'group_updated',
  pageAnchor: updatedPageAnchor,
} satisfies Omit<Required<PersistedShapeEntity>, 'kind' | 'id'>

/** Keyed by kind, mirroring `MAP_BACKED_SAMPLES`. */
export const MAP_BACKED_UPDATE_PATCHES = {
  text: SAMPLE_TEXT_UPDATE,
  file: SAMPLE_FILE_UPDATE,
  drawing: SAMPLE_DRAWING_UPDATE,
  shape: SAMPLE_SHAPE_UPDATE,
} as const
