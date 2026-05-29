import type {
  CursorMotionParams,
  CurveDirection,
  EasingPreset,
  EasingSpec,
  MotionCandidate,
  Vec2,
} from './cursor-motion'
import type { CursorTuningParams } from './cursor-tuning'
import type { PresenceDebugEntry } from './presence-debug'
import type { DrawingBrushType, Tool } from './tool'
import type { BindingId } from './bindings'
import type { CanvasGuidesPayload } from './canvas-guides'
import type { ResizeHandle } from './resize-accumulator'

export type { DrawingBrushType, Tool } from './tool'
export type { ToolDefaultPatch } from './tool-defaults'

// --- IPC Channel Types ---

export type RepoStatus = 'stopped' | 'starting' | 'running' | 'errored'

export interface RepoOriginBinding {
  origin: string
  autoFix: boolean
}

export interface ConnectedRepo {
  id: string
  absolutePath: string
  label: string
  status: RepoStatus
  port: number | null
  baseUrl: string | null
  lastError?: string
  /** Origins (e.g. https://acme.com) that map to this repo for agent fixes. */
  boundOrigins: RepoOriginBinding[]
}

export interface ViewportPreset {
  label: string
  width: number
  height: number
  mobile: boolean
}

export interface PageConfig {
  id?: string
  name?: string
  url: string
  presetIndex: number
  canvasX: number
  canvasY: number
  linked?: boolean
  suppressInitialNavigationBroadcast?: boolean
  source?: WorkspacePageSource
  parentGroupId?: string
  groupId?: string
  metadata?: Record<string, unknown>
}

// --- Generic Canvas Entity Types ---

export type CanvasEntityKind = 'page' | 'text' | 'file' | 'group' | 'edge' | 'drawing' | 'shape'

export type ShapeKind = 'rectangle' | 'ellipse' | 'diamond'

/**
 * Renderer plugin popup contribution tags (ADR 0008 §7). Each tag names a
 * single piece of UI a renderer plugin can opt into in the file selection
 * popup. The main-side registry declares which tags a renderer claims; the
 * renderer-side `renderPopupContributions` switch picks the React component.
 *
 * Adding a tag requires both ends: a literal here + a case in
 * `src/renderer/above-view/file-popup-contributions/index.tsx`.
 */
export type PopupContributionTag =
  | 'wireframe-theme'
  | 'wireframe-json-mode'
  | 'wireframe-device-controls'
  | 'markdown-morph-to-text'

export interface CanvasEntityRef {
  kind: CanvasEntityKind
  id: string
}

export type CanvasSelectableTarget = CanvasEntityRef

export type CanvasHoverTarget = CanvasSelectableTarget | null

export type SelectionModifiers = {
  shift: boolean
  meta: boolean
  ctrl: boolean
}

export type CanvasInteractionState =
  | { kind: 'idle' }
  | {
      kind: 'dragging-edge'
      from: CanvasSelectableTarget
      fromSide: EdgeSide
      target: CanvasSelectableTarget | null
      targetSide: EdgeSide | null
    }
  | { kind: 'dragging-entities'; entityIds: string[] }
  | { kind: 'marquee-select' }
  | { kind: 'panning-canvas' }
  | { kind: 'resizing-entity'; entity: CanvasSelectableTarget }
  | { kind: 'resizing-multi-selection' }
  | { kind: 'editing-entity'; entityId: string }

export interface CanvasScenePageEntity {
  kind: 'page'
  id: string
  label: string
  faviconUrl?: string | null
  url: string
  canGoBack: boolean
  canGoForward: boolean
  isLoading: boolean
  isCustomSize: boolean
  browserSizeMode: 'fill' | 'device'
  canvasX: number
  canvasY: number
  width: number
  height: number
  presetIndex: number
  linked: boolean
  /** Outer screen bounds (includes shell when device page is on). */
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  /** Device page state. */
  deviceId?: string | null
  deviceOrientation?: 'portrait' | 'landscape'
  showDeviceFrame?: boolean
  /** Inner content screen bounds (always the web viewport). */
  contentScreenX?: number
  contentScreenY?: number
  contentScreenWidth?: number
  contentScreenHeight?: number
  /** Use SVG rendering for the device shell (A/B toggle). */
  useSvgDeviceShell?: boolean
}

/** 'plain' = unbacked text, 'sticky' = text in a colored card. See ADR 0004. */
export type TextEntityStyle = 'plain' | 'sticky'

/**
 * 'auto' = shell hugs content (no wrap; width/height reflect rendered text).
 * 'fixed' = explicit width/height; text wraps within bounds. Plain text
 * defaults to 'auto' on creation; the first manual resize flips to 'fixed'.
 * Sticky is always 'fixed'.
 */
export type TextWidthMode = 'auto' | 'fixed'

export interface CanvasSceneTextEntity {
  kind: 'text'
  id: string
  text: string
  color: string
  textStyle: TextEntityStyle
  widthMode: TextWidthMode
  /** Per-entity text size in px. Missing → renderer default (18). ADR 0013 §2. */
  textSize?: number
  canvasX: number
  canvasY: number
  width: number
  height: number
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  parentGroupId?: string
}

export interface CanvasSceneFileEntity {
  kind: 'file'
  id: string
  file: string
  subpath?: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  parentGroupId?: string
  objectFit?: FileObjectFit
  /** Renderer-side dispatch tag chosen by the entity-renderer registry. */
  rendererTag?: 'image' | 'video' | 'markdown' | 'wireframe' | 'component' | 'html'
  /**
   * Static contribution tags declared by the picked renderer plugin (ADR 0008
   * §7). The `FilePopup` reads these to compose plugin-specific controls
   * (e.g. wireframe theme picker). Empty array means no contributions. The
   * tag → component switch lives renderer-side; this string list is the
   * cross-layer contract.
   */
  popupContributions?: PopupContributionTag[]
  /** Whether the resolved renderer has a meaningful inline-edit affordance.
   *  Drives both the dblclick and click-on-solo-selected paths in the
   *  pointer router. Undefined for unclaimed (fallback) entities — treated
   *  as `false`. */
  rendererEditable?: boolean
  /**
   * For component file entities: whether some connected repo claims this
   * file (i.e. resolveUrl will succeed). The renderer suppresses the
   * placeholder when true so the WCV shows through cleanly without a
   * faded "Connect a Vite repo" copy bleeding behind transparent content.
   */
  componentHasRepo?: boolean
  /**
   * For component file entities without a connected repo: the nearest
   * ancestor folder that contains a package.json. Surfaced so the
   * placeholder can offer one-click reconnect without prompting the user
   * to re-pick the folder.
   */
  componentInferredRepoPath?: string
  /** Device page state. */
  deviceId?: string | null
  deviceOrientation?: 'portrait' | 'landscape'
  showDeviceFrame?: boolean
  /** Inner content screen bounds (when device page is on). */
  contentScreenX?: number
  contentScreenY?: number
  contentScreenWidth?: number
  contentScreenHeight?: number
}

export interface CanvasSceneGroupEntity {
  kind: 'group'
  id: string
  label: string
  color?: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  parentGroupId?: string
  layoutMode: WorkspaceGroupLayoutMode
  managedLayout: boolean
  entityIds: string[]
}

export interface CanvasSceneDrawingEntity {
  kind: 'drawing'
  id: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  strokes: AnnotationDrawingStroke[]
  parentGroupId?: string
}

export interface CanvasSceneShapeEntity {
  kind: 'shape'
  id: string
  shapeKind: ShapeKind
  text: string
  color?: string
  strokeWidth?: number
  /** Per-entity text size in px for the inner label. ADR 0013 §2. */
  textSize?: number
  theme?: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
}

export type CanvasSceneEntity =
  | CanvasScenePageEntity
  | CanvasSceneTextEntity
  | CanvasSceneFileEntity
  | CanvasSceneGroupEntity
  | CanvasSceneDrawingEntity
  | CanvasSceneShapeEntity

