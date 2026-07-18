import type {
  CurveDirection,
  EasingPreset,
  EasingSpec,
  MotionCandidate,
  Vec2,
} from './cursor-motion'
import type { CursorTuningParams } from './cursor-tuning'
import type { DrawingBrushType, Tool } from './tool'
import type { PageAnchor } from './page-anchor'
import type { PRESENCE_LABEL_KEYS } from './presence-label-keys'

export type { DrawingBrushType, Tool } from './tool'
export type { PageAnchor } from './page-anchor'
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
  syncId?: string | null
  suppressInitialNavigationBroadcast?: boolean
  source?: WorkspacePageSource
  parentGroupId?: string
  groupId?: string
  metadata?: Record<string, unknown>
  /** Optional — absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
}

// --- Generic Canvas Entity Types ---

export type CanvasEntityKind = 'page' | 'text' | 'file' | 'group' | 'edge' | 'drawing' | 'shape'

import type { ShapeKind } from './shapes'
export type { ShapeKind }

/** Shape border rendering: a drawn outline, a dashed outline, or no outline. */
export type ShapeBorderStyle = 'solid' | 'dashed' | 'none'

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
  | { kind: 'reordering-row'; ids: string[]; movingId: string; dropIndex: number; axis: 'x' | 'y' }
  // Dragging a gap handle. `gap` is the live canvas-space gap; move ticks
  // update only this field (no doc writes — §6 I5) and the renderer previews
  // the positions of `entityIds` from it. Commit writes once: the managed
  // group's `layoutGap` when `groupId` is set, just the entities' positions
  // for a loose selection (`groupId` null).
  | { kind: 'resizing-gap'; groupId: string | null; entityIds: string[]; gap: number; axis: 'x' | 'y' }

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
  canvasX: number
  canvasY: number
  width: number
  height: number
  presetIndex: number
  /** True when this page shares a live sync set (has at least one peer). */
  synced: boolean
  /** The page's sync-set id, so a selection can tell "all one set" from "each in some set". */
  syncId: string | null
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
  /** Optional — absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
  /** Page's absolute scroll offset in raw CSS pixels, default 0. Document
   *  coordinates minus this are viewport coordinates (see phase 2). */
  scrollX: number
  scrollY: number
  /** Live document positions of the DOM selectors this page's anchored items
   *  reference, keyed by selector (ADR 0030 element attachment). Present only
   *  when the page is tracking at least one element. The renderer applies them
   *  as a render-time correction to page-anchored region `docRect`s, the same
   *  correction main applies to canvas-space consumers (page-anchor-scroll.ts).
   *  Ephemeral — never persisted. */
  elementPositions?: Record<
    string,
    { docX: number; docY: number; viewportPositioned?: boolean }
  >
}

export type FocusPresentationMode = 'device' | 'fit' | 'fill'

export interface FocusPresentationData {
  pageId: string
  mode: FocusPresentationMode
  authoredLabel: string
  authoredWidth: number
  authoredHeight: number
  effectiveWidth: number
  effectiveHeight: number
  /** Annotations shown over the focused content. Starts off; latched on by a
   *  working tool or the focus-bar eye (ADR 0021). */
  annotationsVisible: boolean
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
  /** Apparent position: for page-anchored text the scroll-follow shift is
   *  already applied (see shared/page-anchor.ts `scrollX/scrollY`). */
  canvasX: number
  canvasY: number
  width: number
  height: number
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  parentGroupId?: string
  /** Present when the text is hooked to a page (see shared/page-anchor.ts).
   *  The renderer clips/fades it inside that page's overlay band. */
  pageAnchor?: PageAnchor
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
   * Markdown note content, present only once the note has entered the
   * Y.Doc `notes` mirror (i.e. edited at least once — ADR 0023). Undefined
   * means the renderer should fall back to reading the `.md` file directly;
   * once defined, this scene field is the source of truth and reflects
   * undo/redo immediately on the next broadcast.
   */
  noteContent?: string
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
  /** Whether the resolved renderer hosts live content (an iframe) that should
   *  get the page-like select-first / interact-second treatment: the first
   *  click selects, a second click enters interactivity so pointer/scroll
   *  reach the content. Undefined → treated as `false`. */
  rendererInteractive?: boolean
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
  /**
   * Incremented by the main-process file watcher each time the underlying file
   * changes on disk. Renderers use this as a remount key or refetch trigger so
   * the canvas updates without an app restart.
   */
  fileReloadVersion?: number
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
  /** Managed-layout packing gap in px; absent → the default gutter. */
  layoutGap?: number
  entityIds: string[]
}

