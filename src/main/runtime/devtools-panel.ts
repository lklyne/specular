// fallow-ignore-file circular-dependencies
// Suppressed: see #141. runtime-core → workspace-restore/window-init import devtools-panel back
/**
 * DevTools panel lifecycle — open, close, tab switching, resize.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import { markDirty } from './layout-dirty'
import {
  toolbarView,
  win,
  setDevtoolsView,
} from './view-refs'
import {
  pages,
  incrementBrowserDevtoolsAttachGeneration,
} from './runtime-context'
import {
  devtoolsOpen as uiDevtoolsOpen,
  focusAnnotation as focusUiAnnotation,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedPageIndex as uiSelectedPageIndex,
  setDevtoolsOpen as setUiDevtoolsOpen,
  setLeftSidebarOpen as setUiLeftSidebarOpen,
  setDevtoolsPanelTab as setUiDevtoolsPanelTab,
} from '../ui-state'
import { recenterFocusPresentation, requestLayout } from './viewport-control'
import { syncInspectionState } from './inspect-session'
import { devtoolsPanelDebug } from './runtime-constants'

export function notifyDevtoolsChanged(): void {
  if (toolbarView) {
    toolbarView.webContents.send(ipcChannels.leftSidebarChanged, uiLeftSidebarOpen())
    toolbarView.webContents.send(ipcChannels.devtoolsChanged, uiDevtoolsOpen())
  }
}

export function toggleLeftSidebar(): void {
  setUiLeftSidebarOpen(!uiLeftSidebarOpen())
  markDirty('sidebar', 'canvas')
  notifyDevtoolsChanged()
  recenterFocusPresentation(undefined, { animate: false })
  requestLayout()
}

export function closeDevTools(): void {
  if (!uiDevtoolsOpen()) return
  incrementBrowserDevtoolsAttachGeneration()

  for (const page of pages) {
    try {
      page.pageView.webContents.closeDevTools()
    } catch {
      // Ignore close races during shutdown or retargeting.
    }
  }
  // The devtools cluster views (background / header / resize handle) and
  // every page's devtools host view are parked off-screen by the layout
  // pass once devtools is closed — no imperative hiding needed here.
  setDevtoolsView(null)

  setUiDevtoolsOpen(false)
  syncInspectionState()
  notifyDevtoolsChanged()
  recenterFocusPresentation(undefined, { animate: false })
  requestLayout()
}

export function toggleDevTools(): void {
  if (!win) return
  const start = Date.now()
  devtoolsPanelDebug('toggle:start', { openBefore: uiDevtoolsOpen() })

  if (uiDevtoolsOpen()) {
    closeDevTools()
    devtoolsPanelDebug('toggle:close-complete', { durationMs: Date.now() - start })
    return
  }

  focusUiAnnotation(null)

  setUiDevtoolsOpen(true)
  notifyDevtoolsChanged()
  syncInspectionState()
  recenterFocusPresentation(undefined, { animate: false })
  requestLayout()
  devtoolsPanelDebug('toggle:open-complete', { durationMs: Date.now() - start })
}

/**
 * Dismiss the browser devtools panel without destroying the devtools session.
 *
 * Electron's setDevToolsWebContents is a one-time binding per page WebContents —
 * calling closeDevTools() invalidates the binding and it can't be re-established.
 * So we just hide the views and switch tabs, keeping the session alive for reuse.
 */
export function dismissBrowserDevTools(): void {
  if (!win) return
  incrementBrowserDevtoolsAttachGeneration()

  // The layout pass hides every page's devtools host view while the
  // panel is off the browser-devtools tab; the session stays alive.
  setDevtoolsView(null)

  setUiDevtoolsPanelTab('comments')
  notifyDevtoolsChanged()
  syncInspectionState()
  requestLayout()
}

export function openDevToolsForSelectedPage(): void {
  const selectedPageIdx = uiSelectedPageIndex(pages.map((p) => p.id))
  if (!win || selectedPageIdx === null) return

  setUiDevtoolsPanelTab('browser-devtools')
  focusUiAnnotation(null)

  setUiDevtoolsOpen(true)
  notifyDevtoolsChanged()
  syncInspectionState()
  recenterFocusPresentation(undefined, { animate: false })
  requestLayout()
}

export function openInspectPanel(): void {
  if (!win) return
  if (!uiDevtoolsOpen()) {
    setUiDevtoolsOpen(true)
    notifyDevtoolsChanged()
  }
  setUiDevtoolsPanelTab('inspect')
  focusUiAnnotation(null)
  markDirty('toolbar', 'canvas')
  syncInspectionState()
  recenterFocusPresentation(undefined, { animate: false })
  requestLayout()
}

export function openCommentsPanel(annotationId?: string): void {
  if (!win) return
  if (!uiDevtoolsOpen()) {
    setUiDevtoolsOpen(true)
    notifyDevtoolsChanged()
  }
  setUiDevtoolsPanelTab('comments')
  focusUiAnnotation(annotationId ?? null)
  markDirty('toolbar', 'canvas')
  syncInspectionState()
  recenterFocusPresentation(undefined, { animate: false })
  requestLayout()
}

export function focusAnnotation(annotationId?: string): void {
  focusUiAnnotation(annotationId ?? null)
  syncInspectionState()
}
