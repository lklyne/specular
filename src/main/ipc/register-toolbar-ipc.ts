import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type { Tool, ToolDefaultPatch } from '../../shared/types'
import { applyToolDefaultPatch } from '../runtime/tool-defaults'
import { pan, zoom } from '../runtime/runtime-context'
import { requestLayout, setPan, setZoom } from '../runtime/viewport-control'
import {
  focusSelection,
  restoreFocusCamera,
  selectedPageId,
  setActiveTool,
  toggleLeftSidebar,
  toggleDevTools,
} from '../runtime/ui-actions'
import { endDevtoolsResize, setDevtoolsWidthFromScreenX } from '../runtime/window-shell'
import { setToolbarDropdownOpen, setToolbarTooltipOpen } from '../ui-state'

export function registerToolbarIpc(): void {
  ipcMain.on(ipcChannels.zoomIn, () => {
    if (restoreFocusCamera()) return
    setZoom(zoom + 0.1)
    requestLayout()
  })

  ipcMain.on(ipcChannels.zoomOut, () => {
    if (restoreFocusCamera()) return
    setZoom(zoom - 0.1)
    requestLayout()
  })

  ipcMain.on(ipcChannels.zoomReset, () => {
    if (restoreFocusCamera()) return
    setZoom(1.0)
    if (!focusSelection({ animate: false })) {
      setPan(0, 0)
      requestLayout()
    }
  })

  ipcMain.on(ipcChannels.zoomSet, (_event, level: number) => {
    if (restoreFocusCamera()) return
    setZoom(level)
    if (level === 1.0 && focusSelection({ animate: false })) return
    requestLayout()
  })

  ipcMain.on(ipcChannels.toolbarSetTool, (_event, payload: Tool) => {
    if (!payload || typeof payload !== 'object' || typeof payload.kind !== 'string') return
    // Toolbar-initiated add-page inherits the URL of the currently-selected
    // page; the renderer doesn't know which page is selected.
    const tool: Tool =
      payload.kind === 'add-page' && payload.sourcePageId === undefined
        ? { ...payload, sourcePageId: selectedPageId() ?? undefined }
        : payload
    // Activating the eyedropper enables in-page hover highlighting (via
    // setActiveTool → syncInspectionState) but does NOT open the right panel
    // or switch tabs — that happens on click (inspectNodeSelect). Keeps a
    // live browser-devtools session intact when the tool is picked.
    setActiveTool(tool)
  })

  ipcMain.on(ipcChannels.toolDefaultsSet, (_event, patch: ToolDefaultPatch) => {
    if (!patch || typeof patch !== 'object') return
    if (
      patch.scope !== 'add-text' &&
      patch.scope !== 'add-sticky' &&
      patch.scope !== 'add-shape' &&
      patch.scope !== 'draw' &&
      patch.scope !== 'connect'
    ) return
    applyToolDefaultPatch(patch)
  })

  ipcMain.on(ipcChannels.toggleDevtools, () => {
    toggleDevTools()
  })

  ipcMain.on(ipcChannels.toggleLeftSidebar, () => {
    toggleLeftSidebar()
  })

  ipcMain.on(ipcChannels.devtoolsResizeStart, (_event, { screenX }: { screenX: number }) => {
    setDevtoolsWidthFromScreenX(screenX)
  })

  ipcMain.on(ipcChannels.devtoolsResizeMove, (_event, { screenX }: { screenX: number }) => {
    setDevtoolsWidthFromScreenX(screenX)
  })

  ipcMain.on(ipcChannels.devtoolsResizeEnd, () => {
    endDevtoolsResize()
  })

  ipcMain.on(ipcChannels.toolbarDropdownOpen, () => {
    setToolbarDropdownOpen(true)
    requestLayout()
  })

  ipcMain.on(ipcChannels.toolbarDropdownClose, () => {
    setToolbarDropdownOpen(false)
    requestLayout()
  })

  ipcMain.on(ipcChannels.toolbarTooltipOpen, () => {
    setToolbarTooltipOpen(true)
    requestLayout()
  })

  ipcMain.on(ipcChannels.toolbarTooltipClose, () => {
    setToolbarTooltipOpen(false)
    requestLayout()
  })
}