export interface ActiveCanvasEntitySelection {
  entityRef: CanvasEntityRef
  label: string
  width: number
  height: number
  presetIndex: number
  linked: boolean
}

export interface PendingPlacement {
  entityKind: CanvasEntityKind
  presetIndex?: number
  shapeKind?: ShapeKind
  textStyle?: TextEntityStyle
  /** Stored color of the in-flight placement (sticky fill, etc.) so the preview can match the picker. */
  color?: string
  width: number
  height: number
}

// --- Persisted Entity Types ---

export interface CanvasEntityBase {
  id: string
  kind: CanvasEntityKind
  canvasX: number
  canvasY: number
  parentGroupId?: string
}

export interface PersistedPageEntity extends CanvasEntityBase {
  kind: 'page'
  name?: string
  url: string
  presetIndex: number
  linked: boolean
  source?: WorkspacePageSource
  groupId?: string
  metadata?: Record<string, unknown>
}

export interface PersistedTextEntity extends CanvasEntityBase {
  kind: 'text'
  text: string
  color: string
  width: number
  height: number
  /** Optional — reader defaults to 'sticky' when absent (legacy canvases). See ADR 0004. */
  textStyle?: TextEntityStyle
  /** Optional — reader defaults: plain → 'auto', sticky → 'fixed'. */
  widthMode?: TextWidthMode
  /** Optional — renderer defaults to 14 ("Small") when absent. ADR 0013 §2. */
  textSize?: number
  label?: string
}

export type FileObjectFit = 'contain' | 'cover' | 'fill'

export interface PersistedFileEntity extends CanvasEntityBase {
  kind: 'file'
  file: string
  subpath?: string
  width: number
  height: number
  objectFit?: FileObjectFit
  presetIndex?: number
  metadata?: Record<string, unknown>
}

export type WorkspaceGroupLayoutMode = 'freeform' | 'row' | 'grid'

export interface PersistedGroupEntity extends CanvasEntityBase {
  kind: 'group'
  label: string
  color?: string
  width: number
  height: number
  layoutMode: WorkspaceGroupLayoutMode
  managedLayout: boolean
  sourceTaskId?: string
  metadata?: Record<string, unknown>
}

export interface PersistedDrawingEntity extends CanvasEntityBase {
  kind: 'drawing'
  width: number
  height: number
  strokes: AnnotationDrawingStroke[]
  label?: string
}

export interface PersistedShapeEntity extends CanvasEntityBase {
  kind: 'shape'
  shapeKind: ShapeKind
  text: string
  color?: string
  strokeWidth?: number
  /** Per-entity text size in px for the inner label. ADR 0013 §2. */
  textSize?: number
  theme?: string
  width: number
  height: number
  label?: string
}

export type PersistedCanvasEntity =
  | PersistedPageEntity
  | PersistedTextEntity
  | PersistedFileEntity
  | PersistedGroupEntity
  | PersistedDrawingEntity
  | PersistedShapeEntity

// --- Layout Update Data ---

export interface LayoutUpdateData {
  zoom: number
  pan: { x: number; y: number }
  canvasOrigin: { x: number; y: number }
  /**
   * Width of left-edge chrome (sidebar) currently covering the canvas. 0 when
   * the sidebar is closed. The canvas coordinate system is not shifted by the
   * sidebar; surfaces that need to avoid occluded pixels (clipping, viewport
   * centering, tab-bar insets) read this instead of canvasOrigin.x.
   */
  leftChromeWidth: number
  /**
   * X-coordinate (in window pixels) of the centerpoint of the toolbar's tool
   * cluster, when the toolbar is in `showCenterActionsOnly` mode. Popups that
   * anchor below the toolbar (tool-mode popups, ADR 0008 §1) read this to
   * align with the tools regardless of platform padding (mac traffic-lights
   * inset) or sidebar state.
   *
   * Computed in main from `TOOLBAR_PAD_*` constants and the current window
   * width; mirrors the `grid-cols-[1fr_auto_1fr]` layout the toolbar uses in
   * that mode.
   */
  toolbarCenterX: number
  /** Back-to-front stack order across canvas nodes and edges. */
  entityOrder: string[]
  entities: CanvasSceneEntity[]
  browserTabs: WorkspaceTabPageSummary[]
  browserFillViewport: {
    width: number
    height: number
  }
  selectedEntityIds: string[]
  selection: CanvasSelectableTarget[]
  activeSelection: ActiveCanvasEntitySelection | null
  activeTool: Tool
  /** Per-tool persistent defaults (ADR 0008 §9). Tool-mode popup reads/writes. */
  toolDefaults: import('./tool-defaults').ToolDefaults
  annotations: Annotation[]
  fixProgress: Record<string, FixProgressEntry>
  viewMode: WorkspaceViewMode
  activeBrowserTabId: string | null
  activeBrowserPageId: string | null
  selectedGroupId?: string | null
  hover: CanvasHoverTarget
  interaction: CanvasInteractionState
  pendingPlacement: PendingPlacement | null
  devtoolsOpen: boolean
  devtoolsWidth: number
  edges: WorkspaceEdge[]
  groups?: CanvasSceneGroupEntity[]
  presenceCursors: AgentPresenceCursor[]
  /** Predicate-derived: the page id that should hold keyboard + receive
   *  forwarded input, or null. See `shouldFocusSelectedPage`. */
  keyboardTargetPageId: string | null
}

export type PresenceSurface = 'canvas' | 'page'

export type PresenceActivity = 'traveling' | 'acting' | 'waiting' | 'thinking' | 'idle' | 'departing'

export type PresenceLabelKey =
  | 'scan_workspace'
  | 'find_placement'
  | 'create_page'
  | 'select_page'
  | 'attach_page'
  | 'inspect_page'
  | 'find_target'
  | 'click_target'
  | 'type_text'
  | 'select_option'
  | 'wait_page'
  | 'scroll_page'
  | 'read_content'
  | 'add_annotation'
  | 'thinking'
  | 'idle'
  | 'departing'

export interface PresenceTargetRect {
  x: number
  y: number
  width: number
  height: number
}

interface PresenceCoordinates {
  canvasX?: number
  canvasY?: number
  pageX?: number
  pageY?: number
  targetRect?: PresenceTargetRect | null
}

export type PresenceTargetRefSource = 'specular' | 'agent-browser'

interface PresenceEvent {
  sessionId: string
  surface: PresenceSurface
  phase: PresenceActivity
  eventType?: 'start' | 'surface' | 'act' | 'think' | 'done' | null
  pageId?: string | null
  coordinates?: PresenceCoordinates | null
  labelKey: PresenceLabelKey | null
  taskLabel?: string | null
  labelHint?: string | null
  labelParams?: Record<string, string | number | boolean> | null
  targetRef?: string | null
  targetRefSource?: PresenceTargetRefSource | null
  targetName?: string | null
}

export interface AgentPresenceCursor {
  sessionId: string
  clientName: string
  color: string
  canvasX: number
  canvasY: number
  surface: PresenceSurface
  activity: PresenceActivity
  pageId?: string | null
  pageX?: number | null
  pageY?: number | null
  labelKey: PresenceLabelKey | null
  taskLabel?: string | null
  labelHint?: string | null
  labelParams?: Record<string, string | number | boolean> | null
  targetRef?: string | null
  targetRefSource?: PresenceTargetRefSource | null
  targetName?: string | null
  targetRect?: PresenceTargetRect | null
  updatedAt: number
}

export interface AgentSnapshotNode {
  ref: string
  parentRef?: string | null
  depth: number
  tagName: string
  role?: string
  name?: string
  text?: string
  interactive: boolean
  bounds: PresenceTargetRect
  elementPath: string
  fullPath: string
}

export interface AgentSnapshotPage {
  pageId: string
  url: string
  title: string
  nodes: AgentSnapshotNode[]
}

