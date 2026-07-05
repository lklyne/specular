import { contextBridge, ipcRenderer } from 'electron'
import type { AgentPresenceCursor, ToolbarSelectionData } from '../shared/types'
import type { ToolbarElectronAPI } from '../shared/electron-api/toolbar'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: ToolbarElectronAPI = {
  zoomIn: () => ipcRenderer.send(ipcChannels.zoomIn),
  zoomOut: () => ipcRenderer.send(ipcChannels.zoomOut),
  zoomReset: () => ipcRenderer.send(ipcChannels.zoomReset),
  zoomSet: (level) => ipcRenderer.send(ipcChannels.zoomSet, level),
  navigateSelection: (url) => ipcRenderer.send(ipcChannels.toolbarNavigateSelection, url),
  goBackSelection: () => ipcRenderer.send(ipcChannels.toolbarBackSelection),
  goForwardSelection: () => ipcRenderer.send(ipcChannels.toolbarForwardSelection),
  reloadSelection: () => ipcRenderer.send(ipcChannels.toolbarReloadSelection),
  setTool: (tool) => ipcRenderer.send(ipcChannels.toolbarSetTool, tool),
  reloadApp: () => ipcRenderer.send(ipcChannels.reloadApp),
  toggleTheme: () => ipcRenderer.send(ipcChannels.toggleTheme),
  getInitialData: () => ipcRenderer.invoke(ipcChannels.getThemeBootstrap),
  toggleLeftSidebar: () => ipcRenderer.send(ipcChannels.toggleLeftSidebar),
  toggleDevTools: () => ipcRenderer.send(ipcChannels.toggleDevtools),
  dropdownOpen: () => ipcRenderer.send(ipcChannels.toolbarDropdownOpen),
  dropdownClose: () => ipcRenderer.send(ipcChannels.toolbarDropdownClose),
  setTextEditing: (active) => ipcRenderer.send(ipcChannels.canvasSetTextEditing, { active }),
  onZoomChanged: on<number>(ipcChannels.zoomChanged),
  onSelectionChanged: on<ToolbarSelectionData>(ipcChannels.toolbarSelectionChanged),
  onLeftSidebarChanged: on<boolean>(ipcChannels.leftSidebarChanged),
  onDevtoolsChanged: on<boolean>(ipcChannels.devtoolsChanged),
  onThemeChanged: on(ipcChannels.themeChanged),
  onAgentPresenceChanged: on<AgentPresenceCursor[]>(ipcChannels.agentPresenceChanged),
  onFocusAddressBar: on(ipcChannels.focusAddressBar),
  repoConnectViaPicker: () => ipcRenderer.invoke(ipcChannels.repoConnectViaPicker),
  repoDisconnect: (id) => ipcRenderer.invoke(ipcChannels.repoDisconnect, { id }),
}

contextBridge.exposeInMainWorld('electronAPI', api)
