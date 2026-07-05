import { contextBridge, ipcRenderer } from 'electron'
import type { CanvasEntityKind, LeftSidebarData } from '../shared/types'
import type { LeftSidebarElectronAPI } from '../shared/electron-api/left-sidebar'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: LeftSidebarElectronAPI = {
  revealPage: (pageId) => ipcRenderer.send(ipcChannels.canvasRevealPage, { pageId }),
  revealEntity: (entityId, entityKind) =>
    ipcRenderer.send(ipcChannels.canvasRevealEntity, { entityId, entityKind }),
  deleteEntity: (entityId, entityKind) =>
    ipcRenderer.send(ipcChannels.canvasDeleteEntity, { entityId, entityKind }),
  revealGroup: (groupId) => ipcRenderer.send(ipcChannels.canvasRevealGroup, { groupId }),
  ungroupGroup: (groupId) => ipcRenderer.send(ipcChannels.canvasUngroupGroup, { groupId }),
  selectTab: (tabId) => ipcRenderer.send(ipcChannels.canvasSelectTab, { tabId }),
  createTab: () => ipcRenderer.send(ipcChannels.canvasCreateTab),
  renameTab: (tabId, name) => ipcRenderer.send(ipcChannels.canvasRenameTab, { tabId, name }),
  renamePage: (pageId, name) => ipcRenderer.send(ipcChannels.canvasRenamePage, { pageId, name }),
  renameGroup: (groupId, name) => ipcRenderer.send(ipcChannels.canvasRenameGroup, { groupId, name }),
  renameFileEntity: (entityId, name) =>
    ipcRenderer.send(ipcChannels.canvasRenameFileEntity, { entityId, name }),
  renameTextEntity: (entityId, name) =>
    ipcRenderer.send(ipcChannels.canvasRenameTextEntity, { entityId, name }),
  renameDrawingEntity: (entityId, name) =>
    ipcRenderer.send(ipcChannels.canvasRenameDrawingEntity, { entityId, name }),
  deleteTab: (tabId) => ipcRenderer.send(ipcChannels.canvasDeleteTab, { tabId }),
  reorderTab: (tabId, toIndex) => ipcRenderer.send(ipcChannels.canvasReorderTab, { tabId, toIndex }),
  reorderSidebarItem: (section, draggedId, anchorId, position, parentId) =>
    ipcRenderer.send(ipcChannels.canvasReorderSidebarItem, {
      section,
      draggedId,
      anchorId,
      position,
      parentId,
    }),
  deletePage: (pageId) => ipcRenderer.send(ipcChannels.canvasDeletePage, { pageId }),
  setTextEditing: (active) => ipcRenderer.send(ipcChannels.canvasSetTextEditing, { active }),
  getInitialData: () => ipcRenderer.invoke(ipcChannels.getLeftSidebarBootstrap),
  onThemeChanged: on(ipcChannels.themeChanged),
  onSidebarData: on<LeftSidebarData>(ipcChannels.leftSidebarData),
}

contextBridge.exposeInMainWorld('electronAPI', api)