export interface SidebarPageItem {
  kind: 'page'
  id: string
  label: string
  faviconUrl?: string | null
  width?: number
  height?: number
}

export interface SidebarTextItem {
  kind: 'text'
  id: string
  label: string
  color: string
}

export interface SidebarFileItem {
  kind: 'file'
  id: string
  label: string
  file: string
}

export interface SidebarDrawingItem {
  kind: 'drawing'
  id: string
  label: string
  strokeCount: number
}

export interface SidebarShapeItem {
  kind: 'shape'
  id: string
  label: string
  shapeKind: ShapeKind
}

export interface SidebarGroupItem {
  kind: 'group'
  id: string
  label: string
  entityCount: number
  children: SidebarCanvasItem[]
}

export type SidebarCanvasItem =
  | SidebarPageItem
  | SidebarTextItem
  | SidebarFileItem
  | SidebarDrawingItem
  | SidebarShapeItem
  | SidebarGroupItem

export type SidebarSectionKey = 'notes' | 'pages'

export interface LeftSidebarSections {
  notes: SidebarCanvasItem[]
  pages: SidebarCanvasItem[]
}

export interface LeftSidebarData {
  width: number
  selectedEntityIds: string[]
  selectedGroupId?: string | null
  tabs: WorkspaceTabSummary[]
  activeTabId: string | null
  viewMode: WorkspaceViewMode
  hasPages: boolean
  sections: LeftSidebarSections
  items: SidebarCanvasItem[]
}

export interface ToolbarSelectionData {
  activePageId: string | null
  selectedEntityIds: string[]
  selectionCount: number
  availablePageCount: number
  displayUrl: string
  placeholder: string
  canGoBack: boolean
  canGoForward: boolean
  isLoadingActivePage: boolean
  loadingPageCount: number
  isLoadingAnySelected: boolean
  loadingPhase: 'idle' | 'waiting-response' | 'loading'
  activeTabId: string | null
  activeTabName: string | null
  viewMode: WorkspaceViewMode
  activeTool: Tool
  /** Current draw-tool brush default — drives which glyph the Draw button shows. */
  drawBrushType: DrawingBrushType
  /** Current draw-tool color default (raw stored slot/hex) — tints the Draw glyph. */
  drawColor: string
  /** Current sticky-tool color default (raw stored slot/hex) — tints the sticky glyph. */
  stickyColor: string
}

export interface ThemeData {
  isDark: boolean
}

export interface ThemeBootstrapData {
  theme: ThemeData
}

export interface DebugBootstrapData extends ThemeBootstrapData {
  cursorMotion: CursorMotionParams
  cursorSplineViz: boolean
  cursorTuning: CursorTuningParams
  presenceTimeline: PresenceDebugEntry[]
}

export interface LeftSidebarBootstrapData extends ThemeBootstrapData {
  sidebarData: LeftSidebarData
}

// --- Onboarding ---

export type OnboardingComponentId = 'cli' | 'skill' | 'agentBrowser'

export type OnboardingComponentStatus =
  | { kind: 'installed'; detail?: string }
  | { kind: 'outdated'; detail?: string }
  | { kind: 'missing'; detail?: string }
  | { kind: 'blocked'; detail: string }

export interface OnboardingStatusSnapshot {
  cli: OnboardingComponentStatus
  skill: OnboardingComponentStatus
  agentBrowser: OnboardingComponentStatus
  claudeDirExists: boolean
}

export type OnboardingMode = 'welcome' | 'settings'

export interface OnboardingBootstrapData extends ThemeBootstrapData {
  status: OnboardingStatusSnapshot
  mode: OnboardingMode
}

export type OnboardingProgressEvent =
  | { component: OnboardingComponentId; state: 'installing' }
  | { component: OnboardingComponentId; state: 'success'; detail?: string }
  | { component: OnboardingComponentId; state: 'error'; detail: string }
  | { kind: 'done'; status: OnboardingStatusSnapshot }

export interface OnboardingState {
  completed: boolean
  dismissedAt?: number
  completedAt?: number
  /** SHA-256 of each skill's content as we last installed it. Used to
   * detect whether the user has hand-edited the file before auto-updating. */
  skillHashes?: { specular?: string; 'agent-browser'?: string }
}

