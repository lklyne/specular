// fallow-ignore-file circular-dependencies
// Suppressed: see #141. central layout data module is in cycles with selection-controller and layout-engine
/**
 * Canvas layout data builders — payload construction for renderer surfaces.
 *
 * Builds view models consumed by the canvas background, toolbar,
 * left sidebar, and annotation overlay renderers.
 */

import type {
  ActiveCanvasEntitySelection,
  AgentPresenceCursor,
  Annotation,
  CanvasSceneEntity,
  CanvasScenePageEntity,
  CanvasSceneGroupEntity,
  ComponentTreeNode,
  LayoutUpdateData,
  PendingPlacement,
  ToolbarSelectionData,
} from '../../shared/types'
import { resolvePresencePagePoint } from '../../shared/presence-targeting'
import { isUnresolved } from '../../shared/annotation-utils'
import {
  aboveView,
  cursorOverlayWindow,
  leftSidebarView,
  win,
} from './view-refs'
import { safeSend } from './safe-send'
import { layoutCache } from './layout-cache'
import {
  findPageById,
  hoverTarget,
  interactionState,
  pages,
  pan,
  selectedPage,
  selectedPageId,
  zoom,
} from './runtime-context'
import { activeWorkspaceTabId, workspaceAnnotations, workspaceEdges, workspaceGroups } from './workspace-model'
import { getToolDefaults } from './tool-defaults'
import {
  activeBrowserPageId as uiActiveBrowserPageId,
  activeTool as uiActiveTool,
  devtoolsOpen as uiDevtoolsOpen,
  devtoolsWidth as uiDevtoolsWidth,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedCanvasTargets as uiSelectedCanvasTargets,
  selectedEntityIds as uiSelectedEntityIds,
  selectedGroupId as uiSelectedGroupId,
  workspaceViewMode as uiWorkspaceViewMode,
} from '../ui-state'
import {
  LEFT_SIDEBAR_WIDTH,
  TOOLBAR_PAD_LEFT_MAC,
  TOOLBAR_PAD_LEFT_OTHER,
  TOOLBAR_PAD_RIGHT_MAC,
  TOOLBAR_PAD_RIGHT_OTHER,
} from './runtime-constants'
import { currentKeyboardTargetPageId } from './selection-controller'
import {
  pageBodyCanvasBounds,
  pageContentSize,
  boundEffectivePageContentSize as effectivePageContentSize,
  boundAvailableCanvasViewport as localAvailableCanvasViewport,
  boundCanvasOrigin as localCanvasOrigin,
  boundFillBrowserViewportSize as localFillBrowserViewportSize,
  boundScreenBoundsForPage as screenBoundsForPage,
} from './runtime-geometry'
import { pageDisplayLabel, viewportPresetForIndex } from './runtime-serialization'
import {
  textEntities,
  buildTextEntitySceneEntity,
  DEFAULT_TEXT_WIDTH,
  DEFAULT_TEXT_HEIGHT,
} from './text-entity-state'
import {
  pageUsesCustomSize,
  pageBrowserSizeModeFromMetadata,
  deviceIdFromMetadata,
  deviceOrientationFromMetadata,
  showDeviceFrameFromMetadata,
  useSvgDeviceShellFromMetadata,
} from './runtime-entities'
import {
  fileEntities,
  buildFileEntitySceneEntity,
  DEFAULT_FILE_WIDTH,
  DEFAULT_FILE_HEIGHT,
} from './file-entity-state'
import {
  drawingEntitiesForUi,
  buildDrawingEntitySceneEntity,
} from './drawing-entity-state'
import {
  shapeEntities,
  buildShapeEntitySceneEntity,
  defaultShapeSize,
} from './shape-entity-state'
import { buildGroupSceneEntity } from './group-entity-state'
import type { Page } from './runtime-entities'
import { workspaceTabSummaries } from './workspace-tabs'
import { getPresenceCursors } from '../presence-cursor'
import { getFixProgress } from '../agent-fix/fix-progress'
import { DOC_ARRAY_ENTITY_ORDER, getActiveDoc } from './workspace-doc'

