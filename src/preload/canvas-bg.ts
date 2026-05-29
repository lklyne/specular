import { contextBridge, ipcRenderer, webUtils } from 'electron'
import type {
  AnnotationBboxSubscription,
  AnnotationCreateRequest,
  AnnotationLiveBboxUpdate,
  CanvasBgElectronAPI,
  EdgeSide,
  LayoutUpdateData,
  SelectionOverlayPayload,
  ToolDefaultPatch,
} from '../shared/types'
import type { BindingId } from '../shared/bindings'
import type { CanvasGuidesPayload } from '../shared/canvas-guides'

function installSelectionOverlayBridge(): void {
  if (location.href !== 'about:blank') return

  const marquee = document.createElement('div')
  marquee.style.position = 'absolute'
  marquee.style.display = 'none'
  marquee.style.pointerEvents = 'none'
  marquee.style.boxSizing = 'border-box'
  document.body.appendChild(marquee)

  const applyOverlayStyle = (variant: SelectionOverlayPayload['variant'] = 'default') => {
    if (variant === 'region-select') {
      marquee.style.border = '1px solid rgba(232, 180, 184, 0.95)'
      marquee.style.background = 'rgba(232, 180, 184, 0.22)'
      return
    }
    marquee.style.border = '1px solid rgba(59, 130, 246, 0.9)'
    marquee.style.background = 'rgba(59, 130, 246, 0.12)'
  }

  ipcRenderer.on(
    'canvas-selection-overlay',
    (
      _event,
      overlay: SelectionOverlayPayload | null,
    ) => {
      if (!overlay) {
        marquee.style.display = 'none'
        return
      }

      applyOverlayStyle(overlay.variant)
      const { rect } = overlay
      marquee.style.display = 'block'
      marquee.style.left = `${rect.left}px`
      marquee.style.top = `${rect.top}px`
      marquee.style.width = `${rect.width}px`
      marquee.style.height = `${rect.height}px`
    },
  )
}

if (document.readyState === 'loading') {
  window.addEventListener('DOMContentLoaded', installSelectionOverlayBridge, {
    once: true,
  })
} else {
  installSelectionOverlayBridge()
}

