import { contextBridge, ipcRenderer } from 'electron'
import type {
  DebugElectronAPI,
  ThemeData,
} from '../shared/types'
import { on } from './ipc-helpers'

const api: DebugElectronAPI = {
  getInitialData: () => ipcRenderer.invoke('debug:get-initial-data'),
  updateCursorSplineViz: (enabled) =>
    ipcRenderer.send('debug:update-cursor-spline-viz', enabled),
  onCursorSplineVizChanged: on<boolean>('cursor-spline-viz-changed'),
  updateCursorTuning: (params) =>
    ipcRenderer.send('debug:update-cursor-tuning', params),
  resetCursorTuning: () => ipcRenderer.send('debug:reset-cursor-tuning'),
  onThemeChanged: on<ThemeData>('theme-changed'),
}

contextBridge.exposeInMainWorld('electronAPI', api)
