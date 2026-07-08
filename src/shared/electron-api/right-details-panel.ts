import type { Tool } from '../tool'
import type {
  AnnotationCreateRequest,
  DevtoolsPanelData,
  EdgeEnd,
  EdgeSide,
  EntityUpdatePatchMap,
  FixModel,
  FixPermissions,
  PageColorScheme,
  ThemeBootstrapData,
  ThemeData,
  UpdatableEntityKind,
} from '../types'

export interface DevtoolsPanelElectronAPI {
  setTool: (tool: Tool) => void
  setTextEditing: (active: boolean) => void
  selectPage: (pageId: string) => void
  clearInspectSelection: () => void
  setInspectHoverNode: (pageId: string, nodeId: string | null) => void
  setInspectSelectedNode: (pageId: string, nodeId: string | null) => void
  editComponentProp: (
    pageId: string,
    payload: { componentId: string; propPath: string[]; value: unknown },
  ) => void
  editComponentToken: (
    pageId: string,
    payload: { componentId?: string; token: string; value: string; selector?: string },
  ) => void
  createAnnotation: (request: AnnotationCreateRequest) => void
  resolveAnnotation: (annotationId: string) => void
  deleteAnnotation: (annotationId: string) => void
  openAnnotationThread: (annotationId: string) => void
  triggerFixComments: (origin: string) => void
  fixSingleAnnotation: (annotationId: string) => void
  setAutoFix: (origin: string, enabled: boolean) => void
  pickRepoForOrigin: (origin: string) => void
  removeOriginBinding: (origin: string) => void
  setFixConfig: (config: { model: FixModel; permissions: FixPermissions }) => void
  updateEntity: <K extends UpdatableEntityKind>(kind: K, id: string, patch: EntityUpdatePatchMap[K]) => void
  duplicateTextEntity: (id: string) => void
  deleteTextEntity: (id: string) => void
  duplicateFileEntity: (id: string) => void
  deleteFileEntity: (id: string) => void
  setFilePreset: (fileId: string, presetIndex: number) => void
  setFileCustom: (fileId: string) => void
  setFileDeviceOrientation: (fileId: string, orientation: string) => void
  toggleFileDeviceShell: (fileId: string) => void
  deleteDrawingEntity: (id: string) => void
  deleteShapeEntity: (id: string) => void
  updateEdge: (id: string, patch: { fromEnd?: EdgeEnd; toEnd?: EdgeEnd; fromSide?: EdgeSide; toSide?: EdgeSide; color?: string; label?: string }) => void
  deleteEdge: (id: string) => void
  setPagePreset: (pageId: string, presetIndex: number) => void
  setPageColorScheme: (pageId: string, colorScheme: PageColorScheme | null) => void
  setPageCustom: (pageId: string) => void
  setDeviceOrientation: (pageId: string, orientation: string) => void
  toggleDeviceShell: (pageId: string) => void
  toggleSvgDeviceShell: (pageId: string) => void
  focusSelection: () => void
  duplicatePage: (pageId: string) => void
  deletePage: (pageId: string) => void
  openBrowserDevTools: () => void
  closeBrowserDevTools: () => void
  getInitialData: () => Promise<ThemeBootstrapData>
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
  onPanelData: (callback: (data: DevtoolsPanelData) => void) => () => void
}
