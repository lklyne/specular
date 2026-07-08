import { contextBridge, ipcRenderer } from 'electron'
import type { AnnotationCreateRequest, DevtoolsPanelData } from '../shared/types'
import type { DevtoolsPanelElectronAPI } from '../shared/electron-api/right-details-panel'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

const api: DevtoolsPanelElectronAPI = {
  setTool: (tool) => ipcRenderer.send(ipcChannels.toolbarSetTool, tool),
  setTextEditing: (active) => ipcRenderer.send(ipcChannels.canvasSetTextEditing, { active }),
  selectPage: (pageId: string) => ipcRenderer.send(ipcChannels.rightDetailsPanelSelectPage, { pageId }),
  clearInspectSelection: () => ipcRenderer.send(ipcChannels.rightDetailsPanelClearInspectSelection),
  setInspectHoverNode: (pageId: string, nodeId: string | null) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelHoverNode, { pageId, nodeId }),
  setInspectSelectedNode: (pageId: string, nodeId: string | null) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelSelectNode, { pageId, nodeId }),
  editComponentProp: (pageId, payload) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelEditComponentProp, { pageId, ...payload }),
  editComponentToken: (pageId, payload) =>
      ipcRenderer.send(ipcChannels.rightDetailsPanelEditComponentToken, { pageId, ...payload }),
  createAnnotation: (request: AnnotationCreateRequest) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelCreateAnnotation, request),
  resolveAnnotation: (annotationId) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelResolveAnnotation, { annotationId }),
  deleteAnnotation: (annotationId) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelDeleteAnnotation, { annotationId }),
  openAnnotationThread: (annotationId) =>
    ipcRenderer.send(ipcChannels.annotationOpenThread, { annotationId }),
  triggerFixComments: (origin: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelTriggerFixComments, { origin }),
  fixSingleAnnotation: (annotationId: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelFixSingleAnnotation, { annotationId }),
  setAutoFix: (origin: string, enabled: boolean) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelSetAutoFix, { origin, enabled }),
  pickRepoForOrigin: (origin: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelPickRepoForOrigin, { origin }),
  removeOriginBinding: (origin: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelRemoveOriginBinding, { origin }),
  setFixConfig: (config: { model: string; permissions: string }) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelSetFixConfig, config),
  updateEntity: (kind, id, patch) =>
    ipcRenderer.send(ipcChannels.canvasUpdateEntity, { kind, id, patch }),
  duplicateTextEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateTextEntity, { id }),
  deleteTextEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteTextEntity, { id }),
  duplicateFileEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateFileEntity, { id }),
  deleteFileEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteFileEntity, { id }),
  setFilePreset: (fileId: string, presetIndex: number) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelSetFilePreset, { fileId, presetIndex }),
  setFileCustom: (fileId: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelSetFileCustom, { fileId }),
  setFileDeviceOrientation: (fileId: string, orientation: string) =>
    ipcRenderer.send(ipcChannels.canvasSetFileDeviceOrientation, { fileId, orientation }),
  toggleFileDeviceShell: (fileId: string) =>
    ipcRenderer.send(ipcChannels.canvasToggleFileDeviceShell, { fileId }),
  deleteDrawingEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteDrawingEntity, { id }),
  deleteShapeEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteShape, { id }),
  updateEdge: (id, patch) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelUpdateEdge, { id, patch }),
  deleteEdge: (id) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelDeleteEdge, { id }),
  setPagePreset: (pageId: string, presetIndex: number) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelSetPagePreset, { pageId, presetIndex }),
  setPageColorScheme: (pageId, colorScheme) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelSetPageColorScheme, { pageId, colorScheme }),
  setPageCustom: (pageId: string) =>
    ipcRenderer.send(ipcChannels.canvasSetPageCustom, { pageId }),
  setDeviceOrientation: (pageId: string, orientation: string) =>
    ipcRenderer.send(ipcChannels.canvasSetDeviceOrientation, { pageId, orientation }),
  toggleDeviceShell: (pageId: string) =>
    ipcRenderer.send(ipcChannels.canvasToggleDeviceShell, { pageId }),
  // Reuses the global focus channel — the panel only surfaces the selected
  // page, so "focus selection" and "focus this page" are the same action.
  focusSelection: () => ipcRenderer.send(ipcChannels.canvasFocusSelection),
  toggleSvgDeviceShell: (pageId: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelToggleSvgDeviceShell, { pageId }),
  duplicatePage: (pageId: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelDuplicatePage, { pageId }),
  deletePage: (pageId: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelDeletePage, { pageId }),
  openBrowserDevTools: () => ipcRenderer.send(ipcChannels.rightDetailsPanelOpenBrowserDevtools),
  closeBrowserDevTools: () => ipcRenderer.send(ipcChannels.rightDetailsPanelDismissBrowserDevtools),
  getInitialData: () => ipcRenderer.invoke(ipcChannels.getThemeBootstrap),
  onThemeChanged: on(ipcChannels.themeChanged),
  onPanelData: on<DevtoolsPanelData>(ipcChannels.rightDetailsPanelData),
}

contextBridge.exposeInMainWorld('electronAPI', api)
