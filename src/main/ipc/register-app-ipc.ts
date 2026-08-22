import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type { AppThemeMode } from '../../shared/types'
import {
  getCanvasLayoutData,
  getLeftSidebarData,
} from '../runtime/canvas-layout-data'
import { getThemeMode, isDark, setThemeMode } from '../runtime/preferences'
import { requestLayout } from '../runtime/viewport-control'
import { rebuildWindowFromSnapshot } from '../runtime/window-shell'
import {
  currentPersistedWorkspaceRecord,
  spaceSnapshot,
} from '../runtime/space-tabs'
import { restorePersistedSpace } from '../runtime/space-restore'
import { selectionDebug } from '../runtime/runtime-constants'
import { sceneTargetFor } from '../runtime/runtime-patch-broadcast'
import { filterSceneSnapshot } from '../../shared/runtime-store-filter'

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

  ipcMain.on(ipcChannels.setThemeMode, (_event, payload: { mode: AppThemeMode }) => {
    const mode = payload?.mode
    if (mode !== 'system' && mode !== 'light' && mode !== 'dark') return
    setThemeMode(mode)
  })

  ipcMain.handle(ipcChannels.getThemeBootstrap, async () => ({
    theme: { isDark: isDark(), themeMode: getThemeMode() },
  }))

  ipcMain.handle(ipcChannels.getLeftSidebarBootstrap, async () => ({
    theme: { isDark: isDark(), themeMode: getThemeMode() },
    sidebarData: getLeftSidebarData(),
  }))

  ipcMain.handle(ipcChannels.getCanvasLayoutBootstrap, async (event) => {
    // Seeded through the same routing as every later send, so a renderer never
    // starts out holding a slice it will never be sent an update for — which
    // would read as drift the moment the first snapshot lands.
    const target = sceneTargetFor(event.sender)
    const layoutData = getCanvasLayoutData()
    return {
      theme: { isDark: isDark(), themeMode: getThemeMode() },
      layoutData: target ? filterSceneSnapshot(layoutData, target) : layoutData,
    }
  })

  ipcMain.handle(ipcChannels.getFloatingUiBootstrap, async () => ({
    theme: { isDark: isDark(), themeMode: getThemeMode() },
    layoutData: getCanvasLayoutData(),
    surfaceOrigin: { x: 0, y: 0 },
  }))

  ipcMain.on(ipcChannels.reloadApp, () => {
    selectionDebug('ipc:reload-app')
    try {
      const record = currentPersistedWorkspaceRecord()
      rebuildWindowFromSnapshot(spaceSnapshot())
      restorePersistedSpace(record)
      requestLayout()
    } catch (error) {
      console.error('Failed to relaunch app with current state:', error)
    }
  })
}