// --- Exported data builders ---

export function backgroundPageOverlays(): CanvasScenePageEntity[] {
  const viewMode = uiWorkspaceViewMode()
  const activeBrowserPageId = viewMode === 'browser' ? uiActiveBrowserPageId() : null
  return pages.map((page) => {
    const { width, height } = effectivePageContentSize(page)
    const bounds = screenBoundsForPage(page)
    const deviceId = deviceIdFromMetadata(page.metadata)
    // In browser mode, only show the device shell for the active tab
    const showShell = showDeviceFrameFromMetadata(page.metadata)
      && (viewMode === 'canvas' || page.id === activeBrowserPageId)
    return {
      kind: 'page' as const,
      id: page.id,
      label: pageDisplayLabel(page),
      faviconUrl: page.faviconUrl ?? null,
      url: page.url,
      canGoBack: page.pageView.webContents.canGoBack(),
      canGoForward: page.pageView.webContents.canGoForward(),
      isLoading: page.pageView.webContents.isLoading(),
      isCustomSize: pageUsesCustomSize(page.metadata),
      browserSizeMode: viewMode === 'canvas' ? 'device' : pageBrowserSizeModeFromMetadata(page.metadata),
      canvasX: page.canvasX,
      canvasY: page.canvasY,
      width,
      height,
      presetIndex: page.presetIndex,
      linked: page.linked,
      screenX: showShell ? bounds.shell.x : bounds.page.x,
      screenY: showShell ? bounds.shell.y : bounds.page.y,
      screenWidth: showShell ? bounds.shell.width : bounds.page.width,
      screenHeight: showShell ? bounds.shell.height : bounds.page.height,
      // Device state
      deviceId,
      deviceOrientation: deviceOrientationFromMetadata(page.metadata),
      showDeviceFrame: showShell,
      // Inner content bounds (always the web viewport)
      contentScreenX: bounds.page.x,
      contentScreenY: bounds.page.y,
      contentScreenWidth: bounds.page.width,
      contentScreenHeight: bounds.page.height,
      useSvgDeviceShell: useSvgDeviceShellFromMetadata(page.metadata),
    }
  })
}

function buildLiveBrowserTabSummaries() {
  return pages.map((page) => {
    const { width, height } = pageContentSize(page)
    return {
      id: page.id,
      label: pageDisplayLabel(page),
      name: page.name?.trim() || undefined,
      url: page.url,
      presetIndex: page.presetIndex,
      faviconUrl: page.faviconUrl ?? null,
      width,
      height,
    }
  })
}

export function activeCanvasSelection(): ActiveCanvasEntitySelection | null {
  const selectedPageIds = uiSelectedEntityIds()
  const targets = selectedPageIds
    .map((id) => findPageById(id))
    .filter((p): p is Page => p !== undefined)
  const page = selectedPage() ?? targets[0] ?? null
  if (!page) return null
  const vp = viewportPresetForIndex(page.presetIndex)
  return {
    entityRef: { kind: 'page', id: page.id },
    label: pageDisplayLabel(page),
    width: page.peekWidth ?? vp.width,
    height: page.peekHeight ?? vp.height,
    presetIndex: page.presetIndex,
    linked: targets.length > 0 ? targets.every((target) => target.linked) : page.linked,
  }
}


function canonicalAnnotationUrl(value: string | undefined | null): string | null {
  if (!value) return null
  const trimmed = value.trim()
  if (!trimmed) return null
  try {
    const parsed = new URL(trimmed)
    parsed.hash = ''
    return parsed.toString()
  } catch {
    return trimmed
  }
}

