import { contextBridge, ipcRenderer } from 'electron'
import type {
  CanvasEntityKind,
  LeftSidebarData,
  LeftSidebarElectronAPI,
  ThemeData,
} from '../shared/types'

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
  duplicateTab: (tabId) => ipcRenderer.send('canvas-duplicate-tab', { tabId }),
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
  setTabExpanded: (tabId, expanded) =>
    ipcRenderer.send('canvas-set-tab-expanded', { tabId, expanded }),
  setTextEditing: (active) => ipcRenderer.send('canvas-set-text-editing', { active }),
  toggleBrowserMode: () => ipcRenderer.send('toolbar-toggle-browser-mode'),
  getInitialData: () => ipcRenderer.invoke('get-left-sidebar-bootstrap'),
  onThemeChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: ThemeData) => callback(data)
    ipcRenderer.on('theme-changed', handler)
    return () => ipcRenderer.removeListener('theme-changed', handler)
  },
  onSidebarData: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: LeftSidebarData) => callback(data)
    ipcRenderer.on('left-sidebar-data', handler)
    return () => ipcRenderer.removeListener('left-sidebar-data', handler)
  },
}

contextBridge.exposeInMainWorld('electronAPI', api)
