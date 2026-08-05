// Renderer-side derived view of the focus session (ADR 0021). Pure selector
// over the broadcast LayoutUpdateData so the above-view and canvas-bg bundles
// share one definition instead of each re-deriving "are we focused" inline.

import type {
  FocusPresentationData,
  FocusPresentationMode,
  FocusTarget,
  LayoutUpdateData,
} from './types'

export interface FocusContext {
  /** A focus session is active. */
  active: boolean
  /** The focused page id — null when no session is active *or* the session
   *  frames a file entity. Page-specific consumers key off this. */
  pageId: string | null
  /** The focused file entity id — null when no session is active *or* the
   *  session frames a page. */
  fileId: string | null
  /** What the session frames; null when no session is active. */
  target: FocusTarget | null
  mode: FocusPresentationMode | null
  data: FocusPresentationData | null
  /**
   * Surrounding context — other pages (and their chrome), annotations
   * (stickies/text/shapes/drawings/edges), and group backgrounds — renders.
   * Always true outside a session; inside one it's the session's latched eye
   * state (ADR 0021): off for a clean read of the focused page alone, on once a
   * working tool or the focus-bar eye turns it on. Binary show/hide, never a
   * dim — the focused page is the only thing left when it's off.
   */
  showsContext: boolean
}

export function focusContext(layout: LayoutUpdateData): FocusContext {
  const data = layout.focusPresentation
  const active = data !== null
  const target = data?.target ?? null
  return {
    active,
    pageId: target?.kind === 'page' ? target.id : null,
    fileId: target?.kind === 'file' ? target.id : null,
    target,
    mode: data?.mode ?? null,
    data,
    showsContext: !active || data.annotationsVisible,
  }
}
