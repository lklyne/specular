/**
 * The page scene entity, and the set of pages the scene contains.
 *
 * One builder, two callers: the geometry pass (`buildCanvasLayoutData`) maps it
 * over every visible page, and `page-chrome-state.ts` calls it for a single
 * page when a navigation or a load moves that page's chrome. Both produce the
 * same record from the same runtime fields, which is what keeps a chrome patch
 * and the snapshot that follows it from being two descriptions of one page.
 *
 * Nothing here reads the page's `webContents`. Everything the renderer draws —
 * title, favicon, URL, load state, back/forward availability — is mirrored onto
 * the `Page` record by the lifecycle hooks that already know when it changes,
 * so a pass costs a field copy per page rather than a round trip into a
 * renderer process per page.
 */

import type { CanvasScenePageEntity } from '../../shared/types'
import { isPageSynced } from '../navigation-sync'
import { focusedPageId } from './focus-session'
import { pages } from './runtime-context'
import type { Page } from './runtime-entities'
import {
  deviceIdFromMetadata,
  deviceOrientationFromMetadata,
  showDeviceFrameFromMetadata,
  useSvgDeviceShellFromMetadata,
} from './runtime-entities'
import { boundEffectivePageContentSize as effectivePageContentSize } from './runtime-geometry'
import { pageDisplayLabel } from './runtime-serialization'
import { inferRepoPathForOrigin } from './dev-server-manager'

export function buildPageSceneEntity(page: Page): CanvasScenePageEntity {
  const { width, height } = effectivePageContentSize(page)
  let boundRepoPath: string | null = null
  try {
    boundRepoPath = inferRepoPathForOrigin(new URL(page.url).origin)
  } catch {
    boundRepoPath = null
  }
  return {
    kind: 'page',
    id: page.id,
    label: pageDisplayLabel(page),
    faviconUrl: page.faviconUrl ?? null,
    url: page.url,
    canGoBack: page.canGoBack ?? false,
    canGoForward: page.canGoForward ?? false,
    isLoading: page.isLoading ?? false,
    canvasX: page.canvasX,
    canvasY: page.canvasY,
    width,
    height,
    presetIndex: page.presetIndex,
    synced: isPageSynced(page),
    syncId: page.syncId ?? null,
    // Device state
    deviceId: deviceIdFromMetadata(page.metadata),
    deviceOrientation: deviceOrientationFromMetadata(page.metadata),
    showDeviceFrame: showDeviceFrameFromMetadata(page.metadata),
    useSvgDeviceShell: useSvgDeviceShellFromMetadata(page.metadata),
    colorScheme: page.colorScheme,
    scrollX: page.scrollX ?? 0,
    scrollY: page.scrollY ?? 0,
    ...(page.elementPositions?.size
      ? { elementPositions: Object.fromEntries(page.elementPositions) }
      : {}),
    ...(boundRepoPath ? { boundRepoPath } : {}),
  }
}

/** A focus session frames one page; every other page leaves the scene while it
 *  is open, so a patch for one of them would add an entity the next pass
 *  removes. */
export function pageInScene(pageId: string): boolean {
  const focused = focusedPageId()
  return focused ? pageId === focused : true
}

export function backgroundPageOverlays(): CanvasScenePageEntity[] {
  const focused = focusedPageId()
  const visiblePages = focused ? pages.filter((page) => page.id === focused) : pages
  return visiblePages.map(buildPageSceneEntity)
}
