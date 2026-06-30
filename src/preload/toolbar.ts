import { contextBridge, ipcRenderer } from 'electron'
import type {
  AgentPresenceCursor,
  ToolbarElectronAPI,
  ToolbarSelectionData,
} from '../shared/types'

const api: ToolbarElectronAPI = {
  zoomIn: () => ipcRenderer.send('zoom-in'),
  zoomOut: () => ipcRenderer.send('zoom-out'),
  zoomReset: () => ipcRenderer.send('zoom-reset'),
  zoomSet: (level) => ipcRenderer.send('zoom-set', level),
  navigateSelection: (url) => ipcRenderer.send('toolbar-navigate-selection', url),
  goBackSelection: () => ipcRenderer.send('toolbar-back-selection'),
  goForwardSelection: () => ipcRenderer.send('toolbar-forward-selection'),
  reloadSelection: () => ipcRenderer.send('toolbar-reload-selection'),
  setTool: (tool) => ipcRenderer.send('toolbar-set-tool', tool),
  reloadApp: () => ipcRenderer.send('reload-app'),
  toggleTheme: () => ipcRenderer.send('toggle-theme'),
  getInitialData: () => ipcRenderer.invoke('get-theme-bootstrap'),
  toggleLeftSidebar: () => ipcRenderer.send('toggle-left-sidebar'),
  toggleDevTools: () => ipcRenderer.send('toggle-devtools'),
  dropdownOpen: () => ipcRenderer.send('toolbar-dropdown-open'),
  dropdownClose: () => ipcRenderer.send('toolbar-dropdown-close'),
  setTextEditing: (active) => ipcRenderer.send('canvas-set-text-editing', { active }),
  onZoomChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, value: number) => callback(value)
    ipcRenderer.on('zoom-changed', handler)
    return () => ipcRenderer.removeListener('zoom-changed', handler)
  },
  onSelectionChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: ToolbarSelectionData) =>
      callback(data)
    ipcRenderer.on('toolbar-selection-changed', handler)
    return () => ipcRenderer.removeListener('toolbar-selection-changed', handler)
  },
  onLeftSidebarChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, open: boolean) => callback(open)
    ipcRenderer.on('left-sidebar-changed', handler)
    return () => ipcRenderer.removeListener('left-sidebar-changed', handler)
  },
  onDevtoolsChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, open: boolean) => callback(open)
    ipcRenderer.on('devtools-changed', handler)
    return () => ipcRenderer.removeListener('devtools-changed', handler)
  },
  onThemeChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: { isDark: boolean }) =>
      callback(data)
    ipcRenderer.on('theme-changed', handler)
    return () => ipcRenderer.removeListener('theme-changed', handler)
  },
  onAgentPresenceChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, cursors: AgentPresenceCursor[]) =>
      callback(cursors)
    ipcRenderer.on('agent-presence-changed', handler)
    return () => ipcRenderer.removeListener('agent-presence-changed', handler)
  },
  onFocusAddressBar: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('focus-address-bar', handler)
    return () => ipcRenderer.removeListener('focus-address-bar', handler)
  },
  repoConnectViaPicker: () => ipcRenderer.invoke('repo-connect-via-picker'),
  repoDisconnect: (id) => ipcRenderer.invoke('repo-disconnect', { id }),
}

contextBridge.exposeInMainWorld('electronAPI', api)
