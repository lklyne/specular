import { contextBridge, ipcRenderer } from 'electron'
import type { DebugElectronAPI } from '../shared/types'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: DebugElectronAPI = {
  getInitialData: () => ipcRenderer.invoke('debug:get-initial-data'),
  updateCursorSplineViz: (enabled) =>
    ipcRenderer.send('debug:update-cursor-spline-viz', enabled),
  onCursorSplineVizChanged: on<boolean>('cursor-spline-viz-changed'),
  updateCursorTuning: (params) =>
    ipcRenderer.send('debug:update-cursor-tuning', params),
  resetCursorTuning: () => ipcRenderer.send('debug:reset-cursor-tuning'),
  onThemeChanged: on(ipcChannels.themeChanged),
}

contextBridge.exposeInMainWorld('electronAPI', api)
