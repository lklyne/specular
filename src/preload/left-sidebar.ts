import { contextBridge, ipcRenderer } from 'electron'
import type { CanvasEntityKind, LeftSidebarData } from '../shared/types'
import type { LeftSidebarElectronAPI } from '../shared/electron-api/left-sidebar'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: LeftSidebarElectronAPI = {
  revealPage: (pageId, selectionIds, mode) =>
    ipcRenderer.send(ipcChannels.canvasRevealPage, { pageId, selectionIds, mode }),
  openAnnotationThread: (annotationId) =>
    ipcRenderer.send(ipcChannels.annotationOpenThread, { annotationId }),
  revealEntity: (entityId, entityKind, selectionIds, mode) =>
    ipcRenderer.send(ipcChannels.canvasRevealEntity, { entityId, entityKind, selectionIds, mode }),
  deleteEntity: (entityId, entityKind) =>
    ipcRenderer.send(ipcChannels.canvasDeleteEntity, { entityId, entityKind }),
  deleteSelection: () => ipcRenderer.send(ipcChannels.canvasDeleteSelection),
  revealGroup: (groupId) => ipcRenderer.send(ipcChannels.canvasRevealGroup, { groupId }),
  ungroupGroup: (groupId) => ipcRenderer.send(ipcChannels.canvasUngroupGroup, { groupId }),
  selectTab: (tabId) => ipcRenderer.send(ipcChannels.canvasSelectTab, { tabId }),
  createTab: () => ipcRenderer.send(ipcChannels.canvasCreateTab),
  renameTab: (tabId, name) => ipcRenderer.invoke(ipcChannels.canvasRenameTab, { tabId, name }),
  renamePage: (pageId, name) => ipcRenderer.invoke(ipcChannels.canvasRenamePage, { pageId, name }),
  renameGroup: (groupId, name) => ipcRenderer.invoke(ipcChannels.canvasRenameGroup, { groupId, name }),
  renameFileEntity: (entityId, name) =>
    ipcRenderer.invoke(ipcChannels.canvasRenameFileEntity, { entityId, name }),
  renameTextEntity: (entityId, name) =>
    ipcRenderer.invoke(ipcChannels.canvasRenameTextEntity, { entityId, name }),
  renameDrawingEntity: (entityId, name) =>
    ipcRenderer.invoke(ipcChannels.canvasRenameDrawingEntity, { entityId, name }),
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
