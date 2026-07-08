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
  setTool: (tool) => ipcRenderer.send(ipcChannels.toolbarSetTool, tool),
  reloadApp: () => ipcRenderer.send(ipcChannels.reloadApp),
  setThemeMode: (mode) => ipcRenderer.send(ipcChannels.setThemeMode, { mode }),
  getInitialData: () => ipcRenderer.invoke(ipcChannels.getThemeBootstrap),
  toggleLeftSidebar: () => ipcRenderer.send(ipcChannels.toggleLeftSidebar),
  toggleDevTools: () => ipcRenderer.send(ipcChannels.toggleDevtools),
  dropdownOpen: () => ipcRenderer.send(ipcChannels.toolbarDropdownOpen),
  dropdownClose: () => ipcRenderer.send(ipcChannels.toolbarDropdownClose),
  tooltipOpen: () => ipcRenderer.send(ipcChannels.toolbarTooltipOpen),
  tooltipClose: () => ipcRenderer.send(ipcChannels.toolbarTooltipClose),
  setTextEditing: (active) => ipcRenderer.send(ipcChannels.canvasSetTextEditing, { active }),
  onZoomChanged: on<number>(ipcChannels.zoomChanged),
  onSelectionChanged: on<ToolbarSelectionData>(ipcChannels.toolbarSelectionChanged),
  onLeftSidebarChanged: on<boolean>(ipcChannels.leftSidebarChanged),
  onDevtoolsChanged: on<boolean>(ipcChannels.devtoolsChanged),
  onThemeChanged: on(ipcChannels.themeChanged),
  onAgentPresenceChanged: on<AgentPresenceCursor[]>(ipcChannels.agentPresenceChanged),
  repoConnectViaPicker: () => ipcRenderer.invoke(ipcChannels.repoConnectViaPicker),
  repoDisconnect: (id) => ipcRenderer.invoke(ipcChannels.repoDisconnect, { id }),
}

contextBridge.exposeInMainWorld('electronAPI', api)
