/**
 * The single constructor for every WebContentsView in the app.
 *
 * Enforces invariant I6 (spec §6): `setBackgroundColor` is called BEFORE any
 * `addChildView` so WCVs don't flash opaque white during creation. Navigation
 * focus is disabled by default so loading a view cannot activate the app or
 * take keyboard focus from another view.
 *
 * Default is '#00000000' (transparent) for overlay surfaces. Callers that
 * intentionally want an opaque background (e.g. page pages, devtools
 * background) pass `backgroundColor` explicitly.
 *
 * Today only the factory is documented; call sites will migrate over in
 * Phase 7. New WCVs MUST go through this factory.
 */

import { WebContentsView, type WebContentsViewConstructorOptions } from 'electron'

export type CreateViewOptions = WebContentsViewConstructorOptions & {
  /** Override the transparent default. Use hex like '#18181b'. */
  backgroundColor?: string
}

export function createView(options: CreateViewOptions = {}): WebContentsView {
  const { backgroundColor = '#00000000', webPreferences, ...ctorOptions } = options
  const view = new WebContentsView({
    ...ctorOptions,
    webPreferences: {
      focusOnNavigation: false,
      ...webPreferences,
    },
  })
  view.setBackgroundColor(backgroundColor)
  return view
}
