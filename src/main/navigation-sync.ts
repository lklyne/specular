// fallow-ignore-file circular-dependencies
// Suppressed: see #141. page-factory imports navigation-sync creating a cycle through runtime-core and page-runtime
import { ipcChannels } from '../shared/ipc-contract'
import type { ScrollSyncData } from '../shared/types'
import {
  type Page,
  pages,
  findPageById,
} from './runtime/page-runtime'
import { getSelectedEntityIds } from './runtime/ui-actions'
import { scheduleWorkspaceAutosave } from './runtime/workspace-autosave'

const LINKED_SCROLL_SUPPRESSION_MS = 150

export type NavigationSyncAction =
  | { type: 'load-url'; url: string }
  | { type: 'go-back'; fallbackUrl: string }
  | { type: 'go-forward'; fallbackUrl: string }
  | { type: 'reload'; fallbackUrl: string }
  | { type: 'in-page'; url: string }

export function linkedPeersOf(source: Page): Page[] {
  if (!source.linked) return []
  return pages.filter(
    (page) =>
      page !== source &&
      page.linked &&
      (source.groupId ? page.groupId === source.groupId : !page.groupId) &&
      !page.pageView.webContents.isDestroyed(),
  )
}

export function togglePageLinked(page: Page): void {
  page.linked = !page.linked
  scheduleWorkspaceAutosave()
}

const LINKED_NAV_SUPPRESSION_MS = 1500

export function markNavigationSuppressed(page: Page): void {
  page.syncState.suppressNavigationBroadcastUntil = Math.max(
    page.syncState.suppressNavigationBroadcastUntil ?? 0,
    Date.now() + LINKED_NAV_SUPPRESSION_MS,
  )
}

export function isNavigationSuppressed(page: Page): boolean {
  return (page.syncState.suppressNavigationBroadcastUntil ?? 0) > Date.now()
}

export function markScrollSuppressed(
  page: Page,
  durationMs = LINKED_SCROLL_SUPPRESSION_MS,
): void {
  page.syncState.suppressNextScrollBroadcastUntil = Math.max(
    page.syncState.suppressNextScrollBroadcastUntil,
    Date.now() + durationMs,
  )
}

export function isScrollSuppressed(page: Page): boolean {
  return page.syncState.suppressNextScrollBroadcastUntil > Date.now()
}

function applyNavigationAction(page: Page, action: NavigationSyncAction): void {
  const webContents = page.pageView.webContents
  if (webContents.isDestroyed()) return
  const currentUrl = webContents.getURL()

  switch (action.type) {
    case 'load-url':
      if (currentUrl === action.url) return
      webContents.loadURL(action.url)
      return
    case 'in-page':
      // Avoid navigation ping-pong between linked peers when URL is already identical.
      if (currentUrl === action.url) return
      webContents.loadURL(action.url)
      return
    case 'go-back':
      if (webContents.canGoBack()) webContents.goBack()
      else webContents.loadURL(action.fallbackUrl)
      return
    case 'go-forward':
      if (webContents.canGoForward()) webContents.goForward()
      else webContents.loadURL(action.fallbackUrl)
      return
    case 'reload':
      if (webContents.getURL() === action.fallbackUrl) webContents.reload()
      else webContents.loadURL(action.fallbackUrl)
      return
  }
}

export function propagateNavigationFromPage(
  source: Page,
  action: NavigationSyncAction,
): void {
  for (const peer of linkedPeersOf(source)) {
    markNavigationSuppressed(peer)
    applyNavigationAction(peer, action)
  }
}

/**
 * Navigate a page (source) and propagate the action to linked peers.
 * This is the single entry point for all page navigation triggered by
 * user interactions (canvas chrome, right panel, context menu).
 */
export function navigatePage(
  page: Page,
  action: NavigationSyncAction,
): void {
  markNavigationSuppressed(page)
  applyNavigationAction(page, action)
  propagateNavigationFromPage(page, action)
}

export function applyNavigationToSelectedPages(
  action: NavigationSyncAction,
): void {
  const targets = new Map<string, Page>()
  for (const pageId of getSelectedEntityIds()) {
    const page = findPageById(pageId)
    if (!page) continue
    targets.set(page.id, page)
    for (const peer of linkedPeersOf(page)) {
      targets.set(peer.id, peer)
    }
  }

  for (const page of targets.values()) {
    markNavigationSuppressed(page)
    if (
      action.type === 'go-back' ||
      action.type === 'go-forward' ||
      action.type === 'reload'
    ) {
      applyNavigationAction(page, {
        ...action,
        fallbackUrl: page.pageView.webContents.getURL(),
      })
      continue
    }

    applyNavigationAction(page, action)
  }
}

export function propagateScrollFromPage(
  source: Page,
  scrollData: ScrollSyncData,
): void {
  for (const peer of linkedPeersOf(source)) {
    markScrollSuppressed(peer)
    peer.pageView.webContents.send(ipcChannels.applyLinkedScroll, scrollData)
  }
}
