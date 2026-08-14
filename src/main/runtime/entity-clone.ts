/**
 * Clone one map-backed entity (text/file/drawing/shape) through the
 * entity-kind registry's own persist/restore pair (ADR 0024 §5), instead of
 * restating its field list at each call site.
 *
 * Copy/paste and group-duplicate both used to hand-list "what a clone
 * carries" once per kind, and those lists drifted from persistence — copying
 * a styled shape lost its border style, fill, and text alignment
 * (docs/plans/entity-field-drift.md, Step B). A clone built from the same
 * persisted record persistence itself produces cannot drop a field that
 * persistence carries: there is only one field list to fall behind.
 */

import { getEntityKind } from '../entities/contract'
import { markDirty } from './layout-dirty'
import { watchEntityFile } from './local-file-watcher'
import type { AnnotationDrawingStroke } from '../../shared/types'

export type MapBackedEntityKind = 'text' | 'file' | 'drawing' | 'shape'

export interface EntityCloneOverrides {
  id: string
  canvasX: number
  canvasY: number
  parentGroupId?: string
}

/**
 * Clone `record` — a persisted record of `kind`, as produced by
 * `getEntityKind(kind).persist()` — into a new live entity of the same kind.
 *
 * `id`, `parentGroupId`, and `pageAnchor` are always replaced by `overrides`
 * (`pageAnchor` is dropped unconditionally): group membership and page
 * anchoring are placement decisions the caller makes, not fields to copy
 * verbatim. Paste re-attaches an anchor afterward when the source page was
 * copied alongside it (ADR 0031); group-duplicate never re-anchors its
 * children. `record.canvasX`/`canvasY` are read once, before being
 * overwritten, to compute the placement delta applied to drawing's embedded
 * stroke points — the one piece of position data that lives inside a field
 * rather than being a field itself.
 */
export function cloneMapBackedEntity(
  kind: MapBackedEntityKind,
  record: Record<string, unknown>,
  overrides: EntityCloneOverrides,
): Record<string, unknown> {
  const kindDef = getEntityKind(kind)
  const oldX = (record.canvasX as number | undefined) ?? 0
  const oldY = (record.canvasY as number | undefined) ?? 0
  const deltaX = overrides.canvasX - oldX
  const deltaY = overrides.canvasY - oldY

  const { kind: _kind, ...rest } = record
  const clone: Record<string, unknown> = {
    ...rest,
    id: overrides.id,
    canvasX: overrides.canvasX,
    canvasY: overrides.canvasY,
    parentGroupId: overrides.parentGroupId,
    pageAnchor: undefined,
  }

  if (kind === 'drawing' && Array.isArray(clone.strokes)) {
    clone.strokes = (clone.strokes as AnnotationDrawingStroke[]).map((stroke) => ({
      ...stroke,
      id: `${stroke.id}_clone_${Math.random().toString(36).slice(2, 8)}`,
      points: stroke.points.map((point) => ({ x: point.x + deltaX, y: point.y + deltaY })),
    }))
  }

  kindDef.restore([
    ...(kindDef.entities() as unknown as Record<string, unknown>[]),
    clone,
  ])
  markDirty('canvas', 'sidebar')

  if (kind === 'file' && typeof clone.file === 'string') {
    watchEntityFile(overrides.id, clone.file)
  }

  return clone
}
