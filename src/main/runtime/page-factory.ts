/**
 * Page factory — creation and removal of browser page views.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import { WebContentsView } from 'electron'
import { registerPageIdleThrottle } from './page-idle-throttle'
import { randomUUID } from 'crypto'
import { preloadPath } from './load-renderer'
import type { PageConfig } from '../../shared/types'
import { toolAnnotateOverlay } from '../../shared/tool'
import {
  toolbarView,
  win,
} from './view-refs'
import {
  inspectSelectedNodeIdByPage,
  interactivePageId,
  pages,
  setInteractivePageId,
  setPendingFocus,
} from './runtime-context'
import {
  activeTool as uiActiveTool,
  selectedEntityIds as uiSelectedEntityIds,
  selectedPageIndex as uiSelectedPageIndex,
  setSelection as setUiSelection,
  updateSelectionForRemovedEntity,
} from '../ui-state'
import { normalizePresetIndex } from './runtime-serialization'
import type { Page } from './runtime-entities'
import { pageOverridesFromMetadata } from './runtime-entities'
import { markDirty } from './layout-dirty'
import {
  broadcastPageChrome,
  refreshPageChrome,
  refreshPageNavigationState,
} from './page-chrome-state'
import { clearPageAnchorsForPage } from './page-anchor-state'
import { resetAttachmentSubscriptionsForPage } from './element-attachment-subscriptions'
import { requestLayout } from './viewport-control'
import { applyNavigationEmulation } from './layout-engine'
import { endFocusSession, focusedPageId } from './focus-session'
import {
  clearInspectTargets,
  notifyDevtoolsPanelData,
  syncInspectionState,
} from './inspect-session'
import { clearPendingRequestsForPage } from './page-ipc'
import { sendInteractiveState } from './overlay-manager'
import { broadcastCanvasZoomToPages } from './viewport-control'
import { invalidateAgentSnapshot } from './agent-snapshot-cache'
import {
  isNavigationSuppressed,
  markNavigationSuppressed,
  propagateNavigationFromPage,
} from '../navigation-sync'
import { invalidateInteractionSyncResolution } from '../interaction-sync'
import { attachBindingDispatcher } from './binding-dispatcher'
import { openLinkInNewFrame } from './link-open-policy'
import { looksLikeUrl } from '../../shared/url'
import { breadcrumb } from '../sentry-context'
import { installScrollbarCss } from './page-scrollbar-css'

function hostOf(url: string | undefined): string | undefined {
  if (!url) return undefined
  try {
    return new URL(url).host
  } catch {
    return undefined
  }
}

function isSelectedPage(page: Page): boolean {
  const idx = uiSelectedPageIndex(pages.map((p) => p.id))
  return idx !== null && idx >= 0 && idx < pages.length && pages[idx] === page
}

import {
  CARD_BORDER_RADIUS,
  selectionDebug,
} from './runtime-constants'

function makePageId(): string {
  return `page_${randomUUID()}`
}

export function createPage(config: PageConfig): Page {
  if (!win || !toolbarView) throw new Error('Window not initialized')
  breadcrumb('page', 'create', { host: hostOf(config.url), preset: config.presetIndex })
  const presetIndex = normalizePresetIndex(config.presetIndex)

  // Construction only — the layout pass child-list reconcile (layer-stack)
  // owns attachment. createPage just pushes to pages[] and requests layout.
  const pageView = new WebContentsView({
    webPreferences: {
      preload: preloadPath('page-content'),
      focusOnNavigation: false,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })
  pageView.setBorderRadius(CARD_BORDER_RADIUS)

  const page: Page = {
    id: config.id ?? makePageId(),
    name: config.name?.trim() || undefined,
    title: config.name?.trim() || undefined,
    url: config.url,
    faviconUrl: null,
    pageView,
    devtoolsHostAttached: false,
    presetIndex,
    canvasX: config.canvasX,
    canvasY: config.canvasY,
    syncId: config.syncId ?? null,
    source: config.source ?? 'manual',
    parentGroupId: config.parentGroupId ?? config.groupId,
    groupId: config.parentGroupId ?? config.groupId,
    metadata: config.metadata,
    colorScheme: config.colorScheme,
    syncState: {
      suppressNavigationBroadcastUntil: 0,
      suppressNextScrollBroadcastUntil: 0,
    },
    scrollX: 0,
    scrollY: 0,
    scrollHeight: 0,
    navGeneration: 0,
  }
  pages.push(page)
  markDirty('canvas', 'sidebar', 'toolbar')

  registerPageIdleThrottle(page)
  installScrollbarCss(page.pageView.webContents)

  page.pageView.webContents.on('page-title-updated', () => {
    page.title = page.pageView.webContents.getTitle() || undefined
    broadcastPageChrome(page)
    if (isSelectedPage(page)) notifyDevtoolsPanelData()
  })
  page.pageView.webContents.on('page-favicon-updated', (_event, favicons) => {
    page.faviconUrl = favicons[0] ?? null
    broadcastPageChrome(page)
  })
  page.pageView.webContents.on('did-start-loading', () => {
    selectionDebug('page:did-start-loading', { pageId: page.id, url: page.pageView.webContents.getURL() })
    page.isLoading = true
    page.crashedAt = undefined
    page.crashReason = undefined
    // Document-bound items keep the visibility they had while a route is in
    // flight (`offPageDocument`), so a load starting moves chrome and no
    // membership — the scene stays as it is.
    refreshPageChrome(page)
  })
  page.pageView.webContents.on('render-process-gone', (_event, details) => {
    page.crashedAt = Date.now()
    page.crashReason = details.reason
    breadcrumb('page', 'render-process-gone', {
      host: hostOf(page.url),
      reason: details.reason,
      exitCode: details.exitCode,
    })
    selectionDebug('page:render-process-gone', { pageId: page.id, ...details })
  })
  page.pageView.webContents.on('unresponsive', () => {
    breadcrumb('page', 'unresponsive', { host: hostOf(page.url) })
    selectionDebug('page:unresponsive', { pageId: page.id })
  })
  page.pageView.webContents.on('did-stop-loading', () => {
    selectionDebug('page:did-stop-loading', { pageId: page.id, url: page.pageView.webContents.getURL() })
    page.isLoading = false
    refreshPageNavigationState(page)
    // A settled load re-opens the document-binding gate, which adds or removes
    // page-anchored entities — a membership change, so the pass runs.
    markDirty('canvas', 'sidebar')
    requestLayout()
  })
  page.pageView.webContents.on('did-fail-load', (_event, errorCode, errorDescription, validatedURL) => {
    selectionDebug('page:did-fail-load', { pageId: page.id, errorCode, errorDescription, validatedURL })
  })
  page.pageView.webContents.on('did-finish-load', () => {
    selectionDebug('page:did-finish-load', { pageId: page.id, url: page.pageView.webContents.getURL() })
    page.title = page.pageView.webContents.getTitle() || undefined
    page.url = page.pageView.webContents.getURL() || page.url
    // Fallback favicon extraction: if page-favicon-updated didn't fire,
    // query the DOM for <link rel="icon"> and fall back to /favicon.ico
    if (!page.faviconUrl) {
      const faviconTimeout = setTimeout(() => {
        page.pageView.webContents.ipc.removeAllListeners(ipcChannels.queryFaviconResult)
      }, 5000)
      page.pageView.webContents.ipc.once(
        ipcChannels.queryFaviconResult,
        (_event: Electron.IpcMainEvent, href: string | null) => {
          clearTimeout(faviconTimeout)
          if (page.faviconUrl) return
          let resolvedHref = href
          if (!resolvedHref) {
            try {
              resolvedHref =
                new URL(page.pageView.webContents.getURL()).origin +
                '/favicon.ico'
            } catch {
              return
            }
          }
          page.faviconUrl = resolvedHref
          broadcastPageChrome(page)
        },
      )
      page.pageView.webContents.send(ipcChannels.queryFavicon)
    }
    invalidateAgentSnapshot(page.id)
    // A finished load starts the page preload with no subscriptions; re-declare
    // the selectors this page's anchored items track (ADR 0032).
    resetAttachmentSubscriptionsForPage(page.id)
    page.lastSafeAreaCssKey = undefined
    page.lastSafeAreaCssId = undefined
    if (isSelectedPage(page)) clearInspectTargets()
    if (isSelectedPage(page)) notifyDevtoolsPanelData()
    syncInspectionState()
    page.pageView.webContents.send(ipcChannels.setAnnotateMode, toolAnnotateOverlay(uiActiveTool()))
    sendInteractiveState()
    broadcastCanvasZoomToPages()
    const overrides = pageOverridesFromMetadata(page.metadata)
    if (overrides) {
      page.pageView.webContents.send(ipcChannels.applyPageOverrides, overrides)
    }
  })
  // Per-page generation counter for D8 (issue #318): a full navigation
  // typically fires both dom-ready and did-navigate, but the staleness
  // comparison is `>` rather than `+1`, so double-counting is harmless.
  page.pageView.webContents.on('dom-ready', () => {
    page.navGeneration += 1
  })
  page.pageView.webContents.on('did-navigate', (_event, url) => {
    selectionDebug('page:did-navigate', { pageId: page.id, url })
    breadcrumb('navigation', 'did-navigate', { host: hostOf(url) })
    page.url = url
    // Commit is the earliest point the frame is guaranteed live (Electron
    // derefs the frame's widget view unguarded) and precedes the new
    // document's first layout, so it lays out at the emulated viewport and
    // scale instead of reflowing once the next layout pass catches up. A
    // cross-process navigation also swaps in a fresh widget that needs it.
    applyNavigationEmulation(page)
    // The new document starts unscrolled; keeping the old document's offset
    // would shift every page-anchored region until the first scroll event.
    page.scrollX = 0
    page.scrollY = 0
    page.scrollHeight = 0
    // The new document dropped the old preload's subscriptions and its element
    // positions no longer apply — re-declare and clear (ADR 0032).
    page.elementPositions = undefined
    resetAttachmentSubscriptionsForPage(page.id)
    // Annotation visibility and the sidebar's page children key off the
    // page's current URL, so a navigation must re-send both payloads.
    refreshPageNavigationState(page)
    markDirty('canvas', 'sidebar')
    page.navGeneration += 1
    requestLayout()
    invalidateAgentSnapshot(page.id)
    // The page's cached element rects (interaction-sync resolution cache) and
    // origin are stale after a full navigation — a fresh sync point (ADR 0030).
    invalidateInteractionSyncResolution(page.id)
    if (isSelectedPage(page)) clearInspectTargets()
    if (isSelectedPage(page)) notifyDevtoolsPanelData()
    if (isNavigationSuppressed(page)) return
    if (!page.syncId) return
    propagateNavigationFromPage(page, { type: 'load-url', url })
  })
  page.pageView.webContents.on('did-navigate-in-page', (_event, url, isMainFrame) => {
    selectionDebug('page:did-navigate-in-page', { pageId: page.id, url, isMainFrame })
    if (isMainFrame) page.url = url
    if (isMainFrame) refreshPageNavigationState(page)
    if (isMainFrame) markDirty('canvas', 'sidebar')
    if (isMainFrame) requestLayout()
    if (isMainFrame) invalidateAgentSnapshot(page.id)
    if (isSelectedPage(page)) clearInspectTargets()
    if (isSelectedPage(page)) notifyDevtoolsPanelData()
    if (!isMainFrame) return
    if (isNavigationSuppressed(page)) return
    if (!page.syncId) return
    propagateNavigationFromPage(page, { type: 'in-page', url })
  })

  // A link that would open a new tab (target="_blank", window.open without
  // window features, or cmd/ctrl-click) shouldn't spawn a native popup.
  // Instead, reuse the duplicate flow: drop a new frame on the canvas that
  // inherits this page's preset and size but points at the requested URL.
  // Genuine popups (window.open with features — OAuth, pickers) keep their
  // native window so those flows still work.
  page.pageView.webContents.setWindowOpenHandler(({ url, disposition }) => {
    const opensNewTab =
      disposition === 'foreground-tab' || disposition === 'background-tab'
    if (opensNewTab && looksLikeUrl(url)) {
      // Never let a throw escape into Electron's native callback — always
      // resolve the disposition by returning deny.
      try {
        openLinkInNewFrame({
          sourcePageId: page.id,
          url,
          // Background opens (cmd/middle-click) shouldn't yank selection.
          focus: disposition === 'foreground-tab',
        })
      } catch {
        breadcrumb('navigation', 'open-link-as-frame-failed', {
          host: hostOf(url),
        })
      }
      return { action: 'deny' }
    }
    return { action: 'allow' }
  })

  if (config.suppressInitialNavigationBroadcast) {
    markNavigationSuppressed(page)
  }
  page.pageView.webContents.loadURL(config.url).catch(() => {})

  // Selection — not page-side webContents focus — drives the keyboard
  // target. The focus reconciler runs the predicate every layout pass and
  // routes focus accordingly; nothing on this listener side is needed.

  // Optional spike: webContents focus/blur reliability. Enable with
  // `BLUR_SPIKE=1 pnpm dev` to log every focus/blur and devtools-open/close
  // on the page's webContents during diagnosis.
  if (process.env.BLUR_SPIKE === '1') {
    const tag = '[blur-spike]'
    const log = (event: string, extra?: Record<string, unknown>) => {
      console.log(tag, event, { pageId: page.id, host: hostOf(page.url), ...extra })
    }
    page.pageView.webContents.on('focus', () => log('page:focus'))
    page.pageView.webContents.on('blur', () => log('page:blur'))
    page.pageView.webContents.on('devtools-opened', () => log('page:devtools-opened'))
    page.pageView.webContents.on('devtools-closed', () => log('page:devtools-closed'))
    page.pageView.webContents.on('devtools-focused', () => log('page:devtools-focused'))
  }

  attachBindingDispatcher(pageView.webContents, 'page')

  requestLayout()

  return page
}

export function removePageAtIndex(idx: number): Page | null {
  if (!win || idx < 0 || idx >= pages.length) return null
  const page = pages[idx]
  // End a focus session aimed at the page we're deleting — otherwise it
  // survives as a stale session that freezes the canvas (zoom/pan IPC
  // early-returns while focus is active) with no visible affordance to recover.
  // Full select-first / interact-second delete behavior is tracked in #124.
  if (focusedPageId() === page.id) endFocusSession('dismiss')
  if (interactivePageId() === page.id) setInteractivePageId(null)
  breadcrumb('page', 'remove', { host: hostOf(page.url) })
  clearPendingRequestsForPage(page.id)
  // Detachment is owned by the layout pass child-list reconcile — splice
  // pages[], close the webContents, and request layout below.
  page.pageView.webContents.close()
  page.devtoolsHostView?.webContents.close()
  // Transfer focus to aboveView so keyboard shortcuts (including undo) keep
  // working after the deleted page's webContents is destroyed. The actual
  // focus() call lands at the end of the next layout pass via reconcileFocus.
  setPendingFocus({ kind: 'aboveView' })
  pages.splice(idx, 1)
  clearPageAnchorsForPage(page.id)
  markDirty('canvas', 'sidebar', 'toolbar')
  invalidateAgentSnapshot(page.id)
  const previousSelectedIndex = uiSelectedPageIndex(pages.map((p) => p.id))
  updateSelectionForRemovedEntity(page.id)
  inspectSelectedNodeIdByPage.delete(page.id)

  if (previousSelectedIndex === idx) {
    clearInspectTargets()
  }

  if (!uiSelectedEntityIds().length) {
    setUiSelection({ kind: 'none' })
  }

  sendInteractiveState()
  syncInspectionState()
  requestLayout()
  return page
}

export function removePageById(id: string): Page | null {
  const idx = pages.findIndex((page) => page.id === id)
  if (idx === -1) return null
  return removePageAtIndex(idx)
}
