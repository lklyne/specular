/**
 * The `presence` slice: agent cursors, resolved to canvas space.
 *
 * A cursor on a page is stored in that page's document coordinates, so main
 * projects it through the page's frame before broadcasting — which is the only
 * reason presence ever needed the layout pass. It is not scene geometry: a
 * cursor moving changes no entity, so `presence-cursor.ts` patches this slice
 * and the pass reads the same function for its snapshot.
 */

import { selectAmbientMode } from '../../shared/presence-ambient'
import { resolvePresencePagePoint } from '../../shared/presence-targeting'
import type { RuntimeStoreSlices } from '../../shared/runtime-store'
import type { AgentPresenceCursor } from '../../shared/types'
import { getPresenceCursors } from '../presence-cursor'
import { pageInScene } from './page-scene-entity'
import { pages } from './runtime-context'
import {
  projectFramePointToCanvas,
  boundEffectivePageContentSize as effectivePageContentSize,
} from './runtime-geometry'

function presencePoint(cursor: ReturnType<typeof getPresenceCursors>[number]): {
  canvasX: number
  canvasY: number
} {
  if (cursor.surface !== 'page' || !cursor.pageId) {
    return { canvasX: cursor.canvasX, canvasY: cursor.canvasY }
  }
  const page = pages.find((candidate) => candidate.id === cursor.pageId)
  // A page a focus session has taken out of the scene has no frame to project
  // through; the cursor falls back to the canvas point it was reported at.
  if (!page || !pageInScene(page.id)) {
    return { canvasX: cursor.canvasX, canvasY: cursor.canvasY }
  }
  const { width, height } = effectivePageContentSize(page)
  const point = resolvePresencePagePoint({
    pageX: cursor.pageX,
    pageY: cursor.pageY,
    targetRect: cursor.targetRect ?? null,
    fallbackX: width / 2,
    fallbackY: height / 2,
  })
  // Clamp to the page's visible area so the cursor doesn't render outside the
  // page when targeting off-screen elements.
  const projected = projectFramePointToCanvas(page, {
    x: Math.max(0, Math.min(point.x, width)),
    y: Math.max(0, Math.min(point.y, height)),
  })
  return { canvasX: projected.x, canvasY: projected.y }
}

export function currentPresenceSlice(): RuntimeStoreSlices['presence'] {
  return getPresenceCursors().map((c): AgentPresenceCursor => ({
    ...presencePoint(c),
    sessionId: c.sessionId,
    clientName: c.clientName,
    color: c.color,
    source: c.source,
    surface: c.surface,
    activity: c.activity,
    pageId: c.pageId,
    pageX: c.pageX,
    pageY: c.pageY,
    labelKey: c.labelKey,
    taskLabel: c.taskLabel,
    labelHint: c.labelHint,
    labelParams: c.labelParams,
    targetRef: c.targetRef,
    targetRefSource: c.targetRefSource,
    targetName: c.targetName,
    targetRect: c.targetRect,
    updatedAt: c.updatedAt,
    dwellBudgetMs: c.dwellBudgetMs,
    // Synced cursors mirror a real user cursor and must never wander —
    // idle-drift/reading-scan are agent-thinking-gap semantics (ADR 0029)
    // that don't apply here, so force 'none' regardless of activity.
    ambientMode:
      c.source === 'interaction-sync'
        ? 'none'
        : selectAmbientMode(c.activity, c.lastIntentLabelKey),
  }))
}
