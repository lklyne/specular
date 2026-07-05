import type { CursorTuningParams } from '../cursor-tuning'
import type { DebugBootstrapData, ThemeData } from '../types'

export interface DebugElectronAPI {
  getInitialData: () => Promise<DebugBootstrapData>
  updateCursorSplineViz: (on: boolean) => void
  onCursorSplineVizChanged: (callback: (on: boolean) => void) => () => void
  updateCursorTuning: (params: CursorTuningParams) => void
  resetCursorTuning: () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}
