import { contextBridge, ipcRenderer } from 'electron'
import type {
  AnnotationCreateRequest,
  DevtoolsPanelData,
  DevtoolsPanelElectronAPI,
  ThemeData,
} from '../shared/types'
import { on } from './ipc-helpers'

const api: DevtoolsPanelElectronAPI = {
  setTool: (tool) => ipcRenderer.send('toolbar-set-tool', tool),
  setTextEditing: (active) => ipcRenderer.send('canvas-set-text-editing', { active }),
  selectPage: (pageId: string) => ipcRenderer.send('right-details-panel-select-page', { pageId }),
  clearInspectSelection: () => ipcRenderer.send('right-details-panel-clear-inspect-selection'),
  setInspectHoverNode: (pageId: string, nodeId: string | null) =>
    ipcRenderer.send('right-details-panel-hover-node', { pageId, nodeId }),
  setInspectSelectedNode: (pageId: string, nodeId: string | null) =>
    ipcRenderer.send('right-details-panel-select-node', { pageId, nodeId }),
  editComponentProp: (pageId, payload) =>
    ipcRenderer.send('right-details-panel-edit-component-prop', { pageId, ...payload }),
  editComponentToken: (pageId, payload) =>
      ipcRenderer.send('right-details-panel-edit-component-token', { pageId, ...payload }),
  createAnnotation: (request: AnnotationCreateRequest) =>
    ipcRenderer.send('right-details-panel-create-annotation', request),
  resolveAnnotation: (annotationId) =>
    ipcRenderer.send('right-details-panel-resolve-annotation', { annotationId }),
  deleteAnnotation: (annotationId) =>
    ipcRenderer.send('right-details-panel-delete-annotation', { annotationId }),
  openAnnotationThread: (annotationId) =>
    ipcRenderer.send('annotation-open-thread', { annotationId }),
  triggerFixComments: (origin: string) =>
    ipcRenderer.send('right-details-panel-trigger-fix-comments', { origin }),
  fixSingleAnnotation: (annotationId: string) =>
    ipcRenderer.send('right-details-panel-fix-single-annotation', { annotationId }),
  setAutoFix: (origin: string, enabled: boolean) =>
    ipcRenderer.send('right-details-panel-set-auto-fix', { origin, enabled }),
  pickRepoForOrigin: (origin: string) =>
    ipcRenderer.send('right-details-panel-pick-repo-for-origin', { origin }),
  removeOriginBinding: (origin: string) =>
    ipcRenderer.send('right-details-panel-remove-origin-binding', { origin }),
  setFixConfig: (config: { model: string; permissions: string }) =>
    ipcRenderer.send('right-details-panel-set-fix-config', config),
  updateEntity: (kind, id, patch) =>
    ipcRenderer.send('canvas-update-entity', { kind, id, patch }),
  duplicateTextEntity: (id: string) =>
    ipcRenderer.send('canvas-duplicate-text-entity', { id }),
  deleteTextEntity: (id: string) =>
    ipcRenderer.send('canvas-delete-text-entity', { id }),
  duplicateFileEntity: (id: string) =>
    ipcRenderer.send('canvas-duplicate-file-entity', { id }),
  deleteFileEntity: (id: string) =>
    ipcRenderer.send('canvas-delete-file-entity', { id }),
  setFilePreset: (fileId: string, presetIndex: number) =>
    ipcRenderer.send('right-details-panel-set-file-preset', { fileId, presetIndex }),
  setFileCustom: (fileId: string) =>
    ipcRenderer.send('right-details-panel-set-file-custom', { fileId }),
  setFileDeviceOrientation: (fileId: string, orientation: string) =>
    ipcRenderer.send('right-details-panel-set-file-device-orientation', { fileId, orientation }),
  toggleFileDeviceShell: (fileId: string) =>
    ipcRenderer.send('right-details-panel-toggle-file-device-shell', { fileId }),
  deleteDrawingEntity: (id: string) =>
    ipcRenderer.send('canvas-delete-drawing-entity', { id }),
  deleteShapeEntity: (id: string) =>
    ipcRenderer.send('canvas-delete-shape', { id }),
  updateEdge: (id, patch) =>
    ipcRenderer.send('right-details-panel-update-edge', { id, patch }),
  deleteEdge: (id) =>
    ipcRenderer.send('right-details-panel-delete-edge', { id }),
  setPagePreset: (pageId: string, presetIndex: number) =>
    ipcRenderer.send('right-details-panel-set-page-preset', { pageId, presetIndex }),
  setPageCustom: (pageId: string) =>
    ipcRenderer.send('right-details-panel-set-page-custom', { pageId }),
  setDeviceOrientation: (pageId: string, orientation: string) =>
    ipcRenderer.send('right-details-panel-set-device-orientation', { pageId, orientation }),
  toggleDeviceShell: (pageId: string) =>
    ipcRenderer.send('right-details-panel-toggle-device-shell', { pageId }),
  // Reuses the global focus channel — the panel only surfaces the selected
  // page, so "focus selection" and "focus this page" are the same action.
  focusSelection: () => ipcRenderer.send('canvas-focus-selection'),
  toggleSvgDeviceShell: (pageId: string) =>
    ipcRenderer.send('right-details-panel-toggle-svg-device-shell', { pageId }),
  duplicatePage: (pageId: string) =>
    ipcRenderer.send('right-details-panel-duplicate-page', { pageId }),
  toggleLinkedPage: (pageId: string) =>
    ipcRenderer.send('right-details-panel-toggle-linked-page', { pageId }),
  deletePage: (pageId: string) =>
    ipcRenderer.send('right-details-panel-delete-page', { pageId }),
  openBrowserDevTools: () => ipcRenderer.send('right-details-panel-open-browser-devtools'),
  closeBrowserDevTools: () => ipcRenderer.send('right-details-panel-dismiss-browser-devtools'),
  getInitialData: () => ipcRenderer.invoke('get-theme-bootstrap'),
  onThemeChanged: on<ThemeData>('theme-changed'),
  onPanelData: on<DevtoolsPanelData>('right-details-panel-data'),
}

contextBridge.exposeInMainWorld('electronAPI', api)
