import { contextBridge, ipcRenderer, webUtils } from 'electron'
import type { AnnotationBboxSubscription, AnnotationCreateRequest, AnnotationElementSelectionPayload, AnnotationLiveBboxUpdate, BatchLayoutMode, EdgeSide, LayoutUpdateData, SelectionOverlayPayload, ToolDefaultPatch, ViewportNudge, WorkspaceBounds } from '../shared/types'
import type { CanvasBgElectronAPI } from '../shared/electron-api/canvas-bg'
import type { BindingId } from '../shared/bindings'
import type { CancelReason } from '../shared/interaction-types'
import type { CanvasGuidesPayload } from '../shared/canvas-guides'
import { ipcChannels } from '../shared/ipc-contract'
import { on } from './ipc-helpers'

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
    ipcChannels.canvasSelectionOverlay,
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
    ipcRenderer.send(ipcChannels.canvasZoom, { deltaY, mouseX, mouseY }),
  canvasPan: (deltaX, deltaY) => ipcRenderer.send(ipcChannels.canvasPan, { deltaX, deltaY }),
  setSelectionOverlayRect: (overlay) => ipcRenderer.send(ipcChannels.canvasSelectionOverlay, overlay),
  onSelectionOverlayChanged: on<SelectionOverlayPayload | null>(ipcChannels.canvasSelectionOverlay),
  canvasSelectInRect: (rect, modifiers) =>
    ipcRenderer.send(ipcChannels.canvasSelectInRect, { ...rect, modifiers }),
  canvasDeselect: (modifiers) => ipcRenderer.send(ipcChannels.pageDeselect, { modifiers }),
  focusSelection: () => ipcRenderer.send(ipcChannels.canvasFocusSelection),
  restoreFocusCamera: () => ipcRenderer.send(ipcChannels.canvasRestoreFocusCamera),
  setFocusPresentationMode: (mode) =>
    ipcRenderer.send(ipcChannels.canvasSetFocusPresentationMode, mode),
  setFocusAnnotationsVisible: (visible) =>
    ipcRenderer.send(ipcChannels.canvasSetFocusAnnotationsVisible, visible),
  clearAnnotateHover: () => ipcRenderer.send(ipcChannels.canvasClearAnnotateHover),
  selectPage: (pageId, modifiers) =>
    ipcRenderer.send(ipcChannels.canvasSelectPage, { pageId, modifiers }),
  navigatePage: (pageId, url) => ipcRenderer.send(ipcChannels.canvasNavigatePage, { pageId, url }),
  goBackPage: (pageId) => ipcRenderer.send(ipcChannels.canvasBackPage, { pageId }),
  goForwardPage: (pageId) => ipcRenderer.send(ipcChannels.canvasForwardPage, { pageId }),
  reloadPage: (pageId) => ipcRenderer.send(ipcChannels.canvasReloadPage, { pageId }),
  setPageCustom: (pageId) => ipcRenderer.send(ipcChannels.canvasSetPageCustom, { pageId }),
  setPageColorScheme: (pageId, colorScheme) =>
    ipcRenderer.send(ipcChannels.canvasSetPageColorScheme, { pageId, colorScheme }),
  updatePageBounds: (pageId, patch) => ipcRenderer.send(ipcChannels.canvasUpdatePageBounds, { pageId, patch }),
  placePendingEntity: (canvasX, canvasY) =>
    ipcRenderer.send(ipcChannels.canvasPlacePendingEntity, { canvasX, canvasY }),
  setTool: (tool) => ipcRenderer.send(ipcChannels.toolbarSetTool, tool),
  setToolDefault: (patch: ToolDefaultPatch) =>
    ipcRenderer.send(ipcChannels.toolDefaultsSet, patch),
  startDragPage: (pageId, selection) =>
    ipcRenderer.send(ipcChannels.canvasDragPageStart, { pageId, selection }),
  dragPage: (pageId, dx, dy, shiftKey = false) =>
    ipcRenderer.send(ipcChannels.canvasDragPage, { pageId, dx, dy, shiftKey }),
  endDragPage: () => ipcRenderer.send(ipcChannels.canvasDragPageEnd),
  dragCopySelection: (canvasX, canvasY) =>
    ipcRenderer.send(ipcChannels.canvasDragCopySelection, { canvasX, canvasY }),
  dragCopyGroup: (groupId, canvasX, canvasY) =>
    ipcRenderer.send(ipcChannels.canvasDragCopyGroup, { groupId, canvasX, canvasY }),
  dragPreview: (dx, dy, shiftKey = false) =>
    ipcRenderer.send(ipcChannels.canvasDragPreview, { dx, dy, shiftKey }),
  setPagePreset: (pageId, index) => ipcRenderer.send(ipcChannels.canvasSetPagePreset, { pageId, index }),
  setDeviceOrientation: (pageId, orientation) =>
    ipcRenderer.send(ipcChannels.canvasSetDeviceOrientation, { pageId, orientation }),
  toggleDeviceShell: (pageId) =>
    ipcRenderer.send(ipcChannels.canvasToggleDeviceShell, { pageId }),
  setFileDeviceOrientation: (fileId, orientation) =>
    ipcRenderer.send(ipcChannels.canvasSetFileDeviceOrientation, { fileId, orientation }),
  toggleFileDeviceShell: (fileId) =>
    ipcRenderer.send(ipcChannels.canvasToggleFileDeviceShell, { fileId }),
  renamePage: (pageId, name) => ipcRenderer.send(ipcChannels.canvasRenamePage, { pageId, name }),
  duplicatePage: (pageId) => ipcRenderer.send(ipcChannels.canvasDuplicatePage, { pageId }),
  toggleSyncSelection: () => ipcRenderer.send(ipcChannels.canvasToggleSyncSelection),
  unsyncPage: (pageId: string) => ipcRenderer.send(ipcChannels.canvasUnsyncPage, pageId),
  deletePage: (pageId) => ipcRenderer.send(ipcChannels.canvasDeletePage, { pageId }),
  showPageContextMenu: (pageId) => ipcRenderer.send(ipcChannels.canvasShowPageContextMenu, { pageId }),
  dropdownOpen: () => ipcRenderer.send(ipcChannels.canvasBgDropdownOpen),
  dropdownClose: () => ipcRenderer.send(ipcChannels.canvasBgDropdownClose),
  copySelection: () => ipcRenderer.send(ipcChannels.canvasCopySelection),
  pasteSelection: (canvasX, canvasY) =>
    ipcRenderer.send(ipcChannels.canvasPasteSelection, { canvasX, canvasY }),
  deleteSelectedEntities: () => ipcRenderer.send(ipcChannels.canvasDeleteSelection),
  reorderStack: (action, targetId) =>
    ipcRenderer.send(ipcChannels.canvasReorderStack, { action, targetId }),
  updateEntity: (kind, id, patch) =>
    ipcRenderer.send(ipcChannels.canvasUpdateEntity, { kind, id, patch }),
  duplicateTextEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateTextEntity, { id }),
  deleteTextEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteTextEntity, { id }),
  deleteFileEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteFileEntity, { id }),
  duplicateFileEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateFileEntity, { id }),
  deleteDrawingEntity: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteDrawingEntity, { id }),
  duplicateDrawingEntity: (id) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateDrawingEntity, { id }),
  deleteShapeEntity: (id) =>
    ipcRenderer.send(ipcChannels.canvasDeleteShape, { id }),
  duplicateShapeEntity: (id) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateShape, { id }),
  placePendingShape: (canvasX, canvasY, dragRect) =>
    ipcRenderer.send(ipcChannels.canvasPlacePendingEntity, { canvasX, canvasY, dragRect: dragRect ?? null }),
  requestEntityEdit: (entityId) =>
    ipcRenderer.send(ipcChannels.canvasRequestEntityEdit, { entityId }),
  commitEntityEdit: () => ipcRenderer.send(ipcChannels.canvasCommitEntityEdit),
  cancelEntityEdit: () => ipcRenderer.send(ipcChannels.canvasCancelEntityEdit),
  showFileInFinder: (filePath: string) =>
    ipcRenderer.send(ipcChannels.canvasShowFileInFinder, { filePath }),
  copyFileAsPng: (filePath: string) =>
    ipcRenderer.send(ipcChannels.canvasCopyFileAsPng, { filePath }),
  duplicateGroup: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDuplicateGroup, { id }),
  deleteGroup: (id: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteGroup, { id }),
  renameGroup: (groupId: string, name: string) =>
    ipcRenderer.send(ipcChannels.canvasRenameGroup, { groupId, name }),
  renameFileEntity: (entityId: string, name: string) =>
    ipcRenderer.send(ipcChannels.canvasRenameFileEntity, { entityId, name }),
  renameTextEntity: (entityId: string, name: string) =>
    ipcRenderer.send(ipcChannels.canvasRenameTextEntity, { entityId, name }),
  renameDrawingEntity: (entityId: string, name: string) =>
    ipcRenderer.send(ipcChannels.canvasRenameDrawingEntity, { entityId, name }),
  dropFileBuffer: (buffer: Uint8Array, ext: string, canvasX: number, canvasY: number) =>
    ipcRenderer.send(ipcChannels.canvasDropFileBuffer, { buffer: Buffer.from(buffer), ext, canvasX, canvasY }),
  dropComponentFile: (file: File, canvasX: number, canvasY: number) => {
    const absolutePath = webUtils.getPathForFile(file)
    if (!absolutePath) return
    ipcRenderer.send(ipcChannels.canvasDropComponentPath, { absolutePath, canvasX, canvasY })
  },
  selectEntity: (entityId, entityKind, modifiers) =>
    ipcRenderer.send(ipcChannels.canvasSelectEntity, { entityId, entityKind, modifiers }),
  selectGroup: (groupId: string) =>
    ipcRenderer.send(ipcChannels.canvasSelectGroup, { groupId }),
  enterGroup: (groupId: string) =>
    ipcRenderer.send(ipcChannels.canvasEnterGroup, { groupId }),
  enterPageInteractive: (pageId: string) =>
    ipcRenderer.send(ipcChannels.canvasEnterPageInteractive, { pageId }),
  startDragGroup: (groupId: string) =>
    ipcRenderer.send(ipcChannels.canvasDragGroupStart, { groupId }),
  dragGroup: (groupId: string, dx: number, dy: number, shiftKey = false) =>
    ipcRenderer.send(ipcChannels.canvasDragGroup, { groupId, dx, dy, shiftKey }),
  endDragGroup: () => ipcRenderer.send(ipcChannels.canvasDragGroupEnd),
  startDragEntity: (entityId: string, selection) =>
    ipcRenderer.send(ipcChannels.canvasDragEntityStart, { entityId, selection }),
  dragEntity: (entityId: string, dx: number, dy: number, shiftKey: boolean) =>
    ipcRenderer.send(ipcChannels.canvasDragEntity, { entityId, dx, dy, shiftKey }),
  endDragEntity: () => ipcRenderer.send(ipcChannels.canvasDragEntityEnd),
  beginResize: (entityId, entityKind, handle) =>
    ipcRenderer.send(ipcChannels.canvasResizeBegin, { entityId, entityKind, handle }),
  endResize: () => ipcRenderer.send(ipcChannels.canvasResizeEnd),
  beginMultiResize: () => ipcRenderer.send(ipcChannels.canvasMultiResizeBegin),
  endMultiResize: () => ipcRenderer.send(ipcChannels.canvasMultiResizeEnd),
  arrangeSelection: (mode: BatchLayoutMode) =>
    ipcRenderer.send(ipcChannels.canvasArrangeSelection, mode),
  beginReorderDrag: (movingId: string) =>
    ipcRenderer.send(ipcChannels.canvasReorderStart, { movingId }),
  reorderDragMove: (canvasX: number, canvasY: number) =>
    ipcRenderer.send(ipcChannels.canvasReorderMove, { canvasX, canvasY }),
  reorderDragCommit: () => ipcRenderer.send(ipcChannels.canvasReorderCommit),
  reorderDragCancel: (reason?: CancelReason) =>
    ipcRenderer.send(ipcChannels.canvasReorderCancel, { reason }),
  commitRegionSelect: (canvasRect) => ipcRenderer.send(ipcChannels.canvasCommitRegionSelect, canvasRect),
  commitCommentClickAt: (windowX, windowY) =>
    ipcRenderer.send(ipcChannels.canvasCommentClickAt, { windowX, windowY }),
  createAnnotation: (request: AnnotationCreateRequest) =>
    ipcRenderer.send(ipcChannels.canvasCreateAnnotation, request),
  createDrawing: (input) =>
    ipcRenderer.send(ipcChannels.canvasCreateDrawing, input),
  selectEntities: (entityIds: string[]) =>
    ipcRenderer.send(ipcChannels.canvasSelectEntities, entityIds),
  resizeMultiSelection: (entries) =>
    ipcRenderer.send(ipcChannels.canvasResizeMultiSelection, { entries }),
  addAnnotationReply: (annotationId: string, text: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelReplyAnnotation, { annotationId, text }),
  resolveAnnotation: (annotationId: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelResolveAnnotation, { annotationId }),
  deleteAnnotation: (annotationId: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelDeleteAnnotation, { annotationId }),
  fixSingleAnnotation: (annotationId: string) =>
    ipcRenderer.send(ipcChannels.rightDetailsPanelFixSingleAnnotation, { annotationId }),
  openAnnotationThread: (annotationId: string) =>
    ipcRenderer.send(ipcChannels.annotationOpenThread, { annotationId }),
  setCommentOverlayActive: (active: boolean) =>
    ipcRenderer.send(ipcChannels.commentOverlaySetActive, active),
  onCaptureMode: on<boolean>(ipcChannels.captureMode),
  onAnnotateElementSelected: on<AnnotationElementSelectionPayload>(ipcChannels.annotateElementSelected),
  onRegionSelectCommitted: on<{ canvasRect: WorkspaceBounds }>(ipcChannels.regionSelectCommitted),
  onCommentCanvasPointCommitted: on<{ canvasX: number; canvasY: number }>(
    ipcChannels.commentCanvasPointCommitted,
  ),
  setCommentToolPointerState: (state) =>
    ipcRenderer.send(
      ipcChannels.commentToolPointerState,
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
    ipcRenderer.send(ipcChannels.commentToolBboxSubscriptions, { pageId, subscriptions }),
  onAnnotationLiveBbox: on<AnnotationLiveBboxUpdate>(ipcChannels.annotationLiveBbox),
  createRegionAnnotation: (canvasRect, text) =>
    ipcRenderer.send(ipcChannels.canvasCreateRegionAnnotation, { canvasRect, text }),
  onAnnotationThreadOpen: on<{ annotationId: string }>(ipcChannels.annotationThreadOpen),
  beginEdgeDrag: (fromEntityId: string, fromSide: EdgeSide) =>
    ipcRenderer.send(ipcChannels.canvasEdgeDragBegin, { fromEntityId, fromSide }),
  updateEdgeDragTarget: (targetEntityId: string | null, targetSide: EdgeSide | null) =>
    ipcRenderer.send(ipcChannels.canvasEdgeDragTargetChange, { targetEntityId, targetSide }),
  commitEdgeDrag: (fromEntityId: string, toEntityId: string, fromSide: EdgeSide, toSide: EdgeSide) =>
    ipcRenderer.send(ipcChannels.canvasEdgeDragCommit, { fromEntityId, toEntityId, fromSide, toSide }),
  cancelEdgeDrag: () =>
    ipcRenderer.send(ipcChannels.canvasEdgeDragCancel),
  commitEdgeEdit: (
    edgeId: string,
    movingEnd: 'from' | 'to',
    targetEntityId: string,
    targetSide: EdgeSide,
  ) =>
    ipcRenderer.send(ipcChannels.canvasEdgeEditCommit, { edgeId, movingEnd, targetEntityId, targetSide }),
  discardEdgeEdit: (edgeId: string) =>
    ipcRenderer.send(ipcChannels.canvasEdgeEditDiscard, { edgeId }),
  deleteEdge: (edgeId: string) =>
    ipcRenderer.send(ipcChannels.canvasDeleteEdge, { edgeId }),
  updateEdge: (edgeId, patch) =>
    ipcRenderer.send(ipcChannels.canvasUpdateEdge, { edgeId, patch }),
  selectEdge: (edgeId: string | null) =>
    ipcRenderer.send(ipcChannels.canvasSelectEdge, { edgeId }),
  hoverPage: (pageId: string | null) =>
    ipcRenderer.send(ipcChannels.canvasHoverPage, { pageId }),
  forwardWheelToPage: (pageId, payload) =>
    ipcRenderer.send(ipcChannels.canvasForwardWheel, { pageId, payload }),
  forwardPointerToPage: (pageId, payload) =>
    ipcRenderer.send(ipcChannels.canvasForwardPointer, { pageId, payload }),
  onPageCursorChange: on<{ type: string | null }>(ipcChannels.aboveviewCursorUpdate),
  setTextEditing: (active: boolean) =>
    ipcRenderer.send(ipcChannels.canvasSetTextEditing, { active }),
  setAnnotationState: (hasOpenThread: boolean, hasPendingAnnotation: boolean) =>
    ipcRenderer.send(ipcChannels.canvasSetAnnotationState, { hasOpenThread, hasPending: hasPendingAnnotation }),
  onBindingFire: on<BindingId>(ipcChannels.bindingFire),
  onCanvasGuides: on<CanvasGuidesPayload>(ipcChannels.canvasGuides),
  writeNoteFile: (filePath: string, content: string) =>
    ipcRenderer.invoke(ipcChannels.writeNoteFile, { filePath, content }),
  applyNoteContent: (entityId: string, content: string) =>
    ipcRenderer.invoke(ipcChannels.applyNoteContent, { entityId, content }),
  getInitialData: () => ipcRenderer.invoke(ipcChannels.getCanvasLayoutBootstrap),
  repoConnect: (absolutePath: string) =>
    ipcRenderer.invoke(ipcChannels.repoConnect, { absolutePath }),
  onLayoutUpdate: on(ipcChannels.layoutUpdate),
  onViewportNudge: on<ViewportNudge>(ipcChannels.viewportNudge),
  onFixProgressUpdate: on<LayoutUpdateData['fixProgress']>(ipcChannels.fixProgressUpdate),
  onThemeChanged: on(ipcChannels.themeChanged),
}

contextBridge.exposeInMainWorld('electronAPI', api)
