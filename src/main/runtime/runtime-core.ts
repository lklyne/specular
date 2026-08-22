import {
  WebContentsView,
  type WebContents,
} from 'electron'
import {
  devtoolsOpen as uiDevtoolsOpen,
  devtoolsPanelTab as uiDevtoolsPanelTab,
  devtoolsWidth as uiDevtoolsWidth,
  isCommentOverlayVisible as uiCommentOverlayVisible,
  selectedEntityIds as uiSelectedEntityIds,
  selectedGroupId as uiSelectedGroupId,
  selectedPageIndex as uiSelectedPageIndex,
  setCommentOverlayVisible as setUiCommentOverlayVisible,
  setDevtoolsWidth as setUiDevtoolsWidth,
} from '../ui-state'
import type {
  CanvasHoverTarget,
  DevtoolsPanelData,
} from '../../shared/types'
import { sameHoverTarget } from '../../shared/runtime-patch'
import { broadcastRuntimePatch } from './runtime-patch-broadcast'
import {
  devtoolsView,
  setDevtoolsView,
  win,
} from './view-refs'
import {
  browserDevtoolsAttachGeneration,
  hoverTarget,
  pages,
  setHoverTarget,
  setMcpConnectionStatusState,
  findPageById,
  incrementBrowserDevtoolsAttachGeneration,
} from './runtime-context'
import {
  scheduleSpaceAutosave,
} from './space-autosave'
import {
  recenterFocusPresentation,
  requestLayout,
} from './viewport-control'
import {
  notifyDevtoolsPanelData,
} from './inspect-session'
import type { Page } from './runtime-entities'
import {
  selectEntities as commitSelectedEntities,
  selectEntity as commitSelectEntity,
  selectGroup as commitSelectGroup,
  selectNone as commitSelectNone,
  selectPageById as commitSelectPageById,
} from './selection-controller'
import {
} from './page-factory'
import {
  clampDevtoolsWidth,
  savePreferences,
} from './preferences'
import {
  SELECTION_DEBUG,
} from './runtime-constants'

export { destroyActivePages } from './space-restore'

export {
  createSpaceTab,
  deleteSpaceTab,
  duplicateSpaceTab,
  renameWorkspacePage,
  renameWorkspaceGroup,
  renameSpaceTab,
  setActiveSpaceTab,
  setSpaceTabExpanded,
} from './space-tab-operations'

export {
  restorePersistedSpace,
  restoreWorkspaceSnapshot,
  rebuildWindowFromSnapshot,
} from './space-restore'

export { initWindow } from './window-init'

function setDevtoolsWidth(width: number): void {
  const nextWidth = clampDevtoolsWidth(width)
  if (nextWidth === uiDevtoolsWidth()) return
  setUiDevtoolsWidth(nextWidth)
  savePreferences()
  scheduleSpaceAutosave()
}

export function attachBrowserDevtoolsToPage(index: number): void {
  if (index < 0 || index >= pages.length) return
  const inspectorView = ensureDevtoolsView(pages[index])
  if (!inspectorView) return
  const targetPageId = pages[index].id
  const attachGeneration = incrementBrowserDevtoolsAttachGeneration()

  // Close devtools on other pages (not the target — its session may be reusable)
  for (let i = 0; i < pages.length; i += 1) {
    if (pages[i].id === targetPageId) continue
    try {
      pages[i].pageView.webContents.closeDevTools()
    } catch {
      // Ignore close races while retargeting the shared DevTools view.
    }
  }

  setTimeout(() => {
    if (attachGeneration !== browserDevtoolsAttachGeneration) return
    if (!uiDevtoolsOpen() || uiDevtoolsPanelTab() !== 'browser-devtools') return
    const nextPage = pages.find((page) => page.id === targetPageId)
    if (!nextPage) return
    if (nextPage.pageView.webContents.isDestroyed()) return
    const nextInspectorView = ensureDevtoolsView(nextPage)
    if (!nextInspectorView) return

    if (!nextPage.devtoolsHostAttached) {
      // First time: bind the devtools WebContents (one-time per page)
      nextPage.pageView.webContents.setDevToolsWebContents(nextInspectorView.webContents)
      nextPage.devtoolsHostAttached = true
    }

    // The layout pass hides whichever host is no longer active once
    // setDevtoolsView re-points the alias.
    setDevtoolsView(nextInspectorView)

    // openDevTools is safe to call whether the session is new or was just hidden
    nextPage.pageView.webContents.openDevTools({ mode: 'detach' })
    requestLayout()
  }, 0)
}

/**
 * Reconcile the browser DevTools panel's attached page against the current
 * selection. Idempotent and target-diffed — safe to call on every layout
 * pass, mirroring how `reconcileFocus` derives focus from interaction state
 * instead of relying on imperative callers to keep it in sync.
 */
