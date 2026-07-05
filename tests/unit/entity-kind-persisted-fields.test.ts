/**
 * Persisted-field drift tests (ADR 0024 §5).
 *
 * Each kind declares its persisted field list once, next to its persist
 * projection; the reverse sync restores from the same list. These tests pin
 * the remaining unchecked edge: the persist projection's output keys must
 * exactly match the declared list. The other direction (list vs persisted
 * type) is compile-enforced by the `satisfies Record<keyof Persisted…, true>`
 * field sets in the state modules.
 *
 * Mutation-verified by dropping `label` from `persistTextEntity` — the text
 * case fails with the missing key.
 *
 * The persist functions write every key unconditionally (optional fields as
 * explicit `undefined`), so a sample with only required fields still exposes
 * the full key set.
 */

import { describe, expect, it } from 'vitest'
import {
  persistTextEntity,
  TEXT_ENTITY_PERSISTED_FIELDS,
  type TextEntity,
} from '../../src/main/runtime/text-entity-state'
import {
  persistFileEntity,
  FILE_ENTITY_PERSISTED_FIELDS,
  type FileEntity,
} from '../../src/main/runtime/file-entity-state'
import {
  persistDrawingEntity,
  DRAWING_ENTITY_PERSISTED_FIELDS,
  type DrawingEntity,
} from '../../src/main/runtime/drawing-entity-state'
import {
  persistShapeEntity,
  SHAPE_ENTITY_PERSISTED_FIELDS,
  type ShapeEntity,
} from '../../src/main/runtime/shape-entity-state'
import {
  persistPage,
  PAGE_PERSISTED_FIELDS,
} from '../../src/main/runtime/page-doc-projection'
import type { Page } from '../../src/main/runtime/runtime-entities'

function keysOf(record: Record<string, unknown>): string[] {
  return Object.keys(record).sort()
}

describe('persist projections match the declared persisted field lists', () => {
  it('text', () => {
    const entity: TextEntity = {
      id: 't1',
      text: 'hello',
      color: '#FFE18E',
      textStyle: 'sticky',
      widthMode: 'fixed',
      canvasX: 0,
      canvasY: 0,
      width: 200,
      height: 200,
    }
    expect(keysOf(persistTextEntity(entity))).toEqual([...TEXT_ENTITY_PERSISTED_FIELDS].sort())
  })

  it('file', () => {
    const entity: FileEntity = {
      id: 'f1',
      file: 'notes/readme.md',
      canvasX: 0,
      canvasY: 0,
      width: 300,
      height: 300,
    }
    expect(keysOf(persistFileEntity(entity))).toEqual([...FILE_ENTITY_PERSISTED_FIELDS].sort())
  })

  it('drawing', () => {
    const entity: DrawingEntity = {
      id: 'd1',
      canvasX: 0,
      canvasY: 0,
      width: 200,
      height: 200,
      strokes: [],
    }
    expect(keysOf(persistDrawingEntity(entity))).toEqual([...DRAWING_ENTITY_PERSISTED_FIELDS].sort())
  })

  it('shape', () => {
    const entity: ShapeEntity = {
      id: 's1',
      shapeKind: 'rectangle',
      text: '',
      canvasX: 0,
      canvasY: 0,
      width: 200,
      height: 120,
    }
    expect(keysOf(persistShapeEntity(entity))).toEqual([...SHAPE_ENTITY_PERSISTED_FIELDS].sort())
  })

  it('page', () => {
    const page = {
      id: 'p1',
      url: 'http://localhost:3000/',
      presetIndex: 1,
      canvasX: 0,
      canvasY: 0,
      linked: false,
      source: 'user',
      syncState: {
        suppressNavigationBroadcastUntil: 0,
        suppressNextScrollBroadcastUntil: 0,
      },
    } as unknown as Page
    expect(keysOf(persistPage(page))).toEqual([...PAGE_PERSISTED_FIELDS].sort())
  })
})
