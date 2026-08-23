/**
 * A page's browser chrome — title, favicon, URL, load state, back/forward
 * availability — mirrored out of `webContents` and onto the scene.
 *
 * These fields change on navigation, not on geometry, and the events that move
 * them already fire in main. Reading them inside the layout pass instead meant
 * every pass paid a `navigationHistory` walk and an `isLoading()` call per
 * page — a per-frame cost during a drag to observe something that changes a
 * few times per page load.
 *
 * So the lifecycle hooks write the runtime mirror and push one entity patch for
 * the page they moved. `buildPageSceneEntity` is the same builder the geometry
 * pass uses, so the patch and the snapshot behind it cannot disagree, and the
 * bus drops the patch outright when nothing moved (`broadcastRuntimePatches`).
 */

import type { Page } from './runtime-entities'
import { buildPageSceneEntity, pageInScene } from './page-scene-entity'
import { broadcastRuntimePatch } from './runtime-patch-broadcast'

/** Re-read the navigation history into the runtime mirror. The other chrome
 *  fields arrive on their own events; back/forward availability has none, so it
 *  is sampled whenever a navigation settles. */
export function refreshPageNavigationState(page: Page): void {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return
  page.canGoBack = wc.navigationHistory.canGoBack()
  page.canGoForward = wc.navigationHistory.canGoForward()
}

/** Ship the page's current chrome. Safe to call from any lifecycle hook: a
 *  call that changes nothing sends nothing. */
export function broadcastPageChrome(page: Page): void {
  if (!pageInScene(page.id)) return
  broadcastRuntimePatch({
    kind: 'entity',
    id: page.id,
    entity: buildPageSceneEntity(page),
  })
}

export function refreshPageChrome(page: Page): void {
  refreshPageNavigationState(page)
  broadcastPageChrome(page)
}