export interface CanvasSceneDrawingEntity {
  kind: 'drawing'
  id: string
  /** Apparent position: for page-anchored drawings the scroll-follow shift is
   *  already applied to bounds *and* stroke points (strokes are absolute
   *  canvas coords — see shared/page-anchor.ts `scrollX/scrollY`). */
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
  /** Present when the drawing is hooked to a page (see shared/page-anchor.ts).
   *  The renderer clips/fades it inside that page's overlay band. */
  pageAnchor?: PageAnchor
}

export interface CanvasSceneShapeEntity {
  kind: 'shape'
  id: string
  shapeKind: ShapeKind
  text: string
  color?: string
  strokeWidth?: number
  /** Border line style. Absent = 'solid' (backward compat). */
  borderStyle?: ShapeBorderStyle
  /** Border color, independent of fill `color`. Absent = derive from `color`. */
  borderColor?: string
  /** Per-entity text size in px for the inner label. ADR 0013 §2. */
  textSize?: number
  theme?: string
  /** Apparent position: for page-anchored shapes the scroll-follow shift is
   *  already applied (see shared/page-anchor.ts `scrollX/scrollY`). */
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  /** Present when the shape is hooked to a page (see shared/page-anchor.ts).
   *  The renderer clips/fades the shape inside that page's overlay band. */
  pageAnchor?: PageAnchor
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
}

