import { contextBridge, ipcRenderer } from 'electron'
import type {
  CanvasEntityKind,
  LeftSidebarData,
  LeftSidebarElectronAPI,
} from '../shared/types'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: LeftSidebarElectronAPI = {
  revealPage: (pageId) => ipcRenderer.send('canvas-reveal-page', { pageId }),
  revealEntity: (entityId, entityKind) =>
    ipcRenderer.send('canvas-reveal-entity', { entityId, entityKind }),
  deleteEntity: (entityId, entityKind) =>
    ipcRenderer.send('canvas-delete-entity', { entityId, entityKind }),
  revealGroup: (groupId) => ipcRenderer.send('canvas-reveal-group', { groupId }),
  ungroupGroup: (groupId) => ipcRenderer.send('canvas-ungroup-group', { groupId }),
  selectTab: (tabId) => ipcRenderer.send('canvas-select-tab', { tabId }),
  createTab: () => ipcRenderer.send('canvas-create-tab'),
  renameTab: (tabId, name) => ipcRenderer.send('canvas-rename-tab', { tabId, name }),
  renamePage: (pageId, name) => ipcRenderer.send('canvas-rename-page', { pageId, name }),
  renameGroup: (groupId, name) => ipcRenderer.send('canvas-rename-group', { groupId, name }),
  renameFileEntity: (entityId, name) =>
    ipcRenderer.send('canvas-rename-file-entity', { entityId, name }),
  renameTextEntity: (entityId, name) =>
    ipcRenderer.send('canvas-rename-text-entity', { entityId, name }),
  renameDrawingEntity: (entityId, name) =>
    ipcRenderer.send('canvas-rename-drawing-entity', { entityId, name }),
  deleteTab: (tabId) => ipcRenderer.send('canvas-delete-tab', { tabId }),
  reorderTab: (tabId, toIndex) => ipcRenderer.send('canvas-reorder-tab', { tabId, toIndex }),
  reorderSidebarItem: (section, draggedId, anchorId, position, parentId) =>
    ipcRenderer.send('canvas-reorder-sidebar-item', {
      section,
      draggedId,
      anchorId,
      position,
      parentId,
    }),
  deletePage: (pageId) => ipcRenderer.send('canvas-delete-page', { pageId }),
  setTextEditing: (active) => ipcRenderer.send('canvas-set-text-editing', { active }),
  getInitialData: () => ipcRenderer.invoke('get-left-sidebar-bootstrap'),
  onThemeChanged: on(ipcChannels.themeChanged),
  onSidebarData: on<LeftSidebarData>('left-sidebar-data'),
}

contextBridge.exposeInMainWorld('electronAPI', api)