export function annotationsForPage(pageId: string): Annotation[] {
  const page = findPageById(pageId)
  const currentPageUrl = canonicalAnnotationUrl(page?.pageView.webContents.getURL() ?? null)
  return workspaceAnnotations.filter((annotation) => {
    if (!isUnresolved(annotation.status)) return false
    if (annotation.anchor.type === 'canvas') return false
    if (annotation.anchor.type === 'region') return false
    if (annotation.anchor.pageId !== pageId) return false
    const annotationPageUrl = canonicalAnnotationUrl(annotation.metadata?.pageUrl)
    if (!annotationPageUrl || !currentPageUrl) return true
    return annotationPageUrl === currentPageUrl
  })
}

export function pageAnnotationsKey(annotations: Annotation[]): string {
  return annotations
    .map((annotation) => {
      const repliesKey = annotation.replies
        .map((reply) => [reply.author, reply.timestamp, reply.text].join('~'))
        .join(',')
      return [annotation.id, annotation.author, annotation.status, annotation.text, repliesKey].join(':')
    })
    .join('|')
}

export function selectedComponentTreePayload():
  | { pageId: string; tree: ComponentTreeNode[] }
  | null {
  const selectedPageIds = uiSelectedEntityIds()
  if (selectedPageIds.length !== 1) return null
  const pageId = selectedPageIds[0]
  const page = findPageById(pageId)
  if (!page) return null
  return { pageId, tree: page.componentTree ?? [] }
}

export function sendAnnotationLayoutUpdate(data: {
  pages: CanvasScenePageEntity[]
  activeSelection: ActiveCanvasEntitySelection | null
}): void {
  const payload = buildCanvasLayoutData(data.pages, data.activeSelection)
  if (aboveView) safeSend(aboveView.webContents, 'layout-update', payload)
  if (cursorOverlayWindow && !cursorOverlayWindow.isDestroyed()) {
    safeSend(cursorOverlayWindow.webContents, 'layout-update', payload)
  }
}

export function buildFloatingUiUpdatePayload(input: {
  pages: CanvasScenePageEntity[]
  activeSelection: ActiveCanvasEntitySelection | null
  surfaceOrigin: { x: number; y: number }
}) {
  return {
    layoutData: buildCanvasLayoutData(input.pages, input.activeSelection),
    surfaceOrigin: input.surfaceOrigin,
  }
}

function buildUserGroupSceneEntities(
  origin: { x: number; y: number },
): CanvasSceneGroupEntity[] {
  return workspaceGroups
    .map((g) => {
      const entityIds = [
        ...pages.filter((page) => page.parentGroupId === g.id).map((page) => page.id),
        ...textEntities.filter((entity) => entity.parentGroupId === g.id).map((entity) => entity.id),
        ...fileEntities.filter((entity) => entity.parentGroupId === g.id).map((entity) => entity.id),
        ...drawingEntitiesForUi().filter((entity) => entity.parentGroupId === g.id).map((entity) => entity.id),
        ...shapeEntities.filter((entity) => entity.parentGroupId === g.id).map((entity) => entity.id),
        ...workspaceGroups.filter((candidate) => candidate.parentGroupId === g.id).map((group) => group.id),
      ]
      return buildGroupSceneEntity(g, zoom, pan, origin, entityIds)
    })
}

function placementEntityKindForTool(tool: ReturnType<typeof uiActiveTool>): PendingPlacement['entityKind'] | null {
  switch (tool.kind) {
    case 'add-page':
      return 'page'
    case 'add-text':
      return 'text'
    case 'add-sticky':
      return 'text'
    case 'add-shape':
      return 'shape'
    default:
      return null
  }
}

