// Renderer-side derived view of the focus session (ADR 0021). Pure selector
// over the broadcast LayoutUpdateData so the above-view and canvas-bg bundles
// share one definition instead of each re-deriving "are we focused" inline.

import type { FocusPresentationData, FocusPresentationMode, LayoutUpdateData } from './types'
import { isWorkingTool } from './tool'

export const FOCUS_DIMMED_ITEM_OPACITY = 0.2

export interface FocusContext {
  /** A focus session is active. */
  active: boolean
  /** The focused page id, or null when no session is active. */
  pageId: string | null
  mode: FocusPresentationMode | null
  data: FocusPresentationData | null
  /**
   * Other pages should be dimmed. The dim is a resting affordance — a working
   * tool (draw/placement/comment) lifts it so you can annotate freely.
   */
  dimsOtherPages: boolean
}

export function focusContext(layout: LayoutUpdateData): FocusContext {
  const data = layout.focusPresentation
  const active = data !== null
  return {
    active,
    pageId: data?.pageId ?? null,
    mode: data?.mode ?? null,
    data,
    dimsOtherPages: active && !isWorkingTool(layout.activeTool),
  }
}

/**
 * Per-entity opacity under the focus dim: the focused page is full, others fade.
 * Pass `null` to disable dimming entirely (no session, or a working tool has
 * lifted the dim — see `FocusContext.dimsOtherPages`).
 */
export function focusItemOpacity(dimmedAroundPageId: string | null, entityId: string): number {
  if (!dimmedAroundPageId) return 1
  return entityId === dimmedAroundPageId ? 1 : FOCUS_DIMMED_ITEM_OPACITY
}
