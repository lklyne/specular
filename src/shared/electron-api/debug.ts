import type { CursorTuningParams } from '../cursor-tuning'
import type { DebugBootstrapData, ThemeData } from '../types'
import type { TraceSummary } from '../trace-summary'
import type {
  PanZoomPerfTestResult,
  PanZoomPerfTestState,
} from '../pan-zoom-perf-test'
import type { ProcessMetricsSample, VisibilityProbeResult } from '../process-metrics'

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
  /** One read-only snapshot of `app.getAppMetrics()` with view attribution. */
  processMetricsSample: () => Promise<ProcessMetricsSample>
  /** Measures whether `setVisible(false)` throttles culled pages. Wakes each
   *  probed renderer twice, so it is on-demand only. */
  visibilityProbeRun: (windowMs?: number) => Promise<VisibilityProbeResult>
  /** Writes text to the system clipboard from main — the debug window is not a
   *  secure context, so `navigator.clipboard` is unavailable there. */
  copyText: (text: string) => void
}
