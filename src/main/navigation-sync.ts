// fallow-ignore-file circular-dependencies
// Suppressed: see #141. page-factory imports navigation-sync creating a cycle through runtime-core and page-runtime
import { randomUUID } from 'crypto'
import { ipcChannels } from '../shared/ipc-contract'
import type { ScrollSyncData } from '../shared/types'
import {
  type Page,
  pages,
  findPageById,
} from './runtime/page-runtime'
import { mutateWorkspace } from './runtime/mutate-workspace'

const LINKED_SCROLL_SUPPRESSION_MS = 150

export type NavigationSyncAction =
  | { type: 'load-url'; url: string }
  | { type: 'go-back'; fallbackUrl: string }
  | { type: 'go-forward'; fallbackUrl: string }
  | { type: 'reload'; fallbackUrl: string }
  | { type: 'in-page'; url: string }

/**
 * Sync sets are identified by a shared `syncId` on each page — independent of
 * groups. Pages sync navigation/scroll iff they carry the same non-null id.
 */
export function syncPeersOf(source: Page): Page[] {
  if (!source.syncId) return []
  return pages.filter(
    (page) =>
      page !== source &&
      page.syncId === source.syncId &&
      !page.pageView.webContents.isDestroyed(),
  )
}

/** A page is "synced" only when it actually has a live peer in its set. */
export function isPageSynced(page: Page): boolean {
  return syncPeersOf(page).length > 0
}

/**
 * Toggle-merge: if the whole selection already shares one sync set, clear it
 * (unsync); otherwise mint one new set and stamp every selected page. Sets of
 * one are meaningless, so a selection under two pages is a no-op.
 */
export function setSyncForSelection(pageIds: string[]): void {
  const selected = pageIds
    .map((id) => findPageById(id))
    .filter((page): page is Page => page !== undefined)
  if (selected.length < 2) return
  mutateWorkspace(() => {
    const first = selected[0].syncId
    const allShareOne =
      first != null && selected.every((page) => page.syncId === first)
    if (allShareOne) {
      for (const page of selected) page.syncId = null
    } else {
      const id = `sync_${randomUUID()}`
      for (const page of selected) page.syncId = id
    }
    dissolveOrphanSyncSets()
  })
}

/** Remove a single page from its sync set; the set auto-dissolves below two members. */
export function unsyncPage(pageId: string): void {
  const page = findPageById(pageId)
  if (!page || !page.syncId) return
  mutateWorkspace(() => {
    page.syncId = null
    dissolveOrphanSyncSets()
  })
}

/** Clear any sync id left with fewer than two members after a membership change. */
function dissolveOrphanSyncSets(): void {
  const counts = new Map<string, number>()
  for (const page of pages) {
    if (page.syncId) counts.set(page.syncId, (counts.get(page.syncId) ?? 0) + 1)
  }
  for (const page of pages) {
    if (page.syncId && (counts.get(page.syncId) ?? 0) < 2) page.syncId = null
  }
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
      // Avoid navigation ping-pong between sync peers when URL is already identical.
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
  for (const peer of syncPeersOf(source)) {
    markNavigationSuppressed(peer)
    applyNavigationAction(peer, action)
  }
}

/**
 * Navigate a page (source) and propagate the action to sync peers.
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

export function propagateScrollFromPage(
  source: Page,
  scrollData: ScrollSyncData,
): void {
  for (const peer of syncPeersOf(source)) {
    markScrollSuppressed(peer)
    peer.pageView.webContents.send(ipcChannels.applyLinkedScroll, scrollData)
  }
}
