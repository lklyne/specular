import { contextBridge, ipcRenderer } from 'electron'
import type {
  CursorMotionParams,
  DebugElectronAPI,
  PresenceDebugEntry,
} from '../shared/types'
import { makeThemeSubscriber, sub } from './ipc-subscribe'

const api: DebugElectronAPI = {
  getInitialData: () => ipcRenderer.invoke('debug:get-initial-data'),
  updateCursorMotion: (params) =>
    ipcRenderer.send('debug:update-cursor-motion', params),
  resetCursorMotion: () => ipcRenderer.send('debug:reset-cursor-motion'),
  onCursorMotionChanged: sub<CursorMotionParams>('cursor-motion-changed'),
  updateCursorSplineViz: (on) =>
    ipcRenderer.send('debug:update-cursor-spline-viz', on),
  onCursorSplineVizChanged: sub<boolean>('cursor-spline-viz-changed'),
  updateCursorTuning: (params) =>
    ipcRenderer.send('debug:update-cursor-tuning', params),
  resetCursorTuning: () => ipcRenderer.send('debug:reset-cursor-tuning'),
  onPresenceTimelineAppend: sub<PresenceDebugEntry>('presence-timeline-append'),
  onThemeChanged: makeThemeSubscriber(),
}

contextBridge.exposeInMainWorld('electronAPI', api)