function buildPlacementPreview(tool: ReturnType<typeof uiActiveTool>): PendingPlacement | null {
  const entityKind = placementEntityKindForTool(tool)
  if (!entityKind) return null
  const isText = entityKind === 'text'
  const isFile = entityKind === 'file'
  const isShape = entityKind === 'shape'
  const presetIndex = tool.kind === 'add-page' ? tool.presetIndex : undefined
  const textStyle =
    tool.kind === 'add-sticky'
      ? 'sticky'
      : tool.kind === 'add-text'
        ? 'plain'
        : undefined
  const color =
    tool.kind === 'add-sticky' ? getToolDefaults()['add-sticky'].color : undefined
  const customSize = tool.kind === 'add-page' ? tool.customSize === true : false
  const sourcePageId = tool.kind === 'add-page' ? tool.sourcePageId : undefined
  // shapeKind moved to tool defaults per ADR 0009 — preview reads the persisted
  // default so the user sees the variant they last picked in the popup.
  const shapeKind =
    tool.kind === 'add-shape' ? getToolDefaults()['add-shape'].shapeKind : undefined
  const sourcePage = sourcePageId ? findPageById(sourcePageId) : null
  const preset = (isText || isFile || isShape)
    ? null
    : viewportPresetForIndex(presetIndex ?? 0)
  const customDims = sourcePage ? pageContentSize(sourcePage) : localFillBrowserViewportSize()
  const shapeDefault = isShape && shapeKind ? defaultShapeSize(shapeKind) : null
  return {
    entityKind,
    presetIndex,
    shapeKind,
    textStyle,
    color,
    width: isText
      ? DEFAULT_TEXT_WIDTH
      : isFile
        ? DEFAULT_FILE_WIDTH
        : shapeDefault
          ? shapeDefault.width
          : customSize
            ? customDims.width
            : (preset?.width ?? 0),
    height: isText
      ? DEFAULT_TEXT_HEIGHT
      : isFile
        ? DEFAULT_FILE_HEIGHT
        : shapeDefault
          ? shapeDefault.height
          : customSize
            ? customDims.height
            : (preset?.height ?? 0),
  }
}

