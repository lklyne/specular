import { ipcMain } from 'electron'
import type { Tool, ToolDefaultPatch } from '../../shared/types'
import { applyToolDefaultPatch } from '../runtime/tool-defaults'
import {
  pan,
  requestLayout,
  setPan,
  setZoom,
  zoom,
} from '../runtime/surface-layout'
import {
  focusSelection,
  getSelectedEntityIds,
  openInspectPanel,
  selectedPageId,
  setActiveTool,
  toggleLeftSidebar,
  toggleDevTools,
} from '../runtime/ui-actions'
import { endDevtoolsResize, setDevtoolsWidthFromScreenX } from '../runtime/window-shell'
import { applyNavigationToSelectedPages } from '../navigation-sync'
import { setToolbarDropdownOpen } from '../ui-state'

export function registerToolbarIpc(): void {
  ipcMain.on('zoom-in', () => {
    setZoom(zoom + 0.1)
    requestLayout()
  })

  ipcMain.on('zoom-out', () => {
    setZoom(zoom - 0.1)
    requestLayout()
  })

  ipcMain.on('zoom-reset', () => {
    setZoom(1.0)
    if (!focusSelection()) {
      setPan(0, 0)
      requestLayout()
    }
  })

  ipcMain.on('zoom-set', (_event, level: number) => {
    setZoom(level)
    if (level === 1.0 && focusSelection()) return
    requestLayout()
  })

  ipcMain.on('toolbar-navigate-selection', (_event, url: string) => {
    if (!url) return
    applyNavigationToSelectedPages({ type: 'load-url', url })
  })

  ipcMain.on('toolbar-back-selection', () => {
    if (!getSelectedEntityIds().length) return
    applyNavigationToSelectedPages({ type: 'go-back', fallbackUrl: 'about:blank' })
  })

  ipcMain.on('toolbar-forward-selection', () => {
    if (!getSelectedEntityIds().length) return
    applyNavigationToSelectedPages({ type: 'go-forward', fallbackUrl: 'about:blank' })
  })

  ipcMain.on('toolbar-reload-selection', () => {
    if (!getSelectedEntityIds().length) return
    applyNavigationToSelectedPages({ type: 'reload', fallbackUrl: 'about:blank' })
  })

  ipcMain.on('toolbar-set-tool', (_event, payload: Tool) => {
    if (!payload || typeof payload !== 'object' || typeof payload.kind !== 'string') return
    // Toolbar-initiated add-page inherits the URL of the currently-selected
    // page; the renderer doesn't know which page is selected.
    const tool: Tool =
      payload.kind === 'add-page' && payload.sourcePageId === undefined
        ? { ...payload, sourcePageId: selectedPageId() ?? undefined }
        : payload
    const result = setActiveTool(tool)
    if (result.kind === 'inspect') openInspectPanel()
  })

  ipcMain.on('tool-defaults-set', (_event, patch: ToolDefaultPatch) => {
    if (!patch || typeof patch !== 'object') return
    if (
      patch.scope !== 'add-text' &&
      patch.scope !== 'add-sticky' &&
      patch.scope !== 'add-shape' &&
      patch.scope !== 'draw'
    ) return
    applyToolDefaultPatch(patch)
  })

  ipcMain.on('toggle-devtools', () => {
    toggleDevTools()
  })

  ipcMain.on('toggle-left-sidebar', () => {
    toggleLeftSidebar()
  })

  ipcMain.on('devtools-resize-start', (_event, { screenX }: { screenX: number }) => {
    setDevtoolsWidthFromScreenX(screenX)
  })

  ipcMain.on('devtools-resize-move', (_event, { screenX }: { screenX: number }) => {
    setDevtoolsWidthFromScreenX(screenX)
  })

  ipcMain.on('devtools-resize-end', () => {
    endDevtoolsResize()
  })

  ipcMain.on('toolbar-dropdown-open', () => {
    setToolbarDropdownOpen(true)
    requestLayout()
  })

  ipcMain.on('toolbar-dropdown-close', () => {
    setToolbarDropdownOpen(false)
    requestLayout()
  })
}
