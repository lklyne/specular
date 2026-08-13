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

export const SAMPLE_TEXT = {
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
  label: 'Sample text label',
  parentGroupId: 'group_sample',
  pageAnchor,
} satisfies Required<PersistedTextEntity>

export const SAMPLE_FILE = {
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

export const SAMPLE_DRAWING = {
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

export const SAMPLE_SHAPE = {
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