export interface PendingPlacement {
  entityKind: CanvasEntityKind
  presetIndex?: number
  shapeKind?: ShapeKind
  textStyle?: TextEntityStyle
  /** Stored color of the in-flight placement (sticky fill, etc.) so the preview can match the picker. */
  color?: string
  /** Text size in canvas units for plain-text placement preview. */
  textSize?: number
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

/** A page's color-scheme override. Absent means "follow system". */
export type PageColorScheme = 'light' | 'dark'

export interface PersistedPageEntity extends CanvasEntityBase {
  kind: 'page'
  name?: string
  url: string
  presetIndex: number
  syncId: string | null
  source?: WorkspacePageSource
  groupId?: string
  metadata?: Record<string, unknown>
  /** Optional — absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
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
  /** Present when the entity is hooked to a page (see shared/page-anchor.ts). */
  pageAnchor?: PageAnchor
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

export type WorkspaceGroupLayoutMode = 'freeform' | 'row' | 'column' | 'grid'

export interface PersistedGroupEntity extends CanvasEntityBase {
  kind: 'group'
  label: string
  color?: string
  width: number
  height: number
  layoutMode: WorkspaceGroupLayoutMode
  managedLayout: boolean
  /** Managed-layout packing gap in px; absent → the default gutter. */
  layoutGap?: number
  sourceTaskId?: string
  metadata?: Record<string, unknown>
}

export interface PersistedDrawingEntity extends CanvasEntityBase {
  kind: 'drawing'
  width: number
  height: number
  strokes: AnnotationDrawingStroke[]
  label?: string
  /** Present when the entity is hooked to a page (see shared/page-anchor.ts). */
  pageAnchor?: PageAnchor
}

export interface PersistedShapeEntity extends CanvasEntityBase {
  kind: 'shape'
  shapeKind: ShapeKind
  text: string
  color?: string
  strokeWidth?: number
  borderStyle?: ShapeBorderStyle
  borderColor?: string
  /** Per-entity text size in px for the inner label. ADR 0013 §2. */
  textSize?: number
  theme?: string
  width: number
  height: number
  label?: string
  /** Present when the entity is hooked to a page (see shared/page-anchor.ts). */
  pageAnchor?: PageAnchor
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
  /**
   * Wall-clock milliseconds `buildCanvasLayoutData` took to produce this
   * payload, stamped by the layout pass. Diagnostic only — feeds the canvas
   * perf HUD so the O(entities) rebuild cost is visible during pan/zoom. See
   * #257 / #265.
   */
  buildMs?: number
  windowWidth: number
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
   * cluster. Popups that anchor below the toolbar (tool-mode popups, ADR 0008
   * §1) read this to align with the tools regardless of platform padding (mac
   * traffic-lights inset) or sidebar state.
   *
   * Computed in main from `TOOLBAR_PAD_*` constants and the current window
   * width; mirrors the `grid-cols-[1fr_auto_1fr]` layout the toolbar uses.
   */
  toolbarCenterX: number
  /** Back-to-front stack order across canvas nodes and edges. */
  entityOrder: string[]
  entities: CanvasSceneEntity[]
  selectedEntityIds: string[]
  selection: CanvasSelectableTarget[]
  activeSelection: ActiveCanvasEntitySelection | null
  activeTool: Tool
  /** Per-tool persistent defaults (ADR 0008 §9). Tool-mode popup reads/writes. */
  toolDefaults: import('./tool-defaults').ToolDefaults
  annotations: Annotation[]
  inspect: InspectPanelState | null
  fixProgress: Record<string, FixProgressEntry>
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
  /** The page the user has *entered* for interaction (select-first /
   *  interact-second, #124). Only this page forwards pointer input and owns
   *  keyboard; a merely-selected page is null here. */
  interactivePageId: string | null
  /** Ephemeral focus-presentation override for the focused page, if active. */
  focusPresentation: FocusPresentationData | null
  /** Wall-clock (Date.now) start of the in-flight animated camera move, or
   *  null when idle. Lets renderer flip animations fast-forward into phase
   *  with the main-driven camera. */
  cameraTransitionStartedAt: number | null
}

export type PresenceSurface = 'canvas' | 'page'

export type PresenceActivity = 'traveling' | 'acting' | 'waiting' | 'thinking' | 'idle' | 'departing'

export type PresenceLabelKey = (typeof PRESENCE_LABEL_KEYS)[number]

export interface PresenceTargetRect {
  x: number
  y: number
  width: number
  height: number
}

export type PresenceTargetRefSource = 'specular' | 'agent-browser'

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

/**
 * A page-anchored annotation shown as a child row of its page in the sidebar
 * (the page acts as a folder for content anchored to it). Not a canvas
 * entity — annotation rows don't participate in stack-order reordering.
 */
export interface SidebarAnnotationItem {
  kind: 'annotation'
  id: string
  label: string
  /** Thread size: the root comment plus replies. */
  messageCount: number
  /** False when the page has navigated away from the annotation's URL —
   *  the canvas visuals are hidden, so the row renders dimmed. */
  onCurrentPage: boolean
}

/**
 * A canvas entity hooked to this page (shared/page-anchor.ts), shown as a
 * child row of the page in the sidebar. `onCurrentPage` is false when the
 * page has navigated away from the entity's anchor URL — the entity's canvas
 * visuals are hidden, so the row renders dimmed.
 */
export type SidebarAnchoredEntityItem = (
  | SidebarTextItem
  | SidebarFileItem
  | SidebarDrawingItem
  | SidebarShapeItem
) & { onCurrentPage: boolean }

/**
 * Content belonging to a page, shown as child rows under its sidebar row:
 * anchored canvas entities in stack order, then unresolved page-anchored
 * annotations newest first. One builder produces the list; the item kinds
 * only differ in presentation.
 */
export type SidebarPageChildItem = SidebarAnchoredEntityItem | SidebarAnnotationItem

export interface SidebarPageItem {
  kind: 'page'
  id: string
  label: string
  faviconUrl?: string | null
  width?: number
  height?: number
  /** Content anchored to this page (entities in stack order, then
   *  unresolved annotations newest first). */
  children?: SidebarPageChildItem[]
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
  hasPages: boolean
  sections: LeftSidebarSections
  items: SidebarCanvasItem[]
}

export interface ToolbarSelectionData {
  activePageId: string | null
  selectedEntityIds: string[]
  selectionCount: number
  availablePageCount: number
  activeTabId: string | null
  activeTabName: string | null
  activeTool: Tool
  /** Current draw-tool brush default — drives which glyph the Draw button shows. */
  drawBrushType: DrawingBrushType
  /** Current draw-tool color default (raw stored slot/hex) — tints the Draw glyph. */
  drawColor: string
  /** Current sticky-tool color default (raw stored slot/hex) — tints the sticky glyph. */
  stickyColor: string
  /** Current shape-tool color default (raw stored slot/hex) — tints the shape glyph. */
  shapeColor: string
}

/** App-level theme preference: 'system' follows OS appearance; 'light'/'dark' pin it. */
export type AppThemeMode = 'system' | 'light' | 'dark'

export interface ThemeData {
  isDark: boolean
  themeMode: AppThemeMode
}

export interface ThemeBootstrapData {
  theme: ThemeData
}

export interface DebugBootstrapData extends ThemeBootstrapData {
  cursorSplineViz: boolean
  cursorTuning: CursorTuningParams
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

// --- Settings window ---

export interface SettingsBootstrapData extends ThemeBootstrapData {
  status: OnboardingStatusSnapshot
  fixConfig: FixConfig
  connectedRepos: ConnectedRepo[]
}

export type {
  CursorTuningParams,
  EasingPreset,
  EasingSpec,
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
  /** Page entries only — absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
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
  /** Unique nth-of-type path — the selector to re-resolve this element by. */
  uniqueSelector?: string
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
  leftSidebarOpen: boolean
  /** True while a toolbar dropdown is open — the layout pass grows the
   *  toolbar view to full-window bounds so the menu can overflow the strip. */
  toolbarDropdownOpen: boolean
  /** True while a toolbar tooltip is open — the layout pass grows the toolbar
   *  view by a shallow band so the tip can paint just below the strip. */
  toolbarTooltipOpen: boolean
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
  syncId?: string | null
  source?: WorkspacePageSource
  parentGroupId?: string
  groupId?: string
  metadata?: Record<string, unknown>
  /** Optional — absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
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
  source: WorkspacePageSource
  parentGroupId?: string
  groupId?: string
  metadata?: Record<string, unknown>
  /** Optional — absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
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
  /** Optional — absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
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
  /** Page-specific — optional, absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
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
  /** Managed-layout packing gap in px; absent → the default gutter. */
  layoutGap?: number
  pageIds?: string[]
  entityIds?: string[]
  sourceTaskId?: string
  metadata?: Record<string, unknown>
}

export interface WorkspaceDrawingEntity {
  id: string
  kind: 'drawing'
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  label?: string
}

export interface WorkspaceShapeEntity {
  id: string
  kind: 'shape'
  shapeKind: ShapeKind
  text: string
  color?: string
  strokeWidth?: number
  textSize?: number
  theme?: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  label?: string
}

export type WorkspaceCanvasEntity =
  | WorkspacePage
  | WorkspaceTextEntity
  | WorkspaceFileEntity
  | WorkspaceGroup
  | WorkspaceDrawingEntity
  | WorkspaceShapeEntity

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

/**
 * The authoritative viewport, pushed to the overlay renderers immediately on a
 * pan/zoom — ahead of the debounced `layout-update` rebuild. The canvas scene
 * layers translate by (livePan − payloadPan) so selection chrome and entity
 * bodies track the natively-positioned page views during a pan instead of
 * waiting for the next full rebuild. See #257.
 */
export interface ViewportNudge {
  pan: { x: number; y: number }
  zoom: number
}

/**
 * Per-kind interactive update patch shapes. `updateEntity` is typed by this map
 * so a text patch sent with `kind: 'shape'` is a compile error at the call
 * site. Each patch is the union of every field the interactive path forwards
 * for that kind; the registry `update` honors each (see
 * `src/main/entities/builtin/*.ts`).
 */
export interface EntityUpdatePatchMap {
  text: { text?: string; color?: string; textSize?: number; width?: number; height?: number; canvasX?: number; canvasY?: number; widthMode?: TextWidthMode }
  file: { width?: number; height?: number; canvasX?: number; canvasY?: number; objectFit?: FileObjectFit }
  drawing: { width?: number; height?: number; canvasX?: number; canvasY?: number; strokes?: AnnotationDrawingStroke[] }
  shape: { shapeKind?: ShapeKind; text?: string; color?: string; strokeWidth?: number; borderStyle?: ShapeBorderStyle; borderColor?: string; textSize?: number; theme?: string; width?: number; height?: number; canvasX?: number; canvasY?: number }
  group: { width?: number; height?: number; canvasX?: number; canvasY?: number; label?: string; color?: string }
}

/** The entity kinds reachable through the generic `canvas-update-entity` channel. */
export type UpdatableEntityKind = keyof EntityUpdatePatchMap
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

// --- Annotations ---

export type AnnotationAnchor =
  | { type: 'canvas'; canvasX: number; canvasY: number }
  | { type: 'page'; pageId: string; offsetX: number; offsetY: number }
  | { type: 'element'; pageId: string; selector: string; elementPath?: string; boundingBox?: DevtoolsPanelDomRect }
  // Region annotations split by the grab rule. A grab-less marquee marks
  // canvas space and stores `canvasRect`; a marquee that grabbed page content
  // is page-anchored and stores `docRect` in the page's document CSS pixels,
  // relative to the page named by `Annotation.pageAnchor` (ADR 0029 — the
  // pageAnchor is the single source of truth for which page). A region
  // without `docRect` (all existing files) is canvas-anchored, full stop.
  // Narrow the two arms with `'docRect' in anchor` after `type === 'region'`.
  | { type: 'region'; canvasRect: WorkspaceBounds }
  | { type: 'region'; docRect: WorkspaceBounds }

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
  /** Unique nth-of-type path — the selector to re-resolve this element by. */
  uniqueSelector?: string
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
  /** Human-readable page label, e.g. "iPad Mini 768×1024". Display context
   *  only — the page binding lives in `Annotation.pageAnchor`. */
  pageName?: string
  /** Base64-encoded PNG screenshot of the selected region. */
  regionScreenshot?: string
  /** React components found in the selected region, grouped by page. */
  regionComponents?: RegionComponentGroup[]
  /** DOM elements found within the selected region, grouped by page. */
  regionElements?: RegionElementGroup[]
  /** Who resolved this annotation, when status === 'resolved'. */
  resolvedBy?: 'user' | 'agent'
  /**
   * Claude Code session id for this thread's fix conversation. Set after the
   * first fix; subsequent replies resume the same session (`claude --resume`)
   * so the agent keeps its prior context instead of starting cold.
   */
  fixSessionId?: string
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
  /** Present when the annotation is bound to a page's document (see
   *  shared/page-anchor.ts) — the ONLY page-binding read. Written at
   *  creation: element/page anchors from their anchor page; region anchors
   *  iff the marquee grabbed page content; canvas points never. Annotations
   *  without one are canvas-bound: they never hide and never travel. */
  pageAnchor?: PageAnchor
  metadata?: AnnotationMetadata
}

export interface AnnotationCreateRequest {
  anchor: AnnotationAnchor
  author?: 'user' | 'agent'
  text: string
  elementName?: string
  metadata?: AnnotationMetadata
  /** Explicit page binding for a region anchor, decided by the caller (e.g.
   *  region select binds geometrically: majority of the marquee over the page
   *  body). When absent, region binding falls back to the grab rule. */
  anchorPageId?: string
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
  /** Counter-scale for the in-page hover label (= 1 / displayZoom). Keeps the
   *  label a constant on-screen size despite the webview being zoom-scaled, so
   *  it matches the screen-space inspect popover. */
  displayScale?: number
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

/** Element-attachment reflow tracking (ADR 0030). Main declares, per page, the
 *  distinct DOM selectors that anchored items reference; the page resolves them
 *  to document positions and reports back on reflow. Unlike the bbox
 *  subscription this is a plain declaration (no per-item id) — items sharing a
 *  selector share one subscription, and the position is keyed by selector. */
export interface ElementAttachmentSubscriptions {
  selectors: string[]
}

/** One resolved element document position (scroll-invariant): the element's
 *  top-left in document space, matching the capture convention
 *  (`rect.left + scrollX`, `rect.top + scrollY`). */
export interface ElementAttachmentPosition {
  selector: string
  /** False when the selector no longer resolves. Main removes any cached live
   * position so render correction falls back to stored geometry. */
  resolved: false
  docX?: never
  docY?: never
  viewportPositioned?: never
}

export interface ResolvedElementAttachmentPosition {
  selector: string
  resolved?: true
  docX: number
  docY: number
  viewportPositioned?: boolean
}

/** Batched reflow report (page → main). Resolution loss is explicit so main
 *  can discard stale correction while keeping the item itself visible. */
export interface ElementAttachmentPositionsUpdate {
  positions: Array<ElementAttachmentPosition | ResolvedElementAttachmentPosition>
}

// --- Electron API Interfaces ---
