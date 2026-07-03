import type { BindingId } from '../bindings'
import type { CanvasGuidesPayload } from '../canvas-guides'
import type { CancelReason } from '../interaction-types'
import type { ResizeHandle } from '../resize-accumulator'
import type { Tool } from '../tool'
import type {
  AnnotationBboxSubscription,
  AnnotationCreateRequest,
  AnnotationDrawingStroke,
  AnnotationElementSelectionPayload,
  AnnotationLiveBboxUpdate,
  CanvasDragStartSelection,
  CanvasEntityKind,
  CanvasLayoutBootstrapData,
  EdgeSide,
  FocusPresentationMode,
  ForwardPointerPayload,
  ForwardWheelPayload,
  LayoutUpdateData,
  SelectionModifiers,
  SelectionOverlayPayload,
  ShapeKind,
  TextWidthMode,
  ThemeData,
  ViewportNudge,
  WorkspaceBounds,
} from '../types'

export interface CanvasBgElectronAPI {
  canvasZoom: (deltaY: number, mouseX: number, mouseY: number) => void
  canvasPan: (deltaX: number, deltaY: number) => void
  /** Subscribe to main's canvas-selection-overlay broadcast (marquee rect). */
  onSelectionOverlayChanged: (
    callback: (overlay: SelectionOverlayPayload | null) => void,
  ) => () => void
  setSelectionOverlayRect: (
    overlay: SelectionOverlayPayload | null,
  ) => void
  canvasSelectInRect: (rect: WorkspaceBounds, modifiers?: SelectionModifiers) => void
  canvasDeselect: (modifiers?: SelectionModifiers) => void
  focusSelection: () => void
  restoreFocusCamera: () => void
  setFocusPresentationMode: (mode: FocusPresentationMode) => void
  setFocusAnnotationsVisible: (visible: boolean) => void
  clearAnnotateHover: () => void
  selectPage: (pageId: string, modifiers?: SelectionModifiers) => void
  navigatePage: (pageId: string, url: string) => void
  goBackPage: (pageId: string) => void
  goForwardPage: (pageId: string) => void
  reloadPage: (pageId: string) => void
  setPageCustom: (pageId: string) => void
  updatePageBounds: (pageId: string, patch: { width?: number; height?: number; canvasX?: number; canvasY?: number }) => void
  placePendingEntity: (canvasX: number, canvasY: number) => void
  setTool: (tool: Tool) => void
  setToolDefault: (patch: import('../tool-defaults').ToolDefaultPatch) => void
  startDragPage: (pageId: string, selection?: CanvasDragStartSelection) => void
  dragPage: (pageId: string, dx: number, dy: number, shiftKey?: boolean) => void
  endDragPage: () => void
  dragCopySelection: (canvasX: number, canvasY: number) => void
  dragCopyGroup: (groupId: string, canvasX: number, canvasY: number) => void
  dragPreview: (dx: number, dy: number, shiftKey?: boolean) => void
  setPagePreset: (pageId: string, index: number) => void
  setDeviceOrientation: (pageId: string, orientation: string) => void
  toggleDeviceShell: (pageId: string) => void
  setFileDeviceOrientation: (fileId: string, orientation: string) => void
  toggleFileDeviceShell: (fileId: string) => void
  renamePage: (pageId: string, name: string) => void
  duplicatePage: (pageId: string) => void
  toggleLinkedPage: (pageId: string) => void
  deletePage: (pageId: string) => void
  showPageContextMenu: (pageId: string) => void
  dropdownOpen: () => void
  dropdownClose: () => void
  copySelection: () => void
  pasteSelection: (canvasX: number, canvasY: number) => void
  deleteSelectedEntities: () => void
  reorderStack: (
    action: 'bring-forward' | 'send-backward' | 'bring-to-front' | 'send-to-back',
    targetId?: string,
  ) => void
  updateTextEntity: (id: string, patch: { text?: string; color?: string; textSize?: number; width?: number; height?: number; canvasX?: number; canvasY?: number; widthMode?: TextWidthMode }) => void
  duplicateTextEntity: (id: string) => void
  deleteTextEntity: (id: string) => void
  updateFileEntity: (id: string, patch: { width?: number; height?: number; canvasX?: number; canvasY?: number }) => void
  deleteFileEntity: (id: string) => void
  duplicateFileEntity: (id: string) => void
  updateDrawingEntity: (id: string, patch: { width?: number; height?: number; canvasX?: number; canvasY?: number; strokes?: AnnotationDrawingStroke[] }) => void
  deleteDrawingEntity: (id: string) => void
  duplicateDrawingEntity: (id: string) => void
  updateShapeEntity: (id: string, patch: { shapeKind?: ShapeKind; text?: string; color?: string; strokeWidth?: number; textSize?: number; theme?: string; width?: number; height?: number; canvasX?: number; canvasY?: number }) => void
  deleteShapeEntity: (id: string) => void
  duplicateShapeEntity: (id: string) => void
  placePendingShape: (
    canvasX: number,
    canvasY: number,
    dragRect?: { x: number; y: number; width: number; height: number } | null,
  ) => void
  /** Enter inline-edit mode on an entity (sticky, shape, group, etc.). */
  requestEntityEdit: (entityId: string) => void
  /** Commit the active inline edit (renderers fire on blur). */
  commitEntityEdit: () => void
  /** Cancel the active inline edit (renderers fire on Escape). */
  cancelEntityEdit: () => void
  showFileInFinder: (filePath: string) => void
  copyFileAsPng: (filePath: string) => void
  updateGroupEntity: (id: string, patch: { width?: number; height?: number; canvasX?: number; canvasY?: number; label?: string; color?: string }) => void
  duplicateGroup: (id: string) => void
  deleteGroup: (id: string) => void
  renameGroup: (groupId: string, name: string) => void
  renameFileEntity: (entityId: string, name: string) => void
  renameTextEntity: (entityId: string, name: string) => void
  renameDrawingEntity: (entityId: string, name: string) => void
  dropFileBuffer: (buffer: Uint8Array, ext: string, canvasX: number, canvasY: number) => void
  /** Drop a .tsx/.jsx file into the canvas without copying its bytes — the file
   *  stays in the user's repo and the entity references it by absolute path. */
  dropComponentFile: (file: File, canvasX: number, canvasY: number) => void
  selectEntity: (
    entityId: string,
    entityKind: CanvasEntityKind,
    modifiers?: SelectionModifiers,
  ) => void
  selectGroup: (groupId: string) => void
  enterGroup: (groupId: string) => void
  enterPageInteractive: (pageId: string) => void
  startDragGroup: (groupId: string) => void
  dragGroup: (groupId: string, dx: number, dy: number, shiftKey?: boolean) => void
  endDragGroup: () => void
  startDragEntity: (entityId: string, selection?: CanvasDragStartSelection) => void
  dragEntity: (entityId: string, dx: number, dy: number, shiftKey: boolean) => void
  endDragEntity: () => void
  beginResize: (entityId: string, entityKind: CanvasEntityKind, handle: ResizeHandle) => void
  endResize: () => void
  beginMultiResize: () => void
  endMultiResize: () => void
  /** Even out gaps for the current loose multi-selection (ADR 0015 D7). No-op
   *  when fewer than 3 entities are selected or gaps are already even. */
  distributeSelection: () => void
  /** Row reorder drag (ADR 0015 D7). start → move* → commit | cancel. The begin
   *  carries only `movingId`; main resolves which door (selection / managed)
   *  armed the gesture. */
  beginReorderDrag: (movingId: string) => void
  reorderDragMove: (canvasX: number, canvasY: number) => void
  reorderDragCommit: () => void
  reorderDragCancel: (reason?: CancelReason) => void
  commitRegionSelect: (canvasRect: WorkspaceBounds) => void
  /** Comment tool click below the drag threshold. Main resolves the page +
   *  element under the window-coord point and either fires
   *  `annotate-element-selected` (element anchor) or
   *  `comment-canvas-point-committed` (no page hit / no element). ADR 0006. */
  commitCommentClickAt: (windowX: number, windowY: number) => void
  createAnnotation: (request: AnnotationCreateRequest) => void
  createDrawing: (input: { canvasX: number; canvasY: number; width: number; height: number; strokes: AnnotationDrawingStroke[] }) => void
  selectEntities: (entityIds: string[]) => void
  resizeMultiSelection: (entries: Array<{ id: string; kind: 'page' | 'text' | 'file' | 'drawing' | 'shape'; width: number; height: number; canvasX: number; canvasY: number; strokes?: AnnotationDrawingStroke[] }>) => void
  moveAnnotation: (annotationId: string, dx: number, dy: number) => void
  addAnnotationReply: (annotationId: string, text: string) => void
  resolveAnnotation: (annotationId: string) => void
  deleteAnnotation: (annotationId: string) => void
  fixSingleAnnotation: (annotationId: string) => void
  openAnnotationThread: (annotationId: string) => void
  setCommentOverlayActive: (active: boolean) => void
  onCaptureMode: (callback: (active: boolean) => void) => () => void
  onAnnotateElementSelected: (
    callback: (data: AnnotationElementSelectionPayload) => void,
  ) => () => void
  onRegionSelectCommitted: (
    callback: (data: { canvasRect: WorkspaceBounds }) => void,
  ) => () => void
  /** Comment-tool click that landed off-page (or in a page slot with no DOM
   *  element). Renderer mounts a canvas-point pending composer at the given
   *  canvas coordinates. ADR 0006. */
  onCommentCanvasPointCommitted: (
    callback: (data: { canvasX: number; canvasY: number }) => void,
  ) => () => void
  /** Page-paints contract (ADR 0006). The renderer reports the pointer's
   *  window-coord position and the current marquee rect (if any) while the
   *  comment tool is active; main fans these out to every page in page-local
   *  coords so the page can paint hover/region preview outlines. Pass `null`
   *  to clear (tool deactivated, pointer left the window, etc.). */
  setCommentToolPointerState: (
    state:
      | {
          windowX: number
          windowY: number
          regionRect: { x: number; y: number; width: number; height: number } | null
        }
      | null,
  ) => void
  /** Live-bbox subscriptions for element-anchored popovers. The renderer
   *  groups visible popovers by pageId and pushes the full set whenever it
   *  changes; main forwards to the target page. The page returns updates via
   *  `onAnnotationLiveBbox`. ADR 0006. */
  setAnnotationBboxSubscriptions: (
    pageId: string,
    subscriptions: AnnotationBboxSubscription[],
  ) => void
  /** Stream of live bboxes from any page, broadcast on layout tick / page
   *  scroll while the corresponding popover is subscribed. */
  onAnnotationLiveBbox: (
    callback: (update: AnnotationLiveBboxUpdate) => void,
  ) => () => void
  createRegionAnnotation: (canvasRect: WorkspaceBounds, text: string) => void
  onAnnotationThreadOpen: (
    callback: (data: { annotationId: string }) => void,
  ) => () => void
  beginEdgeDrag: (fromEntityId: string, fromSide: EdgeSide) => void
  updateEdgeDragTarget: (targetEntityId: string | null, targetSide: EdgeSide | null) => void
  commitEdgeDrag: (fromEntityId: string, toEntityId: string, fromSide: EdgeSide, toSide: EdgeSide) => void
  cancelEdgeDrag: () => void
  commitEdgeEdit: (
    edgeId: string,
    movingEnd: 'from' | 'to',
    targetEntityId: string,
    targetSide: EdgeSide,
  ) => void
  discardEdgeEdit: (edgeId: string) => void
  deleteEdge: (edgeId: string) => void
  selectEdge: (edgeId: string | null) => void
  hoverPage: (pageId: string | null) => void
  setTextEditing: (active: boolean) => void
  setAnnotationState: (hasOpenThread: boolean, hasPendingAnnotation: boolean) => void
  onBindingFire: (callback: (id: BindingId) => void) => () => void
  onCanvasGuides: (callback: (payload: CanvasGuidesPayload) => void) => () => void
  /** Forward a wheel event hitting the single-selected page's body to the
   *  page's webContents (aboveview-interactive-layer-poc.md). */
  forwardWheelToPage: (pageId: string, payload: ForwardWheelPayload) => void
  /** PoC: forward a pointer event hitting the single-selected page's body
   *  to the page's webContents. */
  forwardPointerToPage: (pageId: string, payload: ForwardPointerPayload) => void
  /** PoC: subscribe to the focused page's `cursor-changed` mirror so the
   *  OS cursor (chosen from aboveView, the topmost WCV) tracks what the
   *  underlying page would show. */
  onPageCursorChange: (
    callback: (data: { type: string | null }) => void,
  ) => () => void
  writeNoteFile: (filePath: string, content: string) => Promise<boolean>
  /**
   * ADR 0013 §3 — morph a plain-text entity into a markdown file entity
   * (or vice versa) at the same canvas rect. Both halves of the swap (the
   * entity replacement and the `.md` file write/delete) collapse into a
   * single undo step on the main-side undo stack.
   */
  morphTextFile: (
    entityId: string,
    direction: 'text-to-file' | 'file-to-text',
  ) => Promise<{ kind: 'morphed'; newEntityId: string } | { kind: 'noop'; reason: string }>
  getInitialData: () => Promise<CanvasLayoutBootstrapData>
  /** Connect a Vite repo at the given absolute folder path. Returns the
   *  connected repo, or null if connection fails. */
  repoConnect: (absolutePath: string) => Promise<unknown>
  onLayoutUpdate: (callback: (data: LayoutUpdateData) => void) => () => void
  onViewportNudge: (callback: (data: ViewportNudge) => void) => () => void
  onFixProgressUpdate: (
    callback: (data: LayoutUpdateData['fixProgress']) => void,
  ) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}
