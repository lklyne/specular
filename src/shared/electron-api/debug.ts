import type { CursorTuningParams } from '../cursor-tuning'
import type { DebugBootstrapData, ThemeData } from '../types'
import type { TraceSummary } from '../trace-summary'
import type {
  PanZoomPerfTestResult,
  PanZoomPerfTestState,
} from '../pan-zoom-perf-test'

export interface PerfTraceState {
  recording: boolean
  status: 'idle' | 'starting' | 'recording' | 'stopping'
  /** Epoch ms of recording start, null when idle. */
  startedAt: number | null
}

export interface PerfTraceFileEntry {
  /** Basename within the logs directory, e.g. specular-trace-….json */
  fileName: string
  sizeBytes: number
  /** Epoch ms mtime. */
  modifiedAt: number
  hasSummary: boolean
}

export interface DebugElectronAPI {
  getInitialData: () => Promise<DebugBootstrapData>
  updateCursorSplineViz: (on: boolean) => void
  onCursorSplineVizChanged: (callback: (on: boolean) => void) => () => void
  updateCursorTuning: (params: CursorTuningParams) => void
  resetCursorTuning: () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
  perfTraceGetState: () => Promise<PerfTraceState>
  perfTraceToggle: () => Promise<void>
  perfPanZoomGetState: () => Promise<PanZoomPerfTestState>
  perfPanZoomRun: () => Promise<PanZoomPerfTestResult>
  perfPanZoomStop: () => Promise<void>
  perfTraceList: () => Promise<PerfTraceFileEntry[]>
  /** Analyzes on demand; cached to <trace>.summary.json beside the trace. */
  perfTraceGetSummary: (fileName: string) => Promise<TraceSummary | null>
  perfTraceReveal: (fileName: string) => void
  onPerfTraceStateChanged: (callback: (state: PerfTraceState) => void) => () => void
  onPerfPanZoomStateChanged: (callback: (state: PanZoomPerfTestState) => void) => () => void
}
