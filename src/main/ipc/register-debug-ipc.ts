import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import {
  DEFAULT_CURSOR_TUNING,
  normalizeCursorTuning,
} from '../../shared/cursor-tuning'
import type { DebugBootstrapData } from '../../shared/types'
import {
  broadcastCursorSplineViz,
  getCursorSplineViz,
  getCursorTuning,
  isDark,
  saveCursorSplineViz,
  saveCursorTuning,
} from '../runtime/preferences'

export function registerDebugIpc(): void {
  ipcMain.handle(ipcChannels.debugGetInitialData, async (): Promise<DebugBootstrapData> => ({
    theme: { isDark: isDark() },
    cursorSplineViz: getCursorSplineViz(),
    cursorTuning: getCursorTuning(),
  }))

  ipcMain.on(ipcChannels.debugUpdateCursorSplineViz, (_event, on: unknown) => {
    saveCursorSplineViz(on === true)
    broadcastCursorSplineViz()
  })

  ipcMain.on(ipcChannels.debugUpdateCursorTuning, (_event, raw: unknown) => {
    saveCursorTuning(normalizeCursorTuning(raw))
  })

  ipcMain.on(ipcChannels.debugResetCursorTuning, () => {
    saveCursorTuning(DEFAULT_CURSOR_TUNING)
  })
}
