/**
 * useAnchoredPosition — turn (entityId, slot) into overlay-local screen coords.
 *
 * Per ADR 0002 §2, all canvas-anchored overlay UI in aboveView positions
 * itself through this hook. The hook reads geometry from the layout broadcast
 * aboveView already subscribes to and applies the per-kind chrome slot
 * geometry from `entity-chrome-slots`.
 *
 * Coords returned are **overlay-local**: aboveView's WCV origin sits at
 * `canvasOrigin.y` (below the toolbar), so we subtract that from window-space
 * `screenY`. Consumers can drop the rect straight into `style.left/top/...`.
 *
 * Today's scene entities expose `screenX/screenY/screenWidth/screenHeight`
 * representing the **body** rect; chrome lives above. ADR §1 unifies this
 * (entity rect = body + chrome stacked). When that migration lands, only
 * `entityRectFor` below changes — every consumer keeps working.
 */

import { useMemo } from 'react'
import {
  CHROME_HEADER_HEIGHT,
  entityChromeSlots,
  type ChromeSlotName,
} from '../../shared/entity-chrome-slots'
import type { Rect } from '../../shared/hit-regions'
import { entityVisualScreenRect } from '../../shared/coords'
import type {
  CanvasSceneEntity,
  CanvasSceneGroupEntity,
  LayoutUpdateData,
} from '../../shared/types'

export type AnchorSlot = ChromeSlotName | 'body'

export interface AnchoredRect extends Rect {}

export function anchoredSlotRect(
  layout: LayoutUpdateData,
  entityId: string,
  slot: AnchorSlot,
): AnchoredRect | null {
  const entity = findAnchorTarget(layout, entityId)
  if (!entity) return null
  const entityRect = entityRectFor(entity, layout)
  const layoutResult = entityChromeSlots(entity.kind, entityRect)
  const rect =
    slot === 'body'
      ? layoutResult.body
      : layoutResult.slots.find((s) => s.name === slot)?.rect
  if (!rect) return null
  return toOverlayLocal(rect, layout)
}

export function useAnchoredPosition(
  layout: LayoutUpdateData,
  entityId: string,
  slot: AnchorSlot,
): AnchoredRect | null {
  return useMemo(() => anchoredSlotRect(layout, entityId, slot), [layout, entityId, slot])
}

/**
 * Multi-entity union rect for same-kind multi-select popups (ADR 0008 §4).
 * Returns the bounding box of every resolved entity's slot rect. The popup
 * anchors against this union so it visually spans the selection.
 *
 * Returns `null` only when `entityIds` is empty. Off-screen entities still
 * contribute their rect — the popup mounts at the (possibly clipped) bbox
 * edge by design.
 */
export function useMultiAnchoredPosition(
  layout: LayoutUpdateData,
  entityIds: readonly string[],
  slot: AnchorSlot,
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
      const rect = anchoredSlotRect(layout, id, slot)
      if (!rect) continue
      any = true
      if (rect.x < minX) minX = rect.x
      if (rect.y < minY) minY = rect.y
      if (rect.x + rect.width > maxX) maxX = rect.x + rect.width
      if (rect.y + rect.height > maxY) maxY = rect.y + rect.height
    }
    if (!any) return null
    return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
  }, [layout, key, slot])
}

type AnchorTarget = CanvasSceneEntity | CanvasSceneGroupEntity

function findAnchorTarget(layout: LayoutUpdateData, id: string): AnchorTarget | undefined {
  const entity = layout.entities.find((e) => e.id === id)
  if (entity) return entity
  return (layout.groups ?? []).find((g) => g.id === id)
}

/**
 * Returns the unified entity rect (body + chrome) in window-space coords.
 *
 * `entity.screenY` is the snap-rect top — bezel top for framed pages, body
 * top otherwise. Chrome lives `CHROME_HEADER_HEIGHT` above it; extend the
 * rect upward to include the strip for kinds that have chrome.
 */
function entityRectFor(entity: AnchorTarget, layout: LayoutUpdateData): Rect {
  const hasHeader = entity.kind === 'page' || entity.kind === 'file' || entity.kind === 'group'
  const headerExtension = hasHeader ? CHROME_HEADER_HEIGHT : 0
  const vb = entityVisualScreenRect(entity, layout)
  return {
    x: vb.screenX,
    y: vb.screenY - headerExtension,
    width: vb.screenWidth,
    height: vb.screenHeight + headerExtension,
  }
}

function toOverlayLocal(rect: Rect, layout: LayoutUpdateData): Rect {
  return {
    x: rect.x,
    y: rect.y - layout.canvasOrigin.y,
    width: rect.width,
    height: rect.height,
  }
}
