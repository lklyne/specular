import { contextBridge, ipcRenderer } from 'electron'
import type { DevtoolsResizeHandleElectronAPI } from '../shared/types'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: DevtoolsResizeHandleElectronAPI = {
  devtoolsResizeStart: (screenX) => ipcRenderer.send(ipcChannels.devtoolsResizeStart, { screenX }),
  devtoolsResizeMove: (screenX) => ipcRenderer.send(ipcChannels.devtoolsResizeMove, { screenX }),
  devtoolsResizeEnd: () => ipcRenderer.send(ipcChannels.devtoolsResizeEnd),
  getInitialData: () => ipcRenderer.invoke(ipcChannels.getThemeBootstrap),
  onThemeChanged: on(ipcChannels.themeChanged),
}

contextBridge.exposeInMainWorld('electronAPI', api)