export function buildCanvasLayoutData(
  pages: CanvasScenePageEntity[],
  activeSelection: ActiveCanvasEntitySelection | null,
): LayoutUpdateData {
  const fillViewport = localFillBrowserViewportSize()
  const tool = uiActiveTool()
  const viewMode = uiWorkspaceViewMode()
  const origin = localCanvasOrigin()
  const pendingPlacementData = buildPlacementPreview(tool)
  const groupEntities = buildUserGroupSceneEntities(origin)
  const windowWidth = win?.getBounds().width ?? 0
  const isMac = process.platform === 'darwin'
  const padLeft = isMac ? TOOLBAR_PAD_LEFT_MAC : TOOLBAR_PAD_LEFT_OTHER
  const padRight = isMac ? TOOLBAR_PAD_RIGHT_MAC : TOOLBAR_PAD_RIGHT_OTHER
  const toolbarCenterX = (padLeft + Math.max(0, windowWidth - padRight)) / 2
  const entities = [
    ...pages,
    ...textEntities.map((te) =>
      buildTextEntitySceneEntity(te, zoom, pan, origin)
    ),
    ...fileEntities.map((fe) =>
      buildFileEntitySceneEntity(fe, zoom, pan, origin)
    ),
    ...drawingEntitiesForUi().map((de) =>
      buildDrawingEntitySceneEntity(de, zoom, pan, origin)
    ),
    ...shapeEntities.map((se) =>
      buildShapeEntitySceneEntity(se, zoom, pan, origin)
    ),
    ...groupEntities,
  ] as CanvasSceneEntity[]
  const edges = [...workspaceEdges]
  const knownStackIds = new Set([
    ...entities.map((entity) => entity.id),
    ...edges.map((edge) => edge.id),
  ])
  const seenStackIds = new Set<string>()
  const entityOrder: string[] = []
  for (const id of getActiveDoc().getArray<string>(DOC_ARRAY_ENTITY_ORDER).toArray()) {
    if (!knownStackIds.has(id) || seenStackIds.has(id)) continue
    seenStackIds.add(id)
    entityOrder.push(id)
  }
  for (const id of knownStackIds) {
    if (seenStackIds.has(id)) continue
    seenStackIds.add(id)
    entityOrder.push(id)
  }
  const orderRank = new Map(entityOrder.map((id, index) => [id, index]))
  entities.sort((a, b) => {
    const aRank = orderRank.get(a.id)
    const bRank = orderRank.get(b.id)
    if (aRank === undefined && bRank === undefined) return 0
    if (aRank === undefined) return 1
    if (bRank === undefined) return -1
    return aRank - bRank
  })
  edges.sort((a, b) => (orderRank.get(a.id) ?? Infinity) - (orderRank.get(b.id) ?? Infinity))
  return {
    zoom,
    pan,
    canvasOrigin: origin,
    leftChromeWidth: uiLeftSidebarOpen() ? LEFT_SIDEBAR_WIDTH : 0,
    toolbarCenterX,
    entityOrder,
    entities,
    browserTabs: buildLiveBrowserTabSummaries(),
    browserFillViewport: fillViewport,
    selectedEntityIds: uiSelectedEntityIds(),
    selection: uiSelectedCanvasTargets(),
    activeSelection,
    activeTool: tool,
    toolDefaults: getToolDefaults(),
    annotations: [...workspaceAnnotations],
    fixProgress: getFixProgress(),
    viewMode,
    activeBrowserTabId:
      viewMode === 'browser'
        ? selectedPageId()
        : null,
    activeBrowserPageId: uiActiveBrowserPageId(),
    selectedGroupId: uiSelectedGroupId(),
    hover: hoverTarget,
    interaction: interactionState,
    pendingPlacement: pendingPlacementData,
    devtoolsOpen: uiDevtoolsOpen(),
    devtoolsWidth: uiDevtoolsWidth(),
    edges,
    groups: groupEntities,
    presenceCursors: getPresenceCursors()
    .filter((c) => {
      // In browser mode, hide cursors that explicitly target a different page.
      if (viewMode !== 'browser') return true
      const activePageId = uiActiveBrowserPageId()
      if (c.surface === 'page' && c.pageId && c.pageId !== activePageId) return false
      return true
    })
    .map((c): AgentPresenceCursor => ({
      ...(function resolvePresencePosition() {
        if (c.surface === 'page' && c.pageId) {
          const page = pages.find((candidate) => candidate.id === c.pageId)
          if (page) {
            const point = resolvePresencePagePoint({
              pageX: c.pageX,
              pageY: c.pageY,
              targetRect: c.targetRect ?? null,
              fallbackX: page.width / 2,
              fallbackY: page.height / 2,
            })
            // Clamp to the page's visible area so the cursor doesn't
            // render outside the page when targeting off-screen elements.
            const clampedX = Math.max(0, Math.min(point.x, page.width))
            const clampedY = Math.max(0, Math.min(point.y, page.height))
            const pageWcv = findPageById(page.id)
            const body = pageWcv ? pageBodyCanvasBounds(pageWcv) : { x: page.canvasX, y: page.canvasY }
            return {
              canvasX: body.x + clampedX,
              canvasY: body.y + clampedY,
            }
          }
        }
        // In browser mode, place canvas-surface cursors on the active page
        // so they remain visible instead of mapping to off-screen canvas coords.
        if (viewMode === 'browser') {
          const activePageId = uiActiveBrowserPageId()
          const page = activePageId
            ? pages.find((candidate) => candidate.id === activePageId)
            : null
          if (page) {
            return {
              canvasX: page.canvasX + page.width / 2,
              canvasY: page.canvasY + page.height / 2,
            }
          }
        }
        return { canvasX: c.canvasX, canvasY: c.canvasY }
      })(),
      sessionId: c.sessionId,
      clientName: c.clientName,
      color: c.color,
      surface: c.surface,
      activity: c.activity,
      pageId: c.pageId,
      pageX: c.pageX,
      pageY: c.pageY,
      labelKey: c.labelKey,
      taskLabel: c.taskLabel,
      labelHint: c.labelHint,
      labelParams: c.labelParams,
      targetRef: c.targetRef,
      targetRefSource: c.targetRefSource,
      targetName: c.targetName,
      targetRect: c.targetRect,
      updatedAt: c.updatedAt,
    })),
    keyboardTargetPageId: currentKeyboardTargetPageId(),
  } as LayoutUpdateData
}

