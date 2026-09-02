import { contextBridge, ipcRenderer } from 'electron'
import type { DebugElectronAPI } from '../shared/electron-api/debug'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: DebugElectronAPI = {
  getInitialData: () => ipcRenderer.invoke(ipcChannels.debugGetInitialData),
  updateCursorSplineViz: (enabled) =>
    ipcRenderer.send(ipcChannels.debugUpdateCursorSplineViz, enabled),
  onCursorSplineVizChanged: on<boolean>(ipcChannels.cursorSplineVizChanged),
  updateCursorTuning: (params) =>
    ipcRenderer.send(ipcChannels.debugUpdateCursorTuning, params),
  resetCursorTuning: () => ipcRenderer.send(ipcChannels.debugResetCursorTuning),
  onThemeChanged: on(ipcChannels.themeChanged),
  perfTraceGetState: () => ipcRenderer.invoke(ipcChannels.debugPerfTraceGetState),
  perfTraceToggle: () => ipcRenderer.invoke(ipcChannels.debugPerfTraceToggle),
  perfPanZoomGetState: () => ipcRenderer.invoke(ipcChannels.debugPerfPanZoomGetState),
  perfPanZoomRun: () => ipcRenderer.invoke(ipcChannels.debugPerfPanZoomRun),
  perfPanZoomStop: () => ipcRenderer.invoke(ipcChannels.debugPerfPanZoomStop),
  perfTraceList: () => ipcRenderer.invoke(ipcChannels.debugPerfTraceList),
  perfTraceGetSummary: (fileName) =>
    ipcRenderer.invoke(ipcChannels.debugPerfTraceGetSummary, fileName),
  perfTraceReveal: (fileName) => ipcRenderer.send(ipcChannels.debugPerfTraceReveal, fileName),
  onPerfTraceStateChanged: on(ipcChannels.debugPerfTraceStateChanged),
  onPerfPanZoomStateChanged: on(ipcChannels.debugPerfPanZoomStateChanged),
  processMetricsSample: () => ipcRenderer.invoke(ipcChannels.debugProcessMetricsSample),
  visibilityProbeRun: (windowMs) =>
    ipcRenderer.invoke(ipcChannels.debugVisibilityProbeRun, windowMs),
  copyText: (text) => ipcRenderer.send(ipcChannels.debugCopyText, text),
}

contextBridge.exposeInMainWorld('electronAPI', api)
