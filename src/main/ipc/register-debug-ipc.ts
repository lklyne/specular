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
  ipcMain.handle('debug:get-initial-data', async (): Promise<DebugBootstrapData> => ({
    theme: { isDark: isDark() },
    cursorSplineViz: getCursorSplineViz(),
    cursorTuning: getCursorTuning(),
  }))

  ipcMain.on('debug:update-cursor-spline-viz', (_event, on: unknown) => {
    saveCursorSplineViz(on === true)
    broadcastCursorSplineViz()
  })

  ipcMain.on('debug:update-cursor-tuning', (_event, raw: unknown) => {
    saveCursorTuning(normalizeCursorTuning(raw))
  })

  ipcMain.on('debug:reset-cursor-tuning', () => {
    saveCursorTuning(DEFAULT_CURSOR_TUNING)
  })
}
