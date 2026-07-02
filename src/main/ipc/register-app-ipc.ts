import { ipcMain, nativeTheme } from 'electron'
import {
  getCanvasLayoutData,
  getLeftSidebarData,
  isDark,
  requestLayout,
} from '../runtime/surface-layout'
import { rebuildWindowFromSnapshot } from '../runtime/window-shell'
import {
  currentPersistedWorkspaceRecord,
  restorePersistedWorkspace,
  workspaceSnapshot,
} from '../runtime/workspace-session'
import { selectionDebug } from '../runtime/runtime-constants'

export function registerAppIpc(): void {
  ipcMain.on(
    'debug-log',
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

  ipcMain.on('toggle-theme', () => {
    nativeTheme.themeSource = nativeTheme.shouldUseDarkColors ? 'light' : 'dark'
  })

  ipcMain.handle('get-theme-bootstrap', async () => ({ theme: { isDark: isDark() } }))

  ipcMain.handle('get-left-sidebar-bootstrap', async () => ({
    theme: { isDark: isDark() },
    sidebarData: getLeftSidebarData(),
  }))

  ipcMain.handle('get-canvas-layout-bootstrap', async () => ({
    theme: { isDark: isDark() },
    layoutData: getCanvasLayoutData(),
  }))

  ipcMain.handle('get-floating-ui-bootstrap', async () => ({
    theme: { isDark: isDark() },
    layoutData: getCanvasLayoutData(),
    surfaceOrigin: { x: 0, y: 0 },
  }))

  ipcMain.on('reload-app', () => {
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
