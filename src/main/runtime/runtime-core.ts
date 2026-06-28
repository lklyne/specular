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
  DevtoolsPanelData,
} from '../../shared/types'
import {
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
  scheduleWorkspaceAutosave,
} from './workspace-autosave'
import {
  recenterFocusPresentation,
  requestLayout,
} from './viewport-control'
import { markDirty } from './layout-dirty'
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
  removePageAtIndex,
} from './page-factory'
import {
  clampDevtoolsWidth,
  savePreferences,
} from './preferences'
import {
  COMMENT_BADGE_DEBUG,
  SELECTION_DEBUG,
} from './runtime-constants'

export { destroyActivePages } from './workspace-restore'

export {
  createWorkspaceTab,
  deleteWorkspaceTab,
  duplicateWorkspaceTab,
  renameWorkspacePage,
  renameWorkspaceGroup,
  renameWorkspaceTab,
  setActiveWorkspaceTab,
  setWorkspaceTabExpanded,
} from './workspace-tab-operations'

export {
  restorePersistedWorkspace,
  restoreWorkspaceSnapshot,
  rebuildWindowFromSnapshot,
} from './workspace-restore'

export { initWindow } from './window-init'

function setDevtoolsWidth(width: number): void {
  const nextWidth = clampDevtoolsWidth(width)
  if (nextWidth === uiDevtoolsWidth()) return
  setUiDevtoolsWidth(nextWidth)
  savePreferences()
  scheduleWorkspaceAutosave()
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
  scheduleWorkspaceAutosave()
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
    page.devtoolsHostView = new WebContentsView()
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
  scheduleWorkspaceAutosave()
}

// backgroundLayoutKey, annotationsForPage, pageAnnotationsKey,

function commentBadgeDebug(event: string, details?: Record<string, unknown>): void {
  if (!COMMENT_BADGE_DEBUG) return
  console.log('[comment-badge-debug:main]', { ts: Date.now(), event, ...details })
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
  const nextHoverTarget = pageId ? { id: pageId, kind: 'page' as const } : null
  if (hoverTarget?.id === nextHoverTarget?.id && hoverTarget?.kind === nextHoverTarget?.kind) return
  setHoverTarget(nextHoverTarget)
  markDirty('canvas')
  requestLayout()
}

export function setHoverEntity(
  nextHoverTarget: import('../../shared/types').CanvasHoverTarget,
): void {
  if (hoverTarget?.id === nextHoverTarget?.id && hoverTarget?.kind === nextHoverTarget?.kind) return
  setHoverTarget(nextHoverTarget)
  markDirty('canvas')
  requestLayout()
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