export interface OnboardingElectronAPI {
  getInitialData: () => Promise<OnboardingBootstrapData>
  refreshStatus: () => Promise<OnboardingStatusSnapshot>
  install: (
    selections: Record<OnboardingComponentId, boolean>,
  ) => Promise<OnboardingStatusSnapshot>
  complete: () => void
  dismiss: () => void
  onProgress: (callback: (event: OnboardingProgressEvent) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}

// --- Settings window ---

export interface SettingsBootstrapData extends ThemeBootstrapData {
  status: OnboardingStatusSnapshot
  fixConfig: FixConfig
  connectedRepos: ConnectedRepo[]
}

export interface SettingsElectronAPI {
  getInitialData: () => Promise<SettingsBootstrapData>
  refreshStatus: () => Promise<OnboardingStatusSnapshot>
  installSkills: (
    selections: Record<OnboardingComponentId, boolean>,
  ) => Promise<OnboardingStatusSnapshot>
  setComponentInstalled: (
    component: OnboardingComponentId,
    installed: boolean,
  ) => Promise<OnboardingStatusSnapshot>
  setFixConfig: (config: { model: FixModel; permissions: FixPermissions }) => void
  removeOriginBinding: (origin: string) => void
  repoConnectViaPicker: () => Promise<ConnectedRepo | null>
  repoDisconnect: (id: string) => Promise<void>
  repoBindOrigin: (repoId: string, origin: string) => Promise<ConnectedRepo | null>
  close: () => void
  onSkillProgress: (callback: (event: OnboardingProgressEvent) => void) => () => void
  onFixConfigChanged: (callback: (config: FixConfig) => void) => () => void
  onConnectedReposChanged: (callback: (repos: ConnectedRepo[]) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}

export interface DebugElectronAPI {
  getInitialData: () => Promise<DebugBootstrapData>
  updateCursorMotion: (params: CursorMotionParams) => void
  resetCursorMotion: () => void
  onCursorMotionChanged: (callback: (params: CursorMotionParams) => void) => () => void
  updateCursorSplineViz: (on: boolean) => void
  onCursorSplineVizChanged: (callback: (on: boolean) => void) => () => void
  updateCursorTuning: (params: CursorTuningParams) => void
  resetCursorTuning: () => void
  onPresenceTimelineAppend: (callback: (entry: PresenceDebugEntry) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}

interface DebugPreviewPath {
  start: Vec2
  end: Vec2
  candidates: MotionCandidate[]
  selectedCandidateIndex: number
  samples: Vec2[]
}

export type {
  CursorMotionParams,
  CurveDirection,
  CursorTuningParams,
  EasingPreset,
  EasingSpec,
  PresenceDebugEntry,
  Vec2,
}

export interface CanvasLayoutBootstrapData extends ThemeBootstrapData {
  layoutData: LayoutUpdateData
}

interface FloatingUiUpdatePayload {
  layoutData: LayoutUpdateData
  surfaceOrigin: { x: number; y: number }
}

interface FloatingUiBootstrapData extends ThemeBootstrapData, FloatingUiUpdatePayload {}

// --- Panel mode (selection-driven) ---

export type PanelMode =
  | { kind: 'document' }
  | { kind: 'page'; entityId: string }
  | { kind: 'text'; entityId: string }
  | { kind: 'file'; entityId: string }
  | { kind: 'drawing'; entityId: string }
  | { kind: 'shape'; entityId: string }
  | { kind: 'edge'; entityId: string }
  | { kind: 'group'; entityId: string }
  | { kind: 'multi'; entityIds: string[] }

export interface PanelShapeEntityDetail {
  id: string
  shapeKind: ShapeKind
  text: string
  color?: string
  strokeWidth?: number
  width: number
  height: number
}

export interface PanelTextEntityDetail {
  id: string
  text: string
  color: string
  width: number
  height: number
}

export type PanelFileType =
  | 'image'
  | 'video'
  | 'markdown'
  | 'wireframe'
  | 'component'
  | 'html'
  | 'other'

export interface PanelFileEntityDetail {
  id: string
  file: string
  subpath?: string
  width: number
  height: number
  objectFit?: FileObjectFit
  fileType: PanelFileType
  presetIndex?: number
  deviceId?: string | null
  deviceOrientation?: 'portrait' | 'landscape'
  showDeviceFrame?: boolean
}

export interface PanelDrawingEntityDetail {
  id: string
  width: number
  height: number
  strokeCount: number
}

export interface PanelEdgeEntityDetail {
  id: string
  fromEntityId: string
  toEntityId: string
  fromLabel: string
  toLabel: string
  fromSide?: EdgeSide
  toSide?: EdgeSide
  fromEnd?: EdgeEnd
  toEnd?: EdgeEnd
  color?: string
  label?: string
  kind: 'breakpoint_variant' | 'connection'
}

export interface PanelGroupEntityDetail {
  id: string
  label: string
  color?: string
  layoutMode: WorkspaceGroupLayoutMode
  entityIds: string[]
}

export interface PanelMultiEntitySummary {
  id: string
  kind: CanvasEntityKind
  label: string
}

export interface DevtoolsPanelData {
  activeTab: DevtoolsPanelTab
  panelMode: PanelMode
  activeTool: Tool
  annotateEnabled?: boolean
  annotateAvailable?: boolean
  focusedAnnotationId?: string | null
  selection?: DevtoolsPanelSelectionSummary
  inspect?: InspectPanelState
  annotations?: Annotation[]
  pages?: DevtoolsPanelPageSummary[]
  originBindings?: OriginBindings
  fixInProgress?: Record<string, number>
  fixProgress?: Record<string, FixProgressEntry>
  fixConfig?: FixConfig
  textEntity?: PanelTextEntityDetail
  fileEntity?: PanelFileEntityDetail
  drawingEntity?: PanelDrawingEntityDetail
  shapeEntity?: PanelShapeEntityDetail
  edgeEntity?: PanelEdgeEntityDetail
  groupEntity?: PanelGroupEntityDetail
  multiEntities?: PanelMultiEntitySummary[]
  emptyState?: {
    kind: 'mcp_setup'
    serverName: string
    command: string
    installCommand: string
    tools: string[]
    configPath: string
    discoveryFile: string
    status: {
      healthy: boolean
      appServerRunning: boolean
      discoveryFilePresent: boolean
      mcpClientConnected: boolean
      activeClientCount: number
      lastClientSeenAt: string | null
    }
  }
}

export interface DevtoolsPanelPageSummary {
  id: string
  label: string
  url: string
  faviconUrl?: string | null
  width?: number
  height?: number
  presetIndex: number
  deviceId?: string | null
  deviceOrientation?: 'portrait' | 'landscape'
  showDeviceFrame?: boolean
  useSvgDeviceShell?: boolean
  canGoBack?: boolean
  canGoForward?: boolean
  isLoading?: boolean
  linked?: boolean
}

export type DevtoolsPanelTab = 'comments' | 'inspect' | 'browser-devtools' | 'settings'

export type InspectNodeSource = 'react' | 'dom' | 'dom_fallback'

export type InspectMode = 'page_locked' | 'global_target'

export interface InspectNodeSummary {
  id: string
  parentId?: string
  pageId: string
  name: string
  source: InspectNodeSource
  dsComponentName?: string
  hasSource: boolean
  childrenIds: string[]
}

export interface InspectNodeDetail extends DevtoolsPanelDomTarget {
  nodeId: string
  pageId: string
  props?: Record<string, unknown>
  tokens?: Record<string, string>
  dsComponentName?: string
  sourceLocation?: SourceLocation
  dsVariants?: Record<string, string>
  dsPropSignature?: Array<{
    name: string
    type: 'string' | 'number' | 'boolean' | 'enum'
    values?: string[]
    defaultValue?: string
  }>
}

export interface InspectPanelState {
  available: boolean
  enabled: boolean
  mode: InspectMode
  activePageId: string | null
  hoveredNodeId: string | null
  selectedNodeId: string | null
  treeRootIds: string[]
  nodesById: Record<string, InspectNodeSummary>
  detailById: Record<string, InspectNodeDetail>
  diagnostics?: {
    collector:
      | 'hook'
      | 'dom_fiber'
      | 'main_world'
      | 'dom_fallback'
      | 'unknown'
    nodeCount: number
    reactNodeCount: number
    domFallbackNodeCount: number
    sourceLocationCount: number
  }
}

export interface DevtoolsPanelSelectionSummary {
  pageId: string
  url: string
  pageTitle: string
  viewportLabel: string
  width: number
  height: number
  linked: boolean
}

export interface DevtoolsPanelDomRect {
  x: number
  y: number
  width: number
  height: number
}

export interface DevtoolsPanelDomAttribute {
  name: string
  value: string
}

export interface DevtoolsPanelDomTarget {
  id: string
  pageId: string
  timestamp: number
  tagName: string
  name: string
  role?: string
  elementPath: string
  fullPath: string
  cssClasses: string[]
  textPreview?: string
  nearbyText?: string
  nearbyElements: string[]
  accessibility: string[]
  attributes: DevtoolsPanelDomAttribute[]
  computedStyles: string[]
  boundingBox?: DevtoolsPanelDomRect
  position?: {
    viewportXPercent: number
    documentY: number
    isFixed: boolean
  }
}

interface DevtoolsPanelDomInspectorState {
  available: boolean
  inspectMode: boolean
  hoverTarget: DevtoolsPanelDomTarget | null
  selectedTarget: DevtoolsPanelDomTarget | null
}

export interface ScrollSyncData {
  xProgress: number
  yProgress: number
  viewportCenterProgress?: number
  sourceUrl: string
  anchorSelector?: string
  anchorProgress?: number
}

export interface SourceLocation {
  file: string
  line?: number
  column?: number
}

export type UiSelection =
  | { kind: 'none' }
  | { kind: 'single-entity'; entityId: string; entityKind: CanvasEntityKind }
  | {
      kind: 'multi-entity'
      entityIds: string[]
      entityKindsById: Partial<Record<string, CanvasEntityKind>>
    }

export type SelectionOverlayRect = {
  left: number
  top: number
  width: number
  height: number
}

export type SelectionOverlayPayload = {
  rect: SelectionOverlayRect
  variant?: 'default' | 'region-select' | 'place-shape'
  shapeKind?: ShapeKind
  /** Entity IDs the marquee currently overlaps. Populated only for the
   *  default variant; the canvas-bg outline layer reads this to draw a
   *  "would-be selected" highlight on each entity inside the rect. */
  entityIds?: string[]
}

export type UiViewMode =
  | { kind: 'canvas' }
  | { kind: 'browser'; pageId: string }

export interface UiDevtoolsState {
  open: boolean
  activeTab: DevtoolsPanelTab
  focusedAnnotationId: string | null
  width: number
}

export interface UiOverlayState {
  commentOverlayVisible: boolean
  selectionMarqueeVisible: boolean
}

export interface UiState {
  selection: UiSelection
  activeTool: Tool
  viewMode: UiViewMode
  leftSidebarOpen: boolean
  /** True while a toolbar dropdown is open — the layout pass grows the
   *  toolbar view to full-window bounds so the menu can overflow the strip. */
  toolbarDropdownOpen: boolean
  devtools: UiDevtoolsState
  overlays: UiOverlayState
}

export interface ComponentTreeNode {
  id: string
  componentName: string
  dsComponentName?: string
  hasSource: boolean
  children: ComponentTreeNode[]
}

export interface ComponentNodeDetail {
  props: Record<string, unknown>
  tokens: Record<string, string>
  sourceLocation?: SourceLocation
  dsComponentName?: string
  dsVariants?: Record<string, string>
  dsPropSignature?: Array<{
    name: string
    type: 'string' | 'number' | 'boolean' | 'enum'
    values?: string[]
    defaultValue?: string
  }>
}

export interface WorkspacePageSnapshot {
  id?: string
  name?: string
  title?: string
  url: string
  presetIndex: number
  canvasX: number
  canvasY: number
  linked: boolean
  source?: WorkspacePageSource
  parentGroupId?: string
  groupId?: string
  metadata?: Record<string, unknown>
}

export interface WorkspaceSnapshot {
  zoom: number
  pan: { x: number; y: number }
  /** @deprecated Use entities instead. Kept for backward compatibility with old snapshots. */
  pages: WorkspacePageSnapshot[]
  /** Generic entity store. When present, this is the canonical source of truth. */
  entities?: Record<string, PersistedCanvasEntity>
  /** Ordered entity IDs for z-ordering (front-to-back). */
  entityOrder?: string[]
  selectedPageIndex: number | null
  selectedPageId?: string | null
  selectedPageIds?: string[]
  selectedGroupId?: string | null
  leftSidebarOpen?: boolean
  devtoolsOpen: boolean
  devtoolsPanelTab?: DevtoolsPanelTab
  devtoolsWidth: number
  browserTabMode?: BrowserTabMode
  groups?: WorkspaceGroup[]
  edges?: WorkspaceEdge[]
}

export interface PersistedWorkspaceTab {
  id: string
  name: string
  updatedAt: string
  snapshot: WorkspaceSnapshot
  annotations: Annotation[]
  expanded?: boolean
}

export interface PersistedWorkspaceRecord {
  id: string
  name: string
  updatedAt: string
  activeTabId: string
  viewMode?: WorkspaceViewMode
  tabs: PersistedWorkspaceTab[]
}

export interface PersistedWorkspaceStore {
  version: 2
  activeWorkspaceId: string
  workspaces: PersistedWorkspaceRecord[]
}

export type LegacyPersistedWorkspaceRecord = {
  id: string
  name: string
  updatedAt: string
  snapshot: WorkspaceSnapshot
  annotations: Annotation[]
}

export type LegacyPersistedWorkspaceStore = {
  version: 1
  activeWorkspaceId: string
  workspaces: LegacyPersistedWorkspaceRecord[]
}

export type WorkspacePageSource = 'manual' | 'generated'

export interface WorkspacePage {
  id: string
  kind: 'page'
  name?: string
  url: string
  presetIndex: number
  canvasX: number
  canvasY: number
  width: number
  height: number
  linkedBrowsing: boolean
  source: WorkspacePageSource
  parentGroupId?: string
  groupId?: string
  metadata?: Record<string, unknown>
}

export interface WorkspaceTextEntity {
  id: string
  kind: 'text'
  /** First 80 chars of the text content. Use get_text_entities for full content. */
  preview: string
  color: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
}

export interface WorkspaceFileEntity {
  id: string
  kind: 'file'
  file: string
  subpath?: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
}

export interface ClipboardPagePayload {
  url: string
  presetIndex: number
  dx: number
  dy: number
}

export interface ClipboardPageSelectionPayload {
  version: 1
  pages: ClipboardPagePayload[]
}

export interface ClipboardEntityPayload {
  kind: CanvasEntityKind
  dx: number
  dy: number
  // Page-specific
  url?: string
  presetIndex?: number
  // Page device metadata (so paste reproduces the device shell)
  metadata?: Record<string, unknown>
  // Text entity-specific
  text?: string
  color?: string
  textStyle?: TextEntityStyle
  /** Per-entity text size in px (text + shape entities). ADR 0013 §2. */
  textSize?: number
  width?: number
  height?: number
  // File entity-specific
  file?: string
  subpath?: string
  objectFit?: FileObjectFit
  // Shape entity-specific
  shapeKind?: ShapeKind
  strokeWidth?: number
  theme?: string
  label?: string
  // Drawing entity-specific; points are relative to the drawing origin.
  strokes?: AnnotationDrawingStroke[]
}

export interface ClipboardEntitySelectionPayload {
  version: 2
  entities: ClipboardEntityPayload[]
}

export interface WorkspaceGroup {
  id: string
  kind: 'group'
  label: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  color?: string
  layoutMode: WorkspaceGroupLayoutMode
  managedLayout: boolean
  pageIds?: string[]
  entityIds?: string[]
  sourceTaskId?: string
  metadata?: Record<string, unknown>
}

export type WorkspaceCanvasEntity =
  | WorkspacePage
  | WorkspaceTextEntity
  | WorkspaceFileEntity
  | WorkspaceGroup

export type EdgeSide = 'top' | 'right' | 'bottom' | 'left'
export type EdgeEnd = 'none' | 'arrow'

export interface WorkspaceEdge {
  id: string
  fromEntityId: string
  toEntityId: string
  fromSide?: EdgeSide
  toSide?: EdgeSide
  fromEnd?: EdgeEnd
  toEnd?: EdgeEnd
  color?: string
  label?: string
  kind: 'breakpoint_variant' | 'connection'
  metadata?: Record<string, unknown>
}


export interface WorkspaceSelection {
  selectedEntityId?: string
  selectedEntityIds?: string[]
  selectedGroupId?: string
}

export interface WorkspaceBounds {
  x: number
  y: number
  width: number
  height: number
}

export type WorkspaceViewMode = 'canvas' | 'browser'
/** @deprecated Browser mode no longer has sub-modes; kept for snapshot compat */
export type BrowserTabMode = 'responsive' | 'page'

export interface WorkspaceTabPageSummary {
  id: string
  label: string
  name?: string
  url: string
  presetIndex: number
  faviconUrl?: string | null
  width?: number
  height?: number
}

export interface WorkspaceTabSummary {
  id: string
  name: string
  expanded: boolean
  isActive: boolean
  pageCount: number
  pages: WorkspaceTabPageSummary[]
}

export interface WorkspaceGraph {
  entities: WorkspaceCanvasEntity[]
  edges: WorkspaceEdge[]
  selection: WorkspaceSelection
  camera: {
    zoom: number
    panX: number
    panY: number
  }
  occupiedRegions: WorkspaceBounds[]
}

export type PlacementAnchor = 'selection_or_empty_region' | 'empty_region'

export interface PlacementRequest {
  width: number
  height: number
  anchor: PlacementAnchor
}

export interface PlacementResult {
  canvasX: number
  canvasY: number
  fallbackUsed: boolean
  reason: string
}

export type BatchLayoutMode = 'row' | 'column' | 'grid'

export interface BatchPlacementRequest {
  /**
   * Each item's `width`/`height` is the OUTER (visible) footprint, including
   * device-shell bezels. The hover-only chrome action header is reserved
   * separately by occupied-region inflation, so it doesn't widen `gap`.
   * `insetX`/`insetY` (default 0) describe the offset from the outer top-left
   * to the entity's data origin (`canvasX`/`canvasY`); the layout engine
   * places outer footprints with `gap` and returns positions in inner
   * data-origin coordinates.
   */
  items: Array<{ width: number; height: number; insetX?: number; insetY?: number }>
  layout?: BatchLayoutMode
  gap?: number
  anchor?: PlacementAnchor
}

export interface BatchPlacementResult {
  positions: Array<{ canvasX: number; canvasY: number }>
}

export type SpacingToken = 'xs' | 's' | 'm' | 'l' | 'xl'

// All multiples of GRID_SIZE (20px) so token-spaced gaps stay snap-aligned.
export const SPACING_TOKEN_PIXELS: Record<SpacingToken, number> = {
  xs: 20,
  s: 40,
  m: 60,
  l: 100,
  xl: 160,
}

export function resolveSpacing(value: number | SpacingToken | undefined, fallback: number): number {
  if (value === undefined) return fallback
  if (typeof value === 'number') return value
  return SPACING_TOKEN_PIXELS[value] ?? fallback
}

export interface LayoutDirective {
  kind: BatchLayoutMode
  gap?: number | SpacingToken
  rowGap?: number | SpacingToken
  colGap?: number | SpacingToken
  cols?: number
  originX?: number
  originY?: number
  near?: string
}

const SPACING_TOKEN_NAMES: ReadonlySet<string> = new Set(Object.keys(SPACING_TOKEN_PIXELS))

function isSpacingValue(v: unknown): boolean {
  return typeof v === 'number' || (typeof v === 'string' && SPACING_TOKEN_NAMES.has(v))
}

/**
 * Validate an unknown value as a `LayoutDirective`. Returns null on success,
 * or a human-readable error string describing the first problem found. Call
 * at the boundary (CLI/HTTP) so bad agent input fails loudly instead of
 * silently falling through to defaults.
 */
export function validateLayoutDirective(value: unknown): string | null {
  if (!value || typeof value !== 'object') return 'layout: expected an object'
  const d = value as Record<string, unknown>
  if (d.kind !== 'row' && d.kind !== 'column' && d.kind !== 'grid') {
    return `layout.kind: expected 'row' | 'column' | 'grid', got ${JSON.stringify(d.kind)}`
  }
  for (const key of ['gap', 'rowGap', 'colGap'] as const) {
    if (d[key] !== undefined && !isSpacingValue(d[key])) {
      return `layout.${key}: expected number or one of ${[...SPACING_TOKEN_NAMES].join('|')}, got ${JSON.stringify(d[key])}`
    }
  }
  if (d.cols !== undefined && (typeof d.cols !== 'number' || !Number.isInteger(d.cols) || d.cols < 1)) {
    return `layout.cols: expected positive integer, got ${JSON.stringify(d.cols)}`
  }
  for (const key of ['originX', 'originY'] as const) {
    if (d[key] !== undefined && typeof d[key] !== 'number') {
      return `layout.${key}: expected number, got ${JSON.stringify(d[key])}`
    }
  }
  if (d.near !== undefined && typeof d.near !== 'string') {
    return `layout.near: expected entity id string, got ${JSON.stringify(d.near)}`
  }
  if ((d.originX === undefined) !== (d.originY === undefined)) {
    return 'layout: originX and originY must be specified together'
  }
  return null
}

export interface ApplyDirectiveRequest {
  layout: LayoutDirective
  /**
   * Each item is either an `id` (re-layout an existing entity — server resolves
   * its outer footprint and data-origin insets) or a new item carrying its own
   * outer-footprint `width`/`height` (device-shell bezels included; the
   * hover-only chrome action header is reserved separately and is *not* part
   * of the footprint). `insetX`/`insetY` describe how far inside the outer
   * top-left the entity's data origin (canvasX/canvasY) sits; for un-framed
   * items pass `0` or omit. The directive lays out outer footprints with the
   * configured `gap`, then returns each position offset back into inner
   * data-origin coordinates.
   */
  items: Array<{
    id?: string
    width?: number
    height?: number
    insetX?: number
    insetY?: number
  }>
}

export interface ApplyDirectiveResult {
  positions: Array<{ canvasX: number; canvasY: number }>
  /**
   * Resolved kind for each item: the kind of the existing entity (when an
   * `id` was passed) or `null` for items being created. Lets the caller route
   * updates to the correct entity-update endpoint without forcing the agent
   * to specify `kind` for every re-layout target.
   */
  kinds: Array<CanvasEntityKind | null>
  warnings?: string[]
}

export type TaskKind = 'breakpoint_map'

export interface BreakpointMapTaskInput {
  url: string
  presets?: string[]
  label?: string
}

export interface ApplyTaskLayoutRequest {
  taskKind: TaskKind
  input: BreakpointMapTaskInput
  options?: {
    anchor?: PlacementAnchor
    focus?: boolean
  }
}

export interface ApplyTaskLayoutResponse {
  taskId: string
  taskKind: TaskKind
  groupId: string
  pageIds: string[]
  edgeIds: string[]
  resolvedPresets: string[]
  placement: PlacementResult
  warnings: string[]
}

export interface LayoutComponentStatesRequest {
  component: string
  url: string
  vary: string[]
  values?: Record<string, unknown[]>
  states?: string[]
  tokens?: Record<string, string>
  selector?: string
  anchor?: PlacementAnchor
  focus?: boolean
  label?: string
}

export interface LayoutComponentStatesResponse {
  taskId: string
  groupId: string
  pageIds: string[]
  placement: PlacementResult
  warnings: string[]
}

export interface DeletePagesRequest {
  pageIds: string[]
  focusAfter?: boolean
}

export interface DeletePagesResponse {
  deletedPageIds: string[]
  deletedEdgeIds: string[]
  deletedGroupIds: string[]
  missingPageIds: string[]
  warnings: string[]
}

export interface DeleteGroupsRequest {
  groupIds: string[]
  deleteMemberPages?: boolean
  focusAfter?: boolean
}

export interface DeleteGroupsResponse {
  deletedGroupIds: string[]
  deletedPageIds: string[]
  deletedEdgeIds: string[]
  missingGroupIds: string[]
  warnings: string[]
}

export interface CreatePagesRequest {
  pages: PageConfig[]
}

export interface CreatePagesResponse {
  pageIds: string[]
}

export interface CreateEdgesRequest {
  edges: Array<Omit<WorkspaceEdge, 'id'> & { id?: string }>
}

export interface CreateEdgesResponse {
  edgeIds: string[]
}

// --- Electron API Interfaces (exposed via contextBridge) ---

export interface ToolbarElectronAPI {
  zoomIn: () => void
  zoomOut: () => void
  zoomReset: () => void
  zoomSet: (level: number) => void
  navigateSelection: (url: string) => void
  goBackSelection: () => void
  goForwardSelection: () => void
  reloadSelection: () => void
  setTool: (tool: Tool) => void
  reloadApp: () => void
  toggleTheme: () => void
  getInitialData: () => Promise<ThemeBootstrapData>
  toggleLeftSidebar: () => void
  toggleDevTools: () => void
  toggleBrowserMode: () => void
  dropdownOpen: () => void
  dropdownClose: () => void
  setTextEditing: (active: boolean) => void
  onZoomChanged: (callback: (value: number) => void) => () => void
  onSelectionChanged: (callback: (data: ToolbarSelectionData) => void) => () => void
  onLeftSidebarChanged: (callback: (open: boolean) => void) => () => void
  onDevtoolsChanged: (callback: (open: boolean) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
  onAgentPresenceChanged: (callback: (cursors: AgentPresenceCursor[]) => void) => () => void
  onFocusAddressBar: (callback: () => void) => () => void
  repoList: () => Promise<ConnectedRepo[]>
  repoConnectViaPicker: () => Promise<ConnectedRepo | null>
  repoDisconnect: (id: string) => Promise<void>
  onReposChanged: (callback: (repos: ConnectedRepo[]) => void) => () => void
}

export interface CanvasBgElectronAPI {
  canvasZoom: (deltaY: number, mouseX: number, mouseY: number) => void
  canvasPan: (deltaX: number, deltaY: number) => void
  canvasPanTo: (x: number, y: number) => void
  /** Subscribe to main's canvas-selection-overlay broadcast (marquee rect). */
  onSelectionOverlayChanged: (
    callback: (overlay: SelectionOverlayPayload | null) => void,
  ) => () => void
  setSelectionOverlayRect: (
    overlay: SelectionOverlayPayload | null,
  ) => void
  canvasSelectInRect: (rect: WorkspaceBounds, modifiers?: SelectionModifiers) => void
  canvasSelectInScreenRect: (rect: WorkspaceBounds, modifiers?: SelectionModifiers) => void
  canvasDeselect: (modifiers?: SelectionModifiers) => void
  canvasClickAt: (
    screenX: number,
    screenY: number,
    modifiers?: SelectionModifiers,
  ) => void
  clearAnnotateHover: () => void
  selectPage: (pageId: string, modifiers?: SelectionModifiers) => void
  selectBrowserTab: (pageId: string) => void
  addBrowserPage: (presetIndex: number | 'custom') => void
  navigatePage: (pageId: string, url: string) => void
  goBackPage: (pageId: string) => void
  goForwardPage: (pageId: string) => void
  reloadPage: (pageId: string) => void
  setPageCustom: (pageId: string) => void
  setBrowserSizeMode: (pageId: string, mode: 'fill' | 'device') => void
  updatePageBounds: (pageId: string, patch: { width?: number; height?: number; canvasX?: number; canvasY?: number }) => void
  placePendingEntity: (canvasX: number, canvasY: number) => void
  setTool: (tool: Tool) => void
  setToolDefault: (patch: import('./tool-defaults').ToolDefaultPatch) => void
  startDragPage: (pageId: string, selection?: CanvasDragStartSelection) => void
  dragPage: (pageId: string, dx: number, dy: number, shiftKey?: boolean) => void
  endDragPage: () => void
  dragCopyPage: (pageId: string, canvasX: number, canvasY: number) => void
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
  tidySelectedEntities: () => void
  reorderStack: (
    action: 'bring-forward' | 'send-backward' | 'bring-to-front' | 'send-to-back',
    targetId?: string,
  ) => void
  createTextEntity: (canvasX: number, canvasY: number, text?: string, color?: string) => void
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
  commitRegionSelect: (canvasRect: WorkspaceBounds) => void
  /** Comment tool click below the drag threshold. Main resolves the page +
   *  element under the window-coord point and either fires
   *  `annotate-element-selected` (element anchor) or
   *  `comment-canvas-point-committed` (no page hit / no element). ADR 0006. */
  commitCommentClickAt: (windowX: number, windowY: number) => void
  createAnnotation: (request: AnnotationCreateRequest) => void
  createDrawing: (input: { canvasX: number; canvasY: number; width: number; height: number; strokes: AnnotationDrawingStroke[] }) => void
  selectEntities: (entityIds: string[]) => void
  resizeMultiSelection: (entries: Array<{ id: string; kind: 'page' | 'text' | 'file' | 'drawing' | 'shape'; width: number; height: number; canvasX: number; canvasY: number }>) => void
  deleteSelection: () => void
  moveAnnotation: (annotationId: string, dx: number, dy: number) => void
  addAnnotationReply: (annotationId: string, text: string) => void
  resolveAnnotation: (annotationId: string) => void
  deleteAnnotation: (annotationId: string) => void
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
  createEdge: (fromEntityId: string, toEntityId: string, fromSide?: EdgeSide, toSide?: EdgeSide) => void
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
  readNoteFile: (filePath: string) => Promise<string | null>
  writeNoteFile: (filePath: string, content: string) => Promise<boolean>
  renameNoteFile: (filePath: string, newName: string) => Promise<string | null>
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
  onFixProgressUpdate: (
    callback: (data: LayoutUpdateData['fixProgress']) => void,
  ) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}

export type CanvasDragStartSelection = {
  entityKind: CanvasEntityKind
  preserveSelection?: boolean
}

/** Payload for `forwardWheelToPage` — kept in shared/types so the renderer
 *  can build it without reaching into main code. Coordinates are in window
 *  space (`event.clientX`, `event.clientY + canvasOrigin.y`). */
export type ForwardWheelPayload = {
  windowX: number
  windowY: number
  deltaX: number
  deltaY: number
  hasPreciseScrollingDeltas: boolean
  canScroll: boolean
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}

/** Payload for `forwardPointerToPage`. Window-space coords; the main-side
 *  helper subtracts the page WCV's origin before dispatching. */
export type ForwardPointerPayload = {
  kind: 'down' | 'up' | 'move'
  windowX: number
  windowY: number
  button: 'left' | 'middle' | 'right'
  buttons?: number
  clickCount?: number
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}

interface FloatingUiElectronAPI {
  navigatePage: (pageId: string, url: string) => void
  goBackPage: (pageId: string) => void
  goForwardPage: (pageId: string) => void
  reloadPage: (pageId: string) => void
  setPagePreset: (pageId: string, index: number) => void
  setPageCustom: (pageId: string) => void
  duplicatePage: (pageId: string) => void
  toggleLinkedPage: (pageId: string) => void
  deletePage: (pageId: string) => void
  updateTextEntity: (id: string, patch: { color?: string }) => void
  duplicateTextEntity: (id: string) => void
  deleteTextEntity: (id: string) => void
  deleteDrawingEntity: (id: string) => void
  startDragEntity: (entityId: string) => void
  dragEntity: (entityId: string, dx: number, dy: number) => void
  endDragEntity: () => void
  setTextEditing: (active: boolean) => void
  dropdownOpen: () => void
  dropdownClose: () => void
  onCloseDropdown: (callback: () => void) => () => void
  getInitialData: () => Promise<FloatingUiBootstrapData>
  onFloatingUiUpdate: (callback: (data: FloatingUiUpdatePayload) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}

interface AboveViewElectronAPI {
  canvasZoom: (deltaY: number, mouseX: number, mouseY: number) => void
  canvasPan: (deltaX: number, deltaY: number) => void
  onSelectionOverlayChanged: (
    callback: (overlay: SelectionOverlayPayload | null) => void,
  ) => () => void
}

interface InteractionOverlayElectronAPI {
  canvasZoom: (deltaY: number, mouseX: number, mouseY: number) => void
  canvasPan: (deltaX: number, deltaY: number) => void
  onSelectionOverlayChanged: (
    callback: (overlay: SelectionOverlayPayload | null) => void,
  ) => () => void
}

export interface DevtoolsResizeHandleElectronAPI {
  devtoolsResizeStart: (screenX: number) => void
  devtoolsResizeMove: (screenX: number) => void
  devtoolsResizeEnd: () => void
  getInitialData: () => Promise<ThemeBootstrapData>
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}

export interface LeftSidebarElectronAPI {
  revealPage: (pageId: string) => void
  revealEntity: (entityId: string, entityKind: CanvasEntityKind) => void
  deleteEntity: (entityId: string, entityKind: CanvasEntityKind) => void
  revealGroup: (groupId: string) => void
  ungroupGroup: (groupId: string) => void
  selectTab: (tabId: string) => void
  createTab: () => void
  renameTab: (tabId: string, name: string) => void
  renamePage: (pageId: string, name: string) => void
  renameGroup: (groupId: string, name: string) => void
  renameFileEntity: (entityId: string, name: string) => void
  renameTextEntity: (entityId: string, name: string) => void
  renameDrawingEntity: (entityId: string, name: string) => void
  duplicateTab: (tabId: string) => void
  deleteTab: (tabId: string) => void
  reorderTab: (tabId: string, toIndex: number) => void
  reorderSidebarItem: (
    section: SidebarSectionKey,
    draggedId: string,
    anchorId: string | null,
    position: 'before' | 'after',
    parentId: string | null,
  ) => void
  deletePage: (pageId: string) => void
  setTabExpanded: (tabId: string, expanded: boolean) => void
  setTextEditing: (active: boolean) => void
  toggleBrowserMode: () => void
  getInitialData: () => Promise<LeftSidebarBootstrapData>
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
  onSidebarData: (callback: (data: LeftSidebarData) => void) => () => void
}

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
  updateTextEntity: (id: string, patch: { color?: string; textSize?: number }) => void
  duplicateTextEntity: (id: string) => void
  deleteTextEntity: (id: string) => void
  updateFileEntity: (id: string, patch: { objectFit?: FileObjectFit }) => void
  duplicateFileEntity: (id: string) => void
  deleteFileEntity: (id: string) => void
  setFilePreset: (fileId: string, presetIndex: number) => void
  setFileCustom: (fileId: string) => void
  setFileDeviceOrientation: (fileId: string, orientation: string) => void
  toggleFileDeviceShell: (fileId: string) => void
  deleteDrawingEntity: (id: string) => void
  updateShapeEntity: (id: string, patch: { shapeKind?: ShapeKind; text?: string; color?: string; strokeWidth?: number; textSize?: number; theme?: string; width?: number; height?: number; canvasX?: number; canvasY?: number }) => void
  deleteShapeEntity: (id: string) => void
  updateEdge: (id: string, patch: { fromEnd?: EdgeEnd; toEnd?: EdgeEnd; fromSide?: EdgeSide; toSide?: EdgeSide; color?: string; label?: string }) => void
  deleteEdge: (id: string) => void
  setPagePreset: (pageId: string, presetIndex: number) => void
  setPageCustom: (pageId: string) => void
  setDeviceOrientation: (pageId: string, orientation: string) => void
  toggleDeviceShell: (pageId: string) => void
  toggleSvgDeviceShell: (pageId: string) => void
  navigatePage: (pageId: string, url: string) => void
  goBackPage: (pageId: string) => void
  goForwardPage: (pageId: string) => void
  reloadPage: (pageId: string) => void
  duplicatePage: (pageId: string) => void
  toggleLinkedPage: (pageId: string) => void
  deletePage: (pageId: string) => void
  openBrowserDevTools: () => void
  closeBrowserDevTools: () => void
  getInitialData: () => Promise<ThemeBootstrapData>
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
  onPanelData: (callback: (data: DevtoolsPanelData) => void) => () => void
}

// --- Annotations ---

export type AnnotationAnchor =
  | { type: 'canvas'; canvasX: number; canvasY: number }
  | { type: 'page'; pageId: string; offsetX: number; offsetY: number }
  | { type: 'element'; pageId: string; selector: string; elementPath?: string; boundingBox?: DevtoolsPanelDomRect }
  | { type: 'region'; canvasRect: WorkspaceBounds }

export type AnnotationStatus = 'pending' | 'acknowledged' | 'resolved' | 'dismissed'
export type AnnotationStatusFilter = AnnotationStatus | 'unresolved' | 'all'

export interface AnnotationReply {
  author: 'user' | 'agent'
  text: string
  timestamp: string
}

export interface AnnotationDrawingPoint {
  x: number
  y: number
}

export interface AnnotationDrawingStroke {
  id: string
  color: string
  width: number
  points: AnnotationDrawingPoint[]
  brushType?: DrawingBrushType
}

export interface AnnotationDrawing {
  version: 1
  bounds: { x: number; y: number; width: number; height: number }
  strokes: AnnotationDrawingStroke[]
}

export interface AnnotationElementSelectionPayload {
  pageId: string
  nodeId: string
  id: string
  timestamp: number
  tagName: string
  name: string
  role?: string
  elementPath: string
  fullPath: string
  cssClasses: string[]
  textPreview?: string
  nearbyText?: string
  nearbyElements: string[]
  accessibility: string[]
  attributes: DevtoolsPanelDomAttribute[]
  computedStyles: string[]
  boundingBox?: DevtoolsPanelDomRect
  position?: {
    viewportXPercent: number
    documentY: number
    isFixed: boolean
  }
  sourceLocation?: SourceLocation
}

export interface AnnotationInspectContext
  extends Omit<AnnotationElementSelectionPayload, 'pageId'> {
  pageId: string
  reactComponents?: string[]
  sourceLocation?: SourceLocation
}

export interface RegionComponentGroup {
  pageId: string
  pageName: string
  components: {
    name: string
    sourceLocation?: { file: string; line?: number; column?: number }
    count: number
  }[]
}

export interface RegionElementGroup {
  pageId: string
  pageName: string
  elements: unknown[]
}

export interface AnnotationMetadata extends Record<string, unknown> {
  inspectContext?: AnnotationInspectContext
  /** Human-readable page label, e.g. "iPad Mini 768×1024" */
  pageName?: string
  /** Canonical page URL (hash removed) associated with the annotation anchor. */
  pageUrl?: string
  /** Base64-encoded PNG screenshot of the selected region. */
  regionScreenshot?: string
  /** React components found in the selected region, grouped by page. */
  regionComponents?: RegionComponentGroup[]
  /** DOM elements found within the selected region, grouped by page. */
  regionElements?: RegionElementGroup[]
  /** Who resolved this annotation, when status === 'resolved'. */
  resolvedBy?: 'user' | 'agent'
}

// --- Origin bindings (derived view from ConnectedRepo.boundOrigins) ---

export interface OriginBinding {
  repoPath: string
  autoFix: boolean
}

export type OriginBindings = Record<string, OriginBinding>

// --- Fix config (model + permissions for the Claude subprocess) ---

export type FixModel = 'opus' | 'sonnet' | 'haiku'
export type FixPermissions = 'dangerously' | 'default'

export interface FixConfig {
  model: FixModel
  permissions: FixPermissions
  configured: boolean
}

// --- Fix progress (live stream of `claude -p` events per annotation) ---

export type FixProgressEventKind =
  | 'system'
  | 'text'
  | 'tool_use'
  | 'tool_result'
  | 'result'
  | 'stderr'
  | 'error'

export interface FixProgressEvent {
  kind: FixProgressEventKind
  text: string
  timestamp: string
}

export type FixProgressStatus = 'running' | 'completed' | 'failed'

export interface FixProgressEntry {
  annotationId: string
  origin: string
  startedAt: string
  updatedAt: string
  status: FixProgressStatus
  events: FixProgressEvent[]
  summary?: string
  shouldResolve?: boolean
  error?: string
}

export interface Annotation {
  id: string
  anchor: AnnotationAnchor
  author: 'user' | 'agent'
  text: string
  status: AnnotationStatus
  replies: AnnotationReply[]
  createdAt: string
  /** Element-anchored annotations only (ADR 0013 §6). User-curated label
   *  like "Submit button" or "Hero CTA", displayed in the composer and thread.
   *  Canvas-point and region anchors leave this undefined. */
  elementName?: string
  metadata?: AnnotationMetadata
}

export interface AnnotationCreateRequest {
  anchor: AnnotationAnchor
  author?: 'user' | 'agent'
  text: string
  elementName?: string
  metadata?: AnnotationMetadata
}

// --- Comment-tool page-paints contract (ADR 0006) ---

/**
 * Per-page snapshot of the comment tool's pointer state. Main fans out one of
 * these to every page on the canvas (~60 Hz) while the comment tool is
 * active. The page paints a single-element outline when `pointer` is set and
 * `regionRect` is null; outlines for every intersecting element when
 * `regionRect` is set; nothing when `active === false`. All coords are in the
 * page's own viewport space (page-local CSS pixels).
 */
export interface CommentToolPagePreviewState {
  active: boolean
  pointer: { x: number; y: number } | null
  regionRect: { x: number; y: number; width: number; height: number } | null
}

/** Live-bbox subscription request: identifies an element-anchored annotation
 *  whose popover/composer is currently visible and needs scroll-tracked
 *  positioning. Sent renderer → main → target page on subscription churn. */
export interface AnnotationBboxSubscription {
  annotationId: string
  selector: string
}

/** Live-bbox response from a page. `boundingBox` is null when the selector no
 *  longer resolves (stale anchor). The renderer keeps the last-known live
 *  bbox in that case and renders a "stale" hint. */
export interface AnnotationLiveBboxUpdate {
  pageId: string
  annotationId: string
  boundingBox: DevtoolsPanelDomRect | null
}

// --- Electron API Interfaces ---
