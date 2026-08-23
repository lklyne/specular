import { ipcChannels } from '../../shared/ipc-contract'
import { clipboard, ipcMain } from 'electron'
import {
  DEFAULT_CURSOR_TUNING,
  normalizeCursorTuning,
} from '../../shared/cursor-tuning'
import type { DebugBootstrapData } from '../../shared/types'
import type { PerfTraceFileEntry, PerfTraceState } from '../../shared/electron-api/debug'
import type { TraceSummary } from '../../shared/trace-summary'
import type {
  ProcessMetricsSample,
  VisibilityProbeResult,
} from '../../shared/process-metrics'
import type {
  PanZoomPerfTestResult,
  PanZoomPerfTestState,
} from '../../shared/pan-zoom-perf-test'
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
import {
  getPanZoomPerfTestState,
  runPanZoomPerfTest,
  stopPanZoomPerfTest,
} from '../pan-zoom-perf-test'
import { sampleProcessMetrics } from '../process-metrics'
import { runVisibilityProbe } from '../visibility-probe'

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

  ipcMain.handle(
    ipcChannels.debugPerfPanZoomGetState,
    async (): Promise<PanZoomPerfTestState> => getPanZoomPerfTestState(),
  )

  ipcMain.handle(
    ipcChannels.debugPerfPanZoomRun,
    async (): Promise<PanZoomPerfTestResult> => runPanZoomPerfTest(),
  )

  ipcMain.handle(ipcChannels.debugPerfPanZoomStop, async (): Promise<void> => {
    await stopPanZoomPerfTest()
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

  ipcMain.handle(
    ipcChannels.debugProcessMetricsSample,
    async (): Promise<ProcessMetricsSample> => sampleProcessMetrics(),
  )

  ipcMain.on(ipcChannels.debugCopyText, (_event, text: unknown) => {
    if (typeof text === 'string') clipboard.writeText(text)
  })

  ipcMain.handle(
    ipcChannels.debugVisibilityProbeRun,
    async (_event, windowMs: unknown): Promise<VisibilityProbeResult> =>
      runVisibilityProbe({ windowMs: typeof windowMs === 'number' ? windowMs : undefined }),
  )
}
