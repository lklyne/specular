import { contextBridge, ipcRenderer } from 'electron'
import type {
  AgentPresenceCursor,
  ToolbarElectronAPI,
  ToolbarSelectionData,
} from '../shared/types'
import { makeThemeSubscriber, sub } from './ipc-subscribe'

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
  onZoomChanged: sub<number>('zoom-changed'),
  onSelectionChanged: sub<ToolbarSelectionData>('toolbar-selection-changed'),
  onLeftSidebarChanged: sub<boolean>('left-sidebar-changed'),
  onDevtoolsChanged: sub<boolean>('devtools-changed'),
  onThemeChanged: makeThemeSubscriber(),
  onAgentPresenceChanged: sub<AgentPresenceCursor[]>('agent-presence-changed'),
  onFocusAddressBar: sub<void>('focus-address-bar'),
  repoConnectViaPicker: () => ipcRenderer.invoke('repo-connect-via-picker'),
  repoDisconnect: (id) => ipcRenderer.invoke('repo-disconnect', { id }),
}

contextBridge.exposeInMainWorld('electronAPI', api)
