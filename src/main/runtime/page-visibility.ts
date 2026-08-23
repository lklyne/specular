/**
 * Per-page overrides for `View.setVisible` on a page's content view.
 *
 * Visibility is the state Chromium reads to decide whether a page may be
 * background-throttled — collapsing bounds to 0×0 hides a page from the
 * compositor but leaves it fully awake. Only the layout pass may call
 * `setVisible` (invariant I1), so callers that want a page hidden record the
 * intent here and request a layout; `layoutAllViews()` reconciles it.
 *
 * An absent override means "visible", which is the default for every page.
 */

const overrides = new Map<string, boolean>()

export function setPageVisibilityOverride(pageId: string, visible: boolean | null): void {
  if (visible === null) overrides.delete(pageId)
  else overrides.set(pageId, visible)
}

/** null when the page has no override and should follow the default. */
export function pageVisibilityOverride(pageId: string): boolean | null {
  return overrides.get(pageId) ?? null
}

export function clearPageVisibilityOverrides(): void {
  overrides.clear()
}
