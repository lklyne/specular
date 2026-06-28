// Renderer-side derived view of the focus session (ADR 0021). Pure selector
// over the broadcast LayoutUpdateData so the above-view and canvas-bg bundles
// share one definition instead of each re-deriving "are we focused" inline.

import type {
  FocusPresentationData,
  FocusPresentationMode,
  LayoutUpdateData,
} from './types'

export const FOCUS_DIMMED_ITEM_OPACITY = 0.2

export interface FocusContext {
  /** A focus session is active. */
  active: boolean
  /** The focused page id, or null when no session is active. */
  pageId: string | null
  mode: FocusPresentationMode | null
  data: FocusPresentationData | null
  /** Other pages recede behind a scrim while a session is active. */
  dimsOtherPages: boolean
  /**
   * Annotations (stickies/text/shapes/drawings/edges) render. Always true
   * outside a session; inside one it's the session's latched eye state
   * (ADR 0021) — off for a clean read, on once a working tool or the focus-bar
   * eye turns it on.
   */
  showsAnnotations: boolean
}

export function focusContext(layout: LayoutUpdateData): FocusContext {
  const data = layout.focusPresentation
  const active = data !== null
  return {
    active,
    pageId: data?.pageId ?? null,
    mode: data?.mode ?? null,
    data,
    dimsOtherPages: active,
    showsAnnotations: !active || data.annotationsVisible,
  }
}