export function getCanvasLayoutData(): LayoutUpdateData {
  return buildCanvasLayoutData(backgroundPageOverlays(), activeCanvasSelection())
}

// Re-export sidebar builders from their dedicated module
export { buildLeftSidebarData, getLeftSidebarData, notifyLeftSidebarData } from './sidebar-builder'

export function toolbarSelectionData(): ToolbarSelectionData {
  const selectedPageIds = uiSelectedEntityIds()
  const targets = selectedPageIds
    .map((id) => findPageById(id))
    .filter((p): p is Page => p !== undefined)
  const activePage = selectedPage() ?? targets[0] ?? null
  const availablePageCount = pages.length
  const activeTabName =
    workspaceTabSummaries().find((t) => t.isActive)?.name ?? null

  if (!targets.length || !activePage) {
    return {
      activePageId: null,
      selectedEntityIds: [],
      selectionCount: 0,
      availablePageCount,
      displayUrl: '',
      placeholder: '',
      canGoBack: false,
      canGoForward: false,
      isLoadingActivePage: false,
      loadingPageCount: 0,
      isLoadingAnySelected: false,
      loadingPhase: 'idle',
      activeTabId: activeWorkspaceTabId,
      activeTabName,
      viewMode: uiWorkspaceViewMode(),
      activeTool: uiActiveTool(),
      drawBrushType: getToolDefaults().draw.brushType,
      drawColor: getToolDefaults().draw.color,
      stickyColor: getToolDefaults()['add-sticky'].color,
    }
  }

  const distinctUrls = [...new Set(targets.map((page) => page.pageView.webContents.getURL()))]
  const selectionCount = targets.length
  const loadingPageCount = targets.filter((page) => page.pageView.webContents.isLoadingMainFrame()).length
  const isLoadingAnySelected = loadingPageCount > 0
  const isLoadingActivePage = activePage.pageView.webContents.isLoadingMainFrame()
  const isWaitingForResponse =
    typeof activePage.pageView.webContents.isWaitingForResponse === 'function' &&
    activePage.pageView.webContents.isWaitingForResponse()
  const loadingPhase: ToolbarSelectionData['loadingPhase'] = isLoadingActivePage
    ? isWaitingForResponse
      ? 'waiting-response'
      : 'loading'
    : 'idle'

  return {
    activePageId: activePage.id,
    selectedEntityIds: targets.map((page) => page.id),
    selectionCount,
    availablePageCount,
    displayUrl: distinctUrls.length === 1 ? distinctUrls[0] ?? '' : '',
    placeholder:
      selectionCount > 1 ? `${selectionCount} pages selected` : 'Enter URL',
    canGoBack: targets.some((page) => page.pageView.webContents.navigationHistory.canGoBack()),
    canGoForward: targets.some((page) => page.pageView.webContents.navigationHistory.canGoForward()),
    isLoadingActivePage,
    loadingPageCount,
    isLoadingAnySelected,
    loadingPhase,
    activeTabId: activeWorkspaceTabId,
    activeTabName,
    viewMode: uiWorkspaceViewMode(),
    activeTool: uiActiveTool(),
    drawBrushType: getToolDefaults().draw.brushType,
    drawColor: getToolDefaults().draw.color,
    stickyColor: getToolDefaults()['add-sticky'].color,
  }
}
