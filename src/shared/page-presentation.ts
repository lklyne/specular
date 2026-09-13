/**
 * Whether a page is shown on the canvas at all, given the focus session.
 *
 * Binary show/hide, never dimmed (ADR 0021). The eye brings other pages back
 * as surrounding context, except in 'fill' mode where the focused page covers
 * the viewport so context can never return. A file-target session is always
 * 'fill' and has no focused page id, so every page is context and the eye
 * governs all of them (ADR 0021 Amendment 2).
 */

import type { FocusContext } from './focus-context'

/** The focus facts presentation reads: `focusContext` in a renderer, the live session in main. */
export type PagePresentationFocus = Pick<FocusContext, 'active' | 'pageId' | 'mode' | 'showsContext'>

export function isPagePresented(pageId: string, focus: PagePresentationFocus): boolean {
  if (!focus.active) return true
  if (focus.pageId === pageId) return true
  return (focus.pageId === null || focus.mode !== 'fill') && focus.showsContext
}