const api: CanvasBgElectronAPI = {
  canvasZoom: (deltaY, mouseX, mouseY) =>
    ipcRenderer.send('canvas-zoom', { deltaY, mouseX, mouseY }),
  canvasPan: (deltaX, deltaY) => ipcRenderer.send('canvas-pan', { deltaX, deltaY }),
  canvasPanTo: (x, y) => ipcRenderer.send('canvas-pan-to', { x, y }),
  setSelectionOverlayRect: (overlay) => ipcRenderer.send('canvas-selection-overlay', overlay),
  onSelectionOverlayChanged: (callback) => {
    const handler = (
      _event: Electron.IpcRendererEvent,
      overlay: import('../shared/types').SelectionOverlayPayload | null,
    ) => callback(overlay)
    ipcRenderer.on('canvas-selection-overlay', handler)
    return () => ipcRenderer.removeListener('canvas-selection-overlay', handler)
  },
  canvasSelectInRect: (rect, modifiers) =>
    ipcRenderer.send('canvas-select-in-rect', { ...rect, modifiers }),
  canvasSelectInScreenRect: (rect, modifiers) =>
    ipcRenderer.send('canvas-select-in-screen-rect', { ...rect, modifiers }),
  canvasDeselect: (modifiers) => ipcRenderer.send('page-deselect', { modifiers }),
  canvasClickAt: (screenX, screenY, modifiers) =>
    ipcRenderer.send('canvas-click-at', { screenX, screenY, modifiers }),
  clearAnnotateHover: () => ipcRenderer.send('canvas-clear-annotate-hover'),
  selectPage: (pageId, modifiers) =>
    ipcRenderer.send('canvas-select-page', { pageId, modifiers }),
  selectBrowserTab: (pageId) => ipcRenderer.send('canvas-select-browser-tab', { pageId }),
  addBrowserPage: (presetIndex) => ipcRenderer.send('add-browser-page', presetIndex),
  navigatePage: (pageId, url) => ipcRenderer.send('canvas-navigate-page', { pageId, url }),
  goBackPage: (pageId) => ipcRenderer.send('canvas-back-page', { pageId }),
  goForwardPage: (pageId) => ipcRenderer.send('canvas-forward-page', { pageId }),
  reloadPage: (pageId) => ipcRenderer.send('canvas-reload-page', { pageId }),
  setPageCustom: (pageId) => ipcRenderer.send('canvas-set-page-custom', { pageId }),
  setBrowserSizeMode: (pageId, mode) => ipcRenderer.send('canvas-set-browser-size-mode', { pageId, mode }),
  updatePageBounds: (pageId, patch) => ipcRenderer.send('canvas-update-page-bounds', { pageId, patch }),
  placePendingEntity: (canvasX, canvasY) =>
    ipcRenderer.send('canvas-place-pending-entity', { canvasX, canvasY }),
  setTool: (tool) => ipcRenderer.send('toolbar-set-tool', tool),
  setToolDefault: (patch: ToolDefaultPatch) =>
    ipcRenderer.send('tool-defaults-set', patch),
  startDragPage: (pageId, selection) =>
    ipcRenderer.send('canvas-drag-page-start', { pageId, selection }),
  dragPage: (pageId, dx, dy, shiftKey = false) =>
    ipcRenderer.send('canvas-drag-page', { pageId, dx, dy, shiftKey }),
  endDragPage: () => ipcRenderer.send('canvas-drag-page-end'),
  dragCopyPage: (pageId, canvasX, canvasY) =>
    ipcRenderer.send('canvas-drag-copy-page', { pageId, canvasX, canvasY }),
  dragCopySelection: (canvasX, canvasY) =>
    ipcRenderer.send('canvas-drag-copy-selection', { canvasX, canvasY }),
  dragCopyGroup: (groupId, canvasX, canvasY) =>
    ipcRenderer.send('canvas-drag-copy-group', { groupId, canvasX, canvasY }),
  dragPreview: (dx, dy, shiftKey = false) =>
    ipcRenderer.send('canvas-drag-preview', { dx, dy, shiftKey }),
  setPagePreset: (pageId, index) => ipcRenderer.send('canvas-set-page-preset', { pageId, index }),
  setDeviceOrientation: (pageId, orientation) =>
    ipcRenderer.send('canvas-set-device-orientation', { pageId, orientation }),
  toggleDeviceShell: (pageId) =>
    ipcRenderer.send('canvas-toggle-device-shell', { pageId }),
  setFileDeviceOrientation: (fileId, orientation) =>
    ipcRenderer.send('canvas-set-file-device-orientation', { fileId, orientation }),
  toggleFileDeviceShell: (fileId) =>
    ipcRenderer.send('canvas-toggle-file-device-shell', { fileId }),
  renamePage: (pageId, name) => ipcRenderer.send('canvas-rename-page', { pageId, name }),
  duplicatePage: (pageId) => ipcRenderer.send('canvas-duplicate-page', { pageId }),
  toggleLinkedPage: (pageId) => ipcRenderer.send('canvas-toggle-linked-page', { pageId }),
  deletePage: (pageId) => ipcRenderer.send('canvas-delete-page', { pageId }),
  showPageContextMenu: (pageId) => ipcRenderer.send('canvas-show-page-context-menu', { pageId }),
  dropdownOpen: () => ipcRenderer.send('canvas-bg-dropdown-open'),
  dropdownClose: () => ipcRenderer.send('canvas-bg-dropdown-close'),
  copySelection: () => ipcRenderer.send('canvas-copy-selection'),
  pasteSelection: (canvasX, canvasY) =>
    ipcRenderer.send('canvas-paste-selection', { canvasX, canvasY }),
  deleteSelectedEntities: () => ipcRenderer.send('canvas-delete-selection'),
  tidySelectedEntities: () => ipcRenderer.send('canvas-tidy-selection'),
  reorderStack: (action, targetId) =>
    ipcRenderer.send('canvas-reorder-stack', { action, targetId }),
  createTextEntity: (canvasX: number, canvasY: number, text?: string, color?: string) =>
    ipcRenderer.send('canvas-create-text-entity', { canvasX, canvasY, text, color }),
  updateTextEntity: (id: string, patch: { text?: string; color?: string; textSize?: number; width?: number; height?: number; canvasX?: number; canvasY?: number; widthMode?: 'auto' | 'fixed' }) =>
    ipcRenderer.send('canvas-update-text-entity', { id, patch }),
  duplicateTextEntity: (id: string) =>
    ipcRenderer.send('canvas-duplicate-text-entity', { id }),
  deleteTextEntity: (id: string) =>
    ipcRenderer.send('canvas-delete-text-entity', { id }),
  updateFileEntity: (id: string, patch: { width?: number; height?: number; canvasX?: number; canvasY?: number }) =>
    ipcRenderer.send('canvas-update-file-entity', { id, patch }),
  deleteFileEntity: (id: string) =>
    ipcRenderer.send('canvas-delete-file-entity', { id }),
  duplicateFileEntity: (id: string) =>
    ipcRenderer.send('canvas-duplicate-file-entity', { id }),
  updateDrawingEntity: (id, patch) =>
    ipcRenderer.send('canvas-update-drawing-entity', { id, patch }),
  deleteDrawingEntity: (id: string) =>
    ipcRenderer.send('canvas-delete-drawing-entity', { id }),
  duplicateDrawingEntity: (id) =>
    ipcRenderer.send('canvas-duplicate-drawing-entity', { id }),
  updateShapeEntity: (id, patch) =>
    ipcRenderer.send('canvas-update-shape', { id, patch }),
  deleteShapeEntity: (id) =>
    ipcRenderer.send('canvas-delete-shape', { id }),
  duplicateShapeEntity: (id) =>
    ipcRenderer.send('canvas-duplicate-shape', { id }),
  placePendingShape: (canvasX, canvasY, dragRect) =>
    ipcRenderer.send('canvas-place-pending-entity', { canvasX, canvasY, dragRect: dragRect ?? null }),
  requestEntityEdit: (entityId) =>
    ipcRenderer.send('canvas-request-entity-edit', { entityId }),
  commitEntityEdit: () => ipcRenderer.send('canvas-commit-entity-edit'),
  cancelEntityEdit: () => ipcRenderer.send('canvas-cancel-entity-edit'),
  showFileInFinder: (filePath: string) =>
    ipcRenderer.send('canvas-show-file-in-finder', { filePath }),
  updateGroupEntity: (id: string, patch: { width?: number; height?: number; canvasX?: number; canvasY?: number; label?: string; color?: string }) =>
    ipcRenderer.send('canvas-update-group-entity', { id, patch }),
  duplicateGroup: (id: string) =>
    ipcRenderer.send('canvas-duplicate-group', { id }),
  deleteGroup: (id: string) =>
    ipcRenderer.send('canvas-delete-group', { id }),
  renameGroup: (groupId: string, name: string) =>
    ipcRenderer.send('canvas-rename-group', { groupId, name }),
  renameFileEntity: (entityId: string, name: string) =>
    ipcRenderer.send('canvas-rename-file-entity', { entityId, name }),
  renameTextEntity: (entityId: string, name: string) =>
    ipcRenderer.send('canvas-rename-text-entity', { entityId, name }),
  renameDrawingEntity: (entityId: string, name: string) =>
    ipcRenderer.send('canvas-rename-drawing-entity', { entityId, name }),
  dropFileBuffer: (buffer: Uint8Array, ext: string, canvasX: number, canvasY: number) =>
    ipcRenderer.send('canvas-drop-file-buffer', { buffer: Buffer.from(buffer), ext, canvasX, canvasY }),
  dropComponentFile: (file: File, canvasX: number, canvasY: number) => {
    const absolutePath = webUtils.getPathForFile(file)
    if (!absolutePath) return
    ipcRenderer.send('canvas-drop-component-path', { absolutePath, canvasX, canvasY })
  },
  selectEntity: (entityId, entityKind, modifiers) =>
    ipcRenderer.send('canvas-select-entity', { entityId, entityKind, modifiers }),
  selectGroup: (groupId: string) =>
    ipcRenderer.send('canvas-select-group', { groupId }),
  enterGroup: (groupId: string) =>
    ipcRenderer.send('canvas-enter-group', { groupId }),
  startDragGroup: (groupId: string) =>
    ipcRenderer.send('canvas-drag-group-start', { groupId }),
  dragGroup: (groupId: string, dx: number, dy: number, shiftKey = false) =>
    ipcRenderer.send('canvas-drag-group', { groupId, dx, dy, shiftKey }),
  endDragGroup: () => ipcRenderer.send('canvas-drag-group-end'),
  startDragEntity: (entityId: string, selection) =>
    ipcRenderer.send('canvas-drag-entity-start', { entityId, selection }),
  dragEntity: (entityId: string, dx: number, dy: number, shiftKey: boolean) =>
    ipcRenderer.send('canvas-drag-entity', { entityId, dx, dy, shiftKey }),
  endDragEntity: () => ipcRenderer.send('canvas-drag-entity-end'),
  beginResize: (entityId, entityKind, handle) =>
    ipcRenderer.send('canvas-resize-begin', { entityId, entityKind, handle }),
  endResize: () => ipcRenderer.send('canvas-resize-end'),
  beginMultiResize: () => ipcRenderer.send('canvas-multi-resize-begin'),
  endMultiResize: () => ipcRenderer.send('canvas-multi-resize-end'),
  commitRegionSelect: (canvasRect) => ipcRenderer.send('canvas-commit-region-select', canvasRect),
  commitCommentClickAt: (windowX, windowY) =>
    ipcRenderer.send('canvas-comment-click-at', { windowX, windowY }),
  createAnnotation: (request: AnnotationCreateRequest) =>
    ipcRenderer.send('canvas-create-annotation', request),
  createDrawing: (input) =>
    ipcRenderer.send('canvas-create-drawing', input),
  selectEntities: (entityIds: string[]) =>
    ipcRenderer.send('canvas-select-entities', entityIds),
  resizeMultiSelection: (entries) =>
    ipcRenderer.send('canvas-resize-multi-selection', { entries }),
  deleteSelection: () =>
    ipcRenderer.send('canvas-delete-selection'),
  moveAnnotation: (annotationId: string, dx: number, dy: number) =>
    ipcRenderer.send('canvas-move-annotation', { annotationId, dx, dy }),
  addAnnotationReply: (annotationId: string, text: string) =>
    ipcRenderer.send('right-details-panel-reply-annotation', { annotationId, text }),
  resolveAnnotation: (annotationId: string) =>
    ipcRenderer.send('right-details-panel-resolve-annotation', { annotationId }),
  deleteAnnotation: (annotationId: string) =>
    ipcRenderer.send('right-details-panel-delete-annotation', { annotationId }),
  openAnnotationThread: (annotationId: string) =>
    ipcRenderer.send('annotation-open-thread', { annotationId }),
  setCommentOverlayActive: (active: boolean) =>
    ipcRenderer.send('comment-overlay-set-active', active),
  onCaptureMode: (callback: (active: boolean) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, active: boolean) => callback(active)
    ipcRenderer.on('capture-mode', handler)
    return () => ipcRenderer.removeListener('capture-mode', handler)
  },
  onAnnotateElementSelected: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: Parameters<typeof callback>[0]) =>
      callback(data)
    ipcRenderer.on('annotate-element-selected', handler)
    return () => ipcRenderer.removeListener('annotate-element-selected', handler)
  },
  onRegionSelectCommitted: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: Parameters<typeof callback>[0]) =>
      callback(data)
    ipcRenderer.on('region-select-committed', handler)
    return () => ipcRenderer.removeListener('region-select-committed', handler)
  },
  onCommentCanvasPointCommitted: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: Parameters<typeof callback>[0]) =>
      callback(data)
    ipcRenderer.on('comment-canvas-point-committed', handler)
    return () => ipcRenderer.removeListener('comment-canvas-point-committed', handler)
  },
  setCommentToolPointerState: (state) =>
    ipcRenderer.send(
      'comment-tool-pointer-state',
      state
        ? {
            windowX: state.windowX,
            windowY: state.windowY,
            regionRect: state.regionRect,
          }
        : null,
    ),
  setAnnotationBboxSubscriptions: (
    pageId: string,
    subscriptions: AnnotationBboxSubscription[],
  ) =>
    ipcRenderer.send('comment-tool-bbox-subscriptions', { pageId, subscriptions }),
  onAnnotationLiveBbox: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, update: AnnotationLiveBboxUpdate) =>
      callback(update)
    ipcRenderer.on('annotation-live-bbox', handler)
    return () => ipcRenderer.removeListener('annotation-live-bbox', handler)
  },
  createRegionAnnotation: (canvasRect, text) =>
    ipcRenderer.send('canvas-create-region-annotation', { canvasRect, text }),
  onAnnotationThreadOpen: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: Parameters<typeof callback>[0]) =>
      callback(data)
    ipcRenderer.on('annotation-thread-open', handler)
    return () => ipcRenderer.removeListener('annotation-thread-open', handler)
  },
  beginEdgeDrag: (fromEntityId: string, fromSide: EdgeSide) =>
    ipcRenderer.send('canvas-edge-drag-begin', { fromEntityId, fromSide }),
  updateEdgeDragTarget: (targetEntityId: string | null, targetSide: EdgeSide | null) =>
    ipcRenderer.send('canvas-edge-drag-target-change', { targetEntityId, targetSide }),
  commitEdgeDrag: (fromEntityId: string, toEntityId: string, fromSide: EdgeSide, toSide: EdgeSide) =>
    ipcRenderer.send('canvas-edge-drag-commit', { fromEntityId, toEntityId, fromSide, toSide }),
  cancelEdgeDrag: () =>
    ipcRenderer.send('canvas-edge-drag-cancel'),
  commitEdgeEdit: (
    edgeId: string,
    movingEnd: 'from' | 'to',
    targetEntityId: string,
    targetSide: EdgeSide,
  ) =>
    ipcRenderer.send('canvas-edge-edit-commit', { edgeId, movingEnd, targetEntityId, targetSide }),
  discardEdgeEdit: (edgeId: string) =>
    ipcRenderer.send('canvas-edge-edit-discard', { edgeId }),
  createEdge: (fromEntityId: string, toEntityId: string, fromSide?: EdgeSide, toSide?: EdgeSide) =>
    ipcRenderer.send('canvas-create-edge', { fromEntityId, toEntityId, fromSide, toSide }),
  deleteEdge: (edgeId: string) =>
    ipcRenderer.send('canvas-delete-edge', { edgeId }),
  selectEdge: (edgeId: string | null) =>
    ipcRenderer.send('canvas-select-edge', { edgeId }),
  hoverPage: (pageId: string | null) =>
    ipcRenderer.send('canvas-hover-page', { pageId }),
  forwardWheelToPage: (pageId, payload) =>
    ipcRenderer.send('canvas-forward-wheel', { pageId, payload }),
  forwardPointerToPage: (pageId, payload) =>
    ipcRenderer.send('canvas-forward-pointer', { pageId, payload }),
  onPageCursorChange: (callback) => {
    const handler = (
      _event: Electron.IpcRendererEvent,
      data: { type: string | null },
    ) => callback(data)
    ipcRenderer.on('aboveview-cursor-update', handler)
    return () => ipcRenderer.removeListener('aboveview-cursor-update', handler)
  },
  setTextEditing: (active: boolean) =>
    ipcRenderer.send('canvas-set-text-editing', { active }),
  setAnnotationState: (hasOpenThread: boolean, hasPendingAnnotation: boolean) =>
    ipcRenderer.send('canvas-set-annotation-state', { hasOpenThread, hasPending: hasPendingAnnotation }),
  onBindingFire: (callback: (id: BindingId) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, id: BindingId) => callback(id)
    ipcRenderer.on('binding-fire', handler)
    return () => ipcRenderer.removeListener('binding-fire', handler)
  },
  onCanvasGuides: (callback: (payload: CanvasGuidesPayload) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, payload: CanvasGuidesPayload) =>
      callback(payload)
    ipcRenderer.on('canvas-guides', handler)
    return () => ipcRenderer.removeListener('canvas-guides', handler)
  },
  readNoteFile: (filePath: string) =>
    ipcRenderer.invoke('read-note-file', { filePath }),
  writeNoteFile: (filePath: string, content: string) =>
    ipcRenderer.invoke('write-note-file', { filePath, content }),
  renameNoteFile: (filePath: string, newName: string) =>
    ipcRenderer.invoke('rename-note-file', { filePath, newName }),
  morphTextFile: (entityId: string, direction: 'text-to-file' | 'file-to-text') =>
    ipcRenderer.invoke('canvas-morph-text-file', { entityId, direction }),
  getInitialData: () => ipcRenderer.invoke('get-canvas-layout-bootstrap'),
  repoConnect: (absolutePath: string) =>
    ipcRenderer.invoke('repo-connect', { absolutePath }),
  onLayoutUpdate: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: LayoutUpdateData) => callback(data)
    ipcRenderer.on('layout-update', handler)
    return () => ipcRenderer.removeListener('layout-update', handler)
  },
  onFixProgressUpdate: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: LayoutUpdateData['fixProgress']) =>
      callback(data)
    ipcRenderer.on('fix-progress-update', handler)
    return () => ipcRenderer.removeListener('fix-progress-update', handler)
  },
  onThemeChanged: (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: { isDark: boolean }) =>
      callback(data)
    ipcRenderer.on('theme-changed', handler)
    return () => ipcRenderer.removeListener('theme-changed', handler)
  },
}

contextBridge.exposeInMainWorld('electronAPI', api)
