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

/** Push one clone onto its kind's live array through the registry's own
 *  `restore` — the one seam both `cloneMapBackedEntity` and `cloneGroupEntity`
 *  push through, so there is exactly one "append a clone" shape to drift. */
function appendClone(
  kind: MapBackedEntityKind | 'group',
  clone: Record<string, unknown>,
): void {
  const kindDef = getEntityKind(kind)
  kindDef.restore([
    ...(kindDef.entities() as unknown as Record<string, unknown>[]),
    clone,
  ])
  markDirty('canvas', 'sidebar')
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

  appendClone(kind, clone)

  if (kind === 'file' && typeof clone.file === 'string') {
    watchEntityFile(overrides.id, clone.file)
  }

  return clone
}

/**
 * Clone a persisted group record (`getEntityKind('group').persist()`) into a
 * new live group, mirroring `cloneMapBackedEntity`'s persist → re-id → offset
 * → restore shape (ADR 0034, "Groups become copyable"). `parentGroupId` is
 * always replaced by `overrides` — group membership is remapped by the
 * caller in a second pass once every clone in a batch has a new id (a
 * group's own parent may itself be mid-clone). Unlike a map-backed entity, a
 * group carries no embedded position data (no stroke points to shift), so no
 * delta computation is needed here.
 */
export function cloneGroupEntity(
  record: Record<string, unknown>,
  overrides: EntityCloneOverrides,
): Record<string, unknown> {
  const clone: Record<string, unknown> = {
    ...record,
    id: overrides.id,
    canvasX: overrides.canvasX,
    canvasY: overrides.canvasY,
    parentGroupId: overrides.parentGroupId,
  }
  appendClone('group', clone)
  return clone
}
