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
}

contextBridge.exposeInMainWorld('electronAPI', api)
