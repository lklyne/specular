import { contextBridge, ipcRenderer } from 'electron'
import type { DevtoolsResizeHandleElectronAPI } from '../shared/types'
import { on } from './ipc-helpers'

const api: DevtoolsResizeHandleElectronAPI = {
  devtoolsResizeStart: (screenX) => ipcRenderer.send('devtools-resize-start', { screenX }),
  devtoolsResizeMove: (screenX) => ipcRenderer.send('devtools-resize-move', { screenX }),
  devtoolsResizeEnd: () => ipcRenderer.send('devtools-resize-end'),
  getInitialData: () => ipcRenderer.invoke('get-theme-bootstrap'),
  onThemeChanged: on<{ isDark: boolean }>('theme-changed'),
}

contextBridge.exposeInMainWorld('electronAPI', api)
