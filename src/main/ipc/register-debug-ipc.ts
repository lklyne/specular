import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import {
  DEFAULT_CURSOR_TUNING,
  normalizeCursorTuning,
} from '../../shared/cursor-tuning'
import type { DebugBootstrapData } from '../../shared/types'
import type { PerfTraceFileEntry, PerfTraceState } from '../../shared/electron-api/debug'
import type { TraceSummary } from '../../shared/trace-summary'
import {
  broadcastCursorSplineViz,
  getCursorSplineViz,
  getCursorTuning,
  getThemeMode,
  isDark,
  saveCursorSplineViz,
  saveCursorTuning,
} from '../runtime/preferences'
import {
  getPerfTraceState,
  getTraceSummary,
  listPerfTraces,
  revealTrace,
  togglePerfTrace,
} from '../perf-trace'

export function registerDebugIpc(): void {
  ipcMain.handle(ipcChannels.debugGetInitialData, async (): Promise<DebugBootstrapData> => ({
    theme: { isDark: isDark(), themeMode: getThemeMode() },
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

  ipcMain.handle(ipcChannels.debugPerfTraceGetState, async (): Promise<PerfTraceState> => getPerfTraceState())

  ipcMain.handle(ipcChannels.debugPerfTraceToggle, async (): Promise<void> => {
    await togglePerfTrace()
  })

  ipcMain.handle(ipcChannels.debugPerfTraceList, async (): Promise<PerfTraceFileEntry[]> => listPerfTraces())

  ipcMain.handle(
    ipcChannels.debugPerfTraceGetSummary,
    async (_event, fileName: unknown): Promise<TraceSummary | null> => {
      if (typeof fileName !== 'string') return null
      return getTraceSummary(fileName)
    },
  )

  ipcMain.on(ipcChannels.debugPerfTraceReveal, (_event, fileName: unknown) => {
    if (typeof fileName === 'string') revealTrace(fileName)
  })
}
