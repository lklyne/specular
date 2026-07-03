import { contextBridge, ipcRenderer } from 'electron'
import type {
  AgentPresenceCursor,
  ToolbarElectronAPI,
  ToolbarSelectionData,
} from '../shared/types'
import { on } from './ipc-helpers'

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
  onZoomChanged: on<number>('zoom-changed'),
  onSelectionChanged: on<ToolbarSelectionData>('toolbar-selection-changed'),
  onLeftSidebarChanged: on<boolean>('left-sidebar-changed'),
  onDevtoolsChanged: on<boolean>('devtools-changed'),
  onThemeChanged: on<{ isDark: boolean }>('theme-changed'),
  onAgentPresenceChanged: on<AgentPresenceCursor[]>('agent-presence-changed'),
  onFocusAddressBar: on('focus-address-bar'),
  repoConnectViaPicker: () => ipcRenderer.invoke('repo-connect-via-picker'),
  repoDisconnect: (id) => ipcRenderer.invoke('repo-disconnect', { id }),
}

contextBridge.exposeInMainWorld('electronAPI', api)
