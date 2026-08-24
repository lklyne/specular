/**
 * useAnchoredPosition — turn an entityId into overlay-local screen coords.
 *
 * All canvas-anchored overlay UI in aboveView positions itself through this
 * hook. The hook reads geometry from the layout broadcast aboveView already
 * subscribes to.
 *
 * Coords returned are **overlay-local**: aboveView's WCV origin sits at
 * `canvasOrigin.y` (below the toolbar), so we subtract that from window-space
 * `screenY`. Consumers can drop the rect straight into `style.left/top/...`.
 *
 * The entity rect is the entity's body rect — `screenX/screenY/screenWidth/
 * screenHeight` from the layout broadcast. There is no separate chrome band.
 */

import type { ProjectedGroupEntity, ProjectedLayoutData, ProjectedSceneEntity } from '../../shared/scene-projection'
import { useMemo } from 'react'
import type { Rect } from '../../shared/hit-regions'

export interface AnchoredRect extends Rect {}

export function anchoredRect(
  layout: ProjectedLayoutData,
  entityId: string,
): AnchoredRect | null {
  const entity = findAnchorTarget(layout, entityId)
  if (!entity) return null
  return toOverlayLocal(entityRectFor(entity), layout)
}

export function useAnchoredPosition(
  layout: ProjectedLayoutData,
  entityId: string,
): AnchoredRect | null {
  return useMemo(() => anchoredRect(layout, entityId), [layout, entityId])
}

/**
 * Multi-entity union rect for same-kind multi-select popups (ADR 0008 §4).
 * Returns the bounding box of every resolved entity's rect. The popup anchors
 * against this union so it visually spans the selection.
 *
 * Returns `null` only when `entityIds` is empty. Off-screen entities still
 * contribute their rect — the popup mounts at the (possibly clipped) bbox
 * edge by design.
 */
export function useMultiAnchoredPosition(
  layout: ProjectedLayoutData,
  entityIds: readonly string[],
): AnchoredRect | null {
  const key = entityIds.join('|')
  return useMemo(() => {
    if (entityIds.length === 0) return null
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    let any = false
    for (const id of entityIds) {
      const rect = anchoredRect(layout, id)
      if (!rect) continue
      any = true
      if (rect.x < minX) minX = rect.x
      if (rect.y < minY) minY = rect.y
      if (rect.x + rect.width > maxX) maxX = rect.x + rect.width
      if (rect.y + rect.height > maxY) maxY = rect.y + rect.height
    }
    if (!any) return null
    return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
  }, [layout, key])
}

type AnchorTarget = ProjectedSceneEntity | ProjectedGroupEntity

function findAnchorTarget(layout: ProjectedLayoutData, id: string): AnchorTarget | undefined {
  const entity = layout.entities.find((e) => e.id === id)
  if (entity) return entity
  return (layout.groups ?? []).find((g) => g.id === id)
}

function entityRectFor(entity: AnchorTarget): Rect {
  return {
    x: entity.screenX,
    y: entity.screenY,
    width: entity.screenWidth,
    height: entity.screenHeight,
  }
}

function toOverlayLocal(rect: Rect, layout: ProjectedLayoutData): Rect {
  return {
    x: rect.x,
    y: rect.y - layout.canvasOrigin.y,
    width: rect.width,
    height: rect.height,
  }
}
