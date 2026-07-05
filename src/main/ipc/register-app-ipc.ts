import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain, nativeTheme } from 'electron'
import {
  getCanvasLayoutData,
  getLeftSidebarData,
} from '../runtime/canvas-layout-data'
import { isDark } from '../runtime/preferences'
import { requestLayout } from '../runtime/viewport-control'
import { rebuildWindowFromSnapshot } from '../runtime/window-shell'
import {
  currentPersistedWorkspaceRecord,
  workspaceSnapshot,
} from '../runtime/workspace-tabs'
import { restorePersistedWorkspace } from '../runtime/workspace-restore'
import { selectionDebug } from '../runtime/runtime-constants'

export function registerAppIpc(): void {
  ipcMain.on(
    ipcChannels.debugLog,
    (
      _event,
      payload: { source: string; level: 'log' | 'warn' | 'error'; args: unknown[] },
    ) => {
      const prefix = `[renderer:${payload.source}]`
      const method =
        payload.level === 'warn'
          ? console.warn
          : payload.level === 'error'
            ? console.error
            : console.log
      method(prefix, ...payload.args)
    },
  )

  ipcMain.on(ipcChannels.toggleTheme, () => {
    nativeTheme.themeSource = nativeTheme.shouldUseDarkColors ? 'light' : 'dark'
  })

  ipcMain.handle(ipcChannels.getThemeBootstrap, async () => ({ theme: { isDark: isDark() } }))

  ipcMain.handle(ipcChannels.getLeftSidebarBootstrap, async () => ({
    theme: { isDark: isDark() },
    sidebarData: getLeftSidebarData(),
  }))

  ipcMain.handle(ipcChannels.getCanvasLayoutBootstrap, async () => ({
    theme: { isDark: isDark() },
    layoutData: getCanvasLayoutData(),
  }))

  ipcMain.handle(ipcChannels.getFloatingUiBootstrap, async () => ({
    theme: { isDark: isDark() },
    layoutData: getCanvasLayoutData(),
    surfaceOrigin: { x: 0, y: 0 },
  }))

  ipcMain.on(ipcChannels.reloadApp, () => {
    selectionDebug('ipc:reload-app')
    try {
      const record = currentPersistedWorkspaceRecord()
      rebuildWindowFromSnapshot(workspaceSnapshot())
      restorePersistedWorkspace(record)
      requestLayout()
    } catch (error) {
      console.error('Failed to relaunch app with current state:', error)
    }
  })
}