export function reconcileBrowserDevtools(): void {
  const desiredIndex =
    uiDevtoolsOpen() && uiDevtoolsPanelTab() === 'browser-devtools'
      ? uiSelectedPageIndex(pages.map((p) => p.id))
      : null
  const desiredPageId = desiredIndex !== null ? pages[desiredIndex]?.id ?? null : null

  const currentPage = devtoolsView
    ? pages.find((page) => page.devtoolsHostView === devtoolsView) ?? null
    : null
  const currentPageId = currentPage?.id ?? null

  if (desiredPageId === currentPageId) return

  if (desiredIndex !== null && desiredPageId !== null) {
    attachBrowserDevtoolsToPage(desiredIndex)
    return
  }

  setDevtoolsView(null)
  requestLayout()
}

export function getSelectedEntityIds(): string[] {
  return uiSelectedEntityIds()
}


export function getSelectedGroupId(): string | null {
  return uiSelectedGroupId()
}

export function setSelectedGroupId(value: string | null): void {
  if (value) {
    commitSelectGroup(value)
  } else if (uiSelectedGroupId()) {
    commitSelectNone()
  }
  scheduleSpaceAutosave()
}

function selectedPages(): Page[] {
  return uiSelectedEntityIds()
    .map((pageId) => findPageById(pageId))
    .filter((page): page is Page => page !== undefined)
}
function ensureDevtoolsView(page: Page): WebContentsView | null {
  if (!win) return null
  if (!page.devtoolsHostView) {
    // Construction only — the layout pass child-list reconcile attaches it
    // and `layoutDevtoolsViews()` parks it off-screen until it goes active.
    page.devtoolsHostView = new WebContentsView({
      webPreferences: {
        focusOnNavigation: false,
      },
    })
    page.devtoolsHostView.setBackgroundColor('#242424')
  }
  return page.devtoolsHostView
}

export function setMcpConnectionStatus(
  status: NonNullable<NonNullable<DevtoolsPanelData['emptyState']>['status']>,
): void {
  setMcpConnectionStatusState(status)
  notifyDevtoolsPanelData()
}

export function setSelectedEntities(
  entityIds: string[],
): void {
  commitSelectedEntities(entityIds)
  scheduleSpaceAutosave()
}

function selectionDebug(event: string, details?: Record<string, unknown>): void {
  if (!SELECTION_DEBUG) return
  console.log('[selection-debug:main]', {
    ts: Date.now(),
    event,
    selectedPageIndex: uiSelectedPageIndex(pages.map((p) => p.id)),
    selectedEntityIds: uiSelectedEntityIds(),
    devtoolsOpen: uiDevtoolsOpen(),
    ...details,
  })
}

export function selectPageById(id: string): boolean {
  return commitSelectPageById(id)
}

export function selectEntity(entityId: string, entityKind: string): void {
  commitSelectEntity(entityId, entityKind as import('../../shared/types').CanvasEntityKind)
}

export function setHoveredPage(pageId: string | null): void {
  commitHoverTarget(pageId ? { id: pageId, kind: 'page' } : null)
}

export function setHoverEntity(nextHoverTarget: CanvasHoverTarget): void {
  commitHoverTarget(nextHoverTarget)
}

/**
 * Hover moves with the pointer, so it is the one runtime slice that cannot
 * afford a scene rebuild per change. It rides a patch to the chrome that draws
 * it; `buildCanvasLayoutData` still reads `hoverTarget` for the snapshot, so a
 * pass triggered by anything else carries the current value.
 */
function commitHoverTarget(next: CanvasHoverTarget): void {
  if (sameHoverTarget(hoverTarget, next)) return
  setHoverTarget(next)
  broadcastRuntimePatch({ kind: 'slice', slice: 'hover', value: next })
}

export function setDevtoolsWidthFromScreenX(screenX: number): void {
  if (!win || !uiDevtoolsOpen()) return
  const bounds = win.getContentBounds()
  setDevtoolsWidth(bounds.x + bounds.width - screenX)
  recenterFocusPresentation(undefined, { animate: false })
  requestLayout()
}

function currentDevtoolsOpen(): boolean {
  return uiDevtoolsOpen()
}

function currentDevtoolsWidth(): number {
  return uiDevtoolsWidth()
}

function currentCommentOverlayActive(): boolean {
  return uiCommentOverlayVisible()
}

export function setCommentOverlayActive(active: boolean): void {
  if (uiCommentOverlayVisible() === active) return
  setUiCommentOverlayVisible(active)
  requestLayout()
}

export function endDevtoolsResize(): void {
  savePreferences()
}
