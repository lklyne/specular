import { contextBridge, ipcRenderer } from 'electron'
import type {
  DebugElectronAPI,
  ThemeData,
} from '../shared/types'

const api: DebugElectronAPI = {
  getInitialData: () => ipcRenderer.invoke('debug:get-initial-data'),
  updateCursorSplineViz: (on) =>
    ipcRenderer.send('debug:update-cursor-spline-viz', on),
  onCursorSplineVizChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, on: boolean) =>
      callback(on)
    ipcRenderer.on('cursor-spline-viz-changed', handler)
    return () => ipcRenderer.removeListener('cursor-spline-viz-changed', handler)
  },
  updateCursorTuning: (params) =>
    ipcRenderer.send('debug:update-cursor-tuning', params),
  resetCursorTuning: () => ipcRenderer.send('debug:reset-cursor-tuning'),
  onThemeChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: ThemeData) =>
      callback(data)
    ipcRenderer.on('theme-changed', handler)
    return () => ipcRenderer.removeListener('theme-changed', handler)
  },
}

contextBridge.exposeInMainWorld('electronAPI', api)
