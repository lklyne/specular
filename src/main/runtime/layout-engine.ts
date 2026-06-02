import { screen, type WebContentsView } from 'electron'
import {
  boundsKey,
  boundApplyEmulation,
  boundEffectivePageContentSize,
  boundIsFillBrowserPage,
  boundScreenBoundsForPage,
  boundSelectedPage,
  boundSelectedPageId,
  boundCanvasOrigin,
} from './runtime-geometry'
import {
  aboveView,
  bgView,
  cursorOverlayWindow,
  devtoolsBackgroundView,
  devtoolsHeaderView,
  devtoolsResizeHandleView,
  devtoolsView,

  leftSidebarView,

  toolbarView,
  win,
} from './view-refs'
import { layoutCache } from './layout-cache'
import { consumeDirty } from './layout-dirty'
import { applyStack } from './layer-stack'
import { reconcileFocus } from './focus-reconciler-runtime'
import { reconcilePageCursorBridge } from './page-cursor-bridge'
import {
  automationInteractivePageCounts,
  hoverTarget,
  hoveringCanvasChrome,
  pages,
  interactionState,
  selectionOverlayActive,
  spaceModifierHeld,
  pan,
  zoom,
} from './runtime-context'
import { shouldGateBeOpen } from './gate-predicate'
import {
  getUiState,
  activeTool as uiActiveTool,
  selectedCanvasTargets,
} from '../ui-state'
import { drawingEntities } from './drawing-entity-state'
import { descendantEntityIdsForGroup } from './group-descendants'
import {
  devtoolsOpen as uiDevtoolsOpen,
  devtoolsPanelTab as uiDevtoolsPanelTab,
  devtoolsWidth as uiDevtoolsWidth,
  isCommentOverlayVisible as uiCommentOverlayVisible,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedEntityIds as uiSelectedEntityIds,
  setDevtoolsWidth as setUiDevtoolsWidth,
  toolbarDropdownOpen as uiToolbarDropdownOpen,
  workspaceViewMode as uiWorkspaceViewMode,
} from '../ui-state'
import {
  backgroundPageOverlays,
  activeCanvasSelection,
  buildCanvasLayoutData,
  buildFloatingUiUpdatePayload,
  sendAnnotationLayoutUpdate,
  selectedComponentTreePayload,
  toolbarSelectionData,
  notifyLeftSidebarData,
  annotationsForPage,
  pageAnnotationsKey,
} from './canvas-layout-data'
import { textEntities, buildTextEntitySceneEntity } from './text-entity-state'
import { fileEntities } from './file-entity-state'
import { listComponentViews, syncComponentViews } from './component-page-factory'
import { getPresenceCursors } from '../presence-cursor'
import { notifyDevtoolsPanelData } from './inspect-session'
import { clampDevtoolsWidth, frameColor, isDark } from './preferences'
import { contentCornerRadiusForDevice, safeAreaCssForDevice } from '../../shared/device-catalog'
import { deviceIdFromMetadata, deviceOrientationFromMetadata, showDeviceFrameFromMetadata } from './runtime-entities'

export function setBoundsIfChanged(
  view: WebContentsView,
  bounds: { x: number; y: number; width: number; height: number },
  previousKey: string | undefined | null,
): string {
  const nextKey = boundsKey(bounds)
  if (nextKey !== previousKey) {
    view.setBounds(bounds)
  }
  return nextKey
}

import {
  BROWSER_HEADER_HEIGHT,
  CARD_BORDER_RADIUS,
  DEVTOOLS_HEADER_GAP,
  DEVTOOLS_HEADER_HEIGHT,
  DEVTOOLS_PANEL_PADDING,
  DEVTOOLS_RESIZE_HANDLE_WIDTH,
  LEFT_SIDEBAR_WIDTH,
  DEVTOOLS_PANEL_DEBUG,
  devtoolsPanelDebug,
} from './runtime-constants'
import { boundsOverlap } from './runtime-geometry'

const HIDDEN_BOUNDS = { x: 0, y: 0, width: 0, height: 0 }

/**
 * Off-screen-but-alive bounds for hidden devtools panels. Unlike a 0×0
 * cull, a 1×1 view parked off-screen keeps its renderer warm so the first
 * visible open does not pay startup + first-paint cost. Page culling still
 * uses HIDDEN_BOUNDS — culled pages should not stay warm.
 */
const DEVTOOLS_HIDDEN_BOUNDS = { x: -10_000, y: 0, width: 1, height: 1 }

/** Off-screen origin for automation-interactive pages parked outside the viewport. */
const AUTOMATION_OFFSCREEN_ORIGIN = -10_000

function layoutDevtoolsViews(): void {
  const devtoolsOpen = uiDevtoolsOpen()
  const devtoolsWidth = uiDevtoolsWidth()
  const devtoolsPanelTab = uiDevtoolsPanelTab()

  // --- Per-page browser-devtools host views ---
  // Each page lazily owns a `devtoolsHostView`. The layout pass sizes the
  // active page's host to the devtools content area and parks every other
  // page's host off-screen — no imperative hiding lives anywhere else.
  const showBrowserDevtools =
    devtoolsOpen && boundSelectedPage() !== null && devtoolsPanelTab === 'browser-devtools'
  let devtoolsContentBounds = DEVTOOLS_HIDDEN_BOUNDS
  if (showBrowserDevtools && win) {
    const { width, height } = win.getBounds()
    const panelWidth = clampDevtoolsWidth(devtoolsWidth)
    setUiDevtoolsWidth(panelWidth)
    const panelY = layoutCache.toolbarHeight
    const panelHeight = height - layoutCache.toolbarHeight
    devtoolsContentBounds = {
      x: width - panelWidth,
      y: panelY + DEVTOOLS_HEADER_HEIGHT + DEVTOOLS_HEADER_GAP,
      width: panelWidth,
      height: Math.max(0, panelHeight - DEVTOOLS_HEADER_HEIGHT - DEVTOOLS_HEADER_GAP),
    }
  }
  for (const page of pages) {
    if (!page.devtoolsHostView) continue
    const isActiveHost = devtoolsView !== null && page.devtoolsHostView === devtoolsView
    page.lastDevtoolsHostBoundsKey = setBoundsIfChanged(
      page.devtoolsHostView,
      isActiveHost ? devtoolsContentBounds : DEVTOOLS_HIDDEN_BOUNDS,
      page.lastDevtoolsHostBoundsKey,
    )
  }

  if (devtoolsBackgroundView && win) {
    const { width, height } = win.getBounds()
    const hiddenBounds = DEVTOOLS_HIDDEN_BOUNDS
    if (devtoolsOpen) {
      layoutCache.lastDevtoolsBackgroundBoundsKey = setBoundsIfChanged(
        devtoolsBackgroundView,
        { x: width - devtoolsWidth, y: layoutCache.toolbarHeight, width: devtoolsWidth, height: height - layoutCache.toolbarHeight },
        layoutCache.lastDevtoolsBackgroundBoundsKey,
      )
    } else {
      layoutCache.lastDevtoolsBackgroundBoundsKey = setBoundsIfChanged(devtoolsBackgroundView, hiddenBounds, layoutCache.lastDevtoolsBackgroundBoundsKey)
    }
  }

  if (devtoolsHeaderView && win) {
    const { width, height } = win.getBounds()
    const hiddenBounds = DEVTOOLS_HIDDEN_BOUNDS
    if (devtoolsOpen) {
      const showCustomPanel =
        boundSelectedPage() === null || devtoolsPanelTab !== 'browser-devtools'
      layoutCache.lastDevtoolsHeaderBoundsKey = setBoundsIfChanged(
        devtoolsHeaderView,
        showCustomPanel
          ? {
              x: width - devtoolsWidth,
              y: layoutCache.toolbarHeight,
              width: devtoolsWidth,
              height: Math.max(0, height - layoutCache.toolbarHeight),
            }
          : {
              x: width - devtoolsWidth,
              y: layoutCache.toolbarHeight,
              width: devtoolsWidth,
              height: DEVTOOLS_HEADER_HEIGHT,
            },
        layoutCache.lastDevtoolsHeaderBoundsKey,
      )
      notifyDevtoolsPanelData()
    } else {
      layoutCache.lastDevtoolsHeaderBoundsKey = setBoundsIfChanged(devtoolsHeaderView, hiddenBounds, layoutCache.lastDevtoolsHeaderBoundsKey)
    }
  }

  if (devtoolsResizeHandleView && win) {
    const { height } = win.getBounds()
    const hiddenBounds = DEVTOOLS_HIDDEN_BOUNDS
    if (devtoolsOpen) {
      const { width, height } = win.getBounds()
      layoutCache.lastDevtoolsResizeBoundsKey = setBoundsIfChanged(
        devtoolsResizeHandleView,
        {
          x: width - devtoolsWidth,
          y: layoutCache.toolbarHeight,
          width: DEVTOOLS_RESIZE_HANDLE_WIDTH,
          height: height - layoutCache.toolbarHeight,
        },
        layoutCache.lastDevtoolsResizeBoundsKey,
      )
    } else {
      layoutCache.lastDevtoolsResizeBoundsKey = setBoundsIfChanged(devtoolsResizeHandleView, hiddenBounds, layoutCache.lastDevtoolsResizeBoundsKey)
    }
  }
}

function layoutAllViews(): void {
  if (!win || win.isDestroyed()) return
  const layoutStart = DEVTOOLS_PANEL_DEBUG ? Date.now() : 0
  const viewMode = uiWorkspaceViewMode()

  const devtoolsOpen = uiDevtoolsOpen()
  const devtoolsWidth = uiDevtoolsWidth()
  const devtoolsPanelTab = uiDevtoolsPanelTab()
  const selectedPageIds = uiSelectedEntityIds()
  const contentTopInset = layoutCache.toolbarHeight + (viewMode === 'browser' ? BROWSER_HEADER_HEIGHT : 0)

  const pageOverlays = backgroundPageOverlays()
  const nextActiveSelection = activeCanvasSelection()

  // --- Canvas background + annotation overlay ---
  if (bgView && win) {
    const { width, height } = win.getBounds()
    const bgWidth = Math.max(0, width - (devtoolsOpen ? devtoolsWidth : 0))
    layoutCache.lastBackgroundBoundsKey = setBoundsIfChanged(bgView, { x: 0, y: 0, width: bgWidth, height }, layoutCache.lastBackgroundBoundsKey)
    if (consumeDirty('canvas')) {
      bgView.webContents.send(
        'layout-update',
        buildCanvasLayoutData(pageOverlays, nextActiveSelection),
      )
      sendAnnotationLayoutUpdate({
        pages: pageOverlays,
        activeSelection: nextActiveSelection,
      })
      bgView.webContents.send('component-tree-data', selectedComponentTreePayload())
    }
  }

  // --- Left sidebar ---
  if (leftSidebarView && win) {
    const { height } = win.getBounds()
    const showLeftSidebar = uiLeftSidebarOpen()
    leftSidebarView.setVisible(showLeftSidebar)
    layoutCache.lastLeftSidebarBoundsKey = setBoundsIfChanged(
      leftSidebarView,
      showLeftSidebar
        ? {
            x: 0,
            y: layoutCache.toolbarHeight,
            width: LEFT_SIDEBAR_WIDTH,
            height: Math.max(0, height - layoutCache.toolbarHeight),
          }
        : { x: 0, y: 0, width: 0, height: 0 },
      layoutCache.lastLeftSidebarBoundsKey,
    )
    if (consumeDirty('sidebar')) {
      notifyLeftSidebarData()
    }
  }

  // --- Above-view bounds ---
  // Main-authoritative cover: shouldGateBeOpen() derives the predicate
  // from interaction + tool mode + modifiers + chrome-hover + presence +
  // marquee + floating menu + saved drawings. The renderer no longer
  // drives this; it only renders what it's told to render.
  if (aboveView && win) {
    const { width, height } = win.getBounds()
    const selectedTargets = selectedCanvasTargets()
    let selectedGroupOwnsPageContent = false
    if (selectedTargets.length === 1 && selectedTargets[0]?.kind === 'group') {
      const groupDescendantIds = new Set(descendantEntityIdsForGroup(selectedTargets[0].id))
      selectedGroupOwnsPageContent = pages.some((page) => groupDescendantIds.has(page.id))
    }
    const selectionOwnsPageContent =
      (selectedTargets.length > 1 &&
        selectedTargets.some((target) => target.kind === 'page')) ||
      selectedGroupOwnsPageContent
    const shouldCover = shouldGateBeOpen({
      interactionKind: interactionState.kind === 'idle' ? 'idle'
        : interactionState.kind === 'panning-canvas' ? 'panning'
        : interactionState.kind === 'marquee-select' ? 'marquee'
        : interactionState.kind === 'resizing-entity' ? 'resizing-entity'
        : interactionState.kind === 'editing-entity' ? 'editing-entity'
        : interactionState.kind,
      activeTool: getUiState().activeTool,
      viewMode: uiWorkspaceViewMode(),
      commentOverlayActive: uiCommentOverlayVisible(),
      selectionMarqueeVisible: selectionOverlayActive,
      spaceHeld: spaceModifierHeld,
      hoveringCanvasChrome,
      selectedEntityIds: selectedTargets.map((t) => t.id),
      selectedEntityKinds: selectedTargets.map((t) => t.kind),
      selectionOwnsPageContent,
      hasSavedDrawings: drawingEntities.length > 0,
    })
    const bounds = shouldCover
          ? {
              x: 0,
              y: contentTopInset,
              width: Math.max(0, width - (devtoolsOpen ? devtoolsWidth : 0)),
              height: Math.max(0, height - contentTopInset),
            }
          : { x: 0, y: 0, width: 0, height: 0 }
    layoutCache.lastCommentOverlayBoundsKey = setBoundsIfChanged(
      aboveView,
      bounds,
      layoutCache.lastCommentOverlayBoundsKey,
    )
  }

  // --- Cursor overlay window bounds ---
  // Child BrowserWindow for agent-presence cursors. Bounds are in screen
  // coordinates (not win-relative), derived from the main window's
  // content bounds + the toolbar inset. Shown only when cursors exist.
  if (cursorOverlayWindow && !cursorOverlayWindow.isDestroyed() && win) {
    const hasCursors = getPresenceCursors().length > 0
    if (!hasCursors) {
      if (cursorOverlayWindow.isVisible()) cursorOverlayWindow.hide()
      layoutCache.lastCursorOverlayBoundsKey = null
    } else {
      const contentBounds = win.getContentBounds()
      const overlayBounds = {
        x: contentBounds.x,
        y: contentBounds.y + contentTopInset,
        width: Math.max(1, contentBounds.width - (devtoolsOpen ? devtoolsWidth : 0)),
        height: Math.max(1, contentBounds.height - contentTopInset),
      }
      const key = `${overlayBounds.x},${overlayBounds.y},${overlayBounds.width},${overlayBounds.height}`
      if (layoutCache.lastCursorOverlayBoundsKey !== key) {
        cursorOverlayWindow.setBounds(overlayBounds)
        layoutCache.lastCursorOverlayBoundsKey = key
      }
      if (!cursorOverlayWindow.isVisible()) cursorOverlayWindow.showInactive()
    }
  }

  const winBounds = win.getBounds()
  const windowRect = { x: 0, y: 0, width: winBounds.width, height: winBounds.height }

  // --- Per-page bounds, emulation, annotations ---
  const visibleBrowserPageId = boundSelectedPageId()
  for (const page of pages) {
    const pageStart = DEVTOOLS_PANEL_DEBUG ? Date.now() : 0
    const isVisibleInCurrentMode = viewMode === 'canvas' || page.id === visibleBrowserPageId
    if (!isVisibleInCurrentMode) {
      page.lastFrameBoundsKey = setBoundsIfChanged(page.frameView, HIDDEN_BOUNDS, page.lastFrameBoundsKey)
      page.lastPageBoundsKey = setBoundsIfChanged(page.pageView, HIDDEN_BOUNDS, page.lastPageBoundsKey)
      continue
    }
    const bounds = boundScreenBoundsForPage(page)

    // Viewport culling — off-screen pages get hidden bounds.
    // Skip culling during drag and for pages in automation-interactive mode
    // (agents need non-zero bounds to interact with off-screen pages).
    const isOnScreen = boundsOverlap(bounds.page, windowRect)
    const isAutomationActive = automationInteractivePageCounts.has(page.id)
    if (!isOnScreen && interactionState.kind !== 'dragging-entities') {
      if (isAutomationActive) {
        // Automation-interactive pages that aren't visible on the canvas
        // are parked off-screen at their logical viewport size, so an
        // agent always has a real (un-zoomed) viewport to drive.
        const parkedSize = boundEffectivePageContentSize(page)
        page.lastFrameBoundsKey = setBoundsIfChanged(page.frameView, HIDDEN_BOUNDS, page.lastFrameBoundsKey)
        page.lastPageBoundsKey = setBoundsIfChanged(
          page.pageView,
          {
            x: AUTOMATION_OFFSCREEN_ORIGIN,
            y: AUTOMATION_OFFSCREEN_ORIGIN,
            width: parkedSize.width,
            height: parkedSize.height,
          },
          page.lastPageBoundsKey,
        )
        devtoolsPanelDebug('layout:page', {
          pageId: page.id,
          durationMs: Date.now() - pageStart,
          visible: false,
          parked: true,
          isSelected: selectedPageIds.includes(page.id),
          devtoolsOpen,
        })
        continue
      }
      page.lastFrameBoundsKey = setBoundsIfChanged(page.frameView, HIDDEN_BOUNDS, page.lastFrameBoundsKey)
      page.lastPageBoundsKey = setBoundsIfChanged(page.pageView, HIDDEN_BOUNDS, page.lastPageBoundsKey)
      devtoolsPanelDebug('layout:page', {
        pageId: page.id,
        durationMs: Date.now() - pageStart,
        visible: false,
        culled: true,
        isSelected: selectedPageIds.includes(page.id),
        devtoolsOpen,
      })
      continue
    }

    const isFillBrowser = boundIsFillBrowserPage(page)
    const deviceId = deviceIdFromMetadata(page.metadata)
    const showShell = showDeviceFrameFromMetadata(page.metadata)
    const borderRadius = isFillBrowser
      ? 0
      : deviceId && showShell
        ? Math.round(contentCornerRadiusForDevice(deviceId, deviceOrientationFromMetadata(page.metadata)) * zoom)
        : CARD_BORDER_RADIUS
    page.frameView.setBorderRadius(borderRadius)
    page.pageView.setBorderRadius(borderRadius)
    page.lastFrameBoundsKey = setBoundsIfChanged(page.frameView, bounds.frame, page.lastFrameBoundsKey)
    page.lastPageBoundsKey = setBoundsIfChanged(page.pageView, bounds.page, page.lastPageBoundsKey)

    const { width: emulatedWidth, height: emulatedHeight } = boundEffectivePageContentSize(page)
    const nativeScale = screen.getPrimaryDisplay().scaleFactor
    const pageScale = isFillBrowser ? 1 : zoom
    const pageEmulationKey = `${emulatedWidth}:${emulatedHeight}:${pageScale}:${nativeScale}:${viewMode}:${devtoolsOpen ? devtoolsWidth : 0}`
    if (pageEmulationKey !== page.lastPageEmulationKey) {
      const emulationStart = DEVTOOLS_PANEL_DEBUG ? Date.now() : 0
      boundApplyEmulation(page.pageView.webContents, page.presetIndex, page)
      page.lastPageEmulationKey = pageEmulationKey
      devtoolsPanelDebug('layout:apply-emulation', {
        pageId: page.id,
        durationMs: Date.now() - emulationStart,
        emulatedWidth,
        emulatedHeight,
        viewMode,

        devtoolsOpen,
      })
    }

    // Inject or remove safe-area CSS padding when the device shell is active
    const orientation = deviceOrientationFromMetadata(page.metadata)
    const safeAreaCss = deviceId && showShell && !isFillBrowser
      ? safeAreaCssForDevice(deviceId, orientation)
      : null
    const safeAreaKey = safeAreaCss ?? ''
    if (safeAreaKey !== (page.lastSafeAreaCssKey ?? '')) {
      if (page.lastSafeAreaCssId) {
        page.pageView.webContents.removeInsertedCSS(page.lastSafeAreaCssId).catch(() => {})
        page.lastSafeAreaCssId = undefined
      }
      if (safeAreaCss) {
        page.pageView.webContents.insertCSS(safeAreaCss).then((id) => {
          page.lastSafeAreaCssId = id
        }).catch(() => {})
      }
      page.lastSafeAreaCssKey = safeAreaKey
    }

    const themeKey = isDark()
    if (page.lastSelected !== themeKey) {
      page.frameView.setBackgroundColor(frameColor())
      page.lastSelected = themeKey
    }

    const pageAnnotations = annotationsForPage(page.id)
    const nextPageAnnotationsKey = pageAnnotationsKey(pageAnnotations)
    if (nextPageAnnotationsKey !== page.lastPageAnnotationsKey) {
      page.pageView.webContents.send('page-annotations-update', {
        annotations: pageAnnotations,
      })
      page.lastPageAnnotationsKey = nextPageAnnotationsKey
    }
    devtoolsPanelDebug('layout:page', {
      pageId: page.id,
      durationMs: Date.now() - pageStart,
      visible: true,
      isSelected: selectedPageIds.includes(page.id),
      devtoolsOpen,
    })
  }

  // (above-view bounds are now handled in the consolidated block above)

  // --- Per-component bounds + emulation ---
  // Reconcile the component-view set against the current file entities,
  // then position each view to match its entity's canvas footprint. Hidden
  // entirely in browser mode — components are design artifacts, not
  // navigable web content, so they don't get tabs.
  syncComponentViews(fileEntities)

  // Child-list reconcile runs here — after syncComponentViews so component
  // views created this pass are attached the same pass — and owns the full
  // ordered child list (bgView → pages → components → overlays → toolbar).
  applyStack()

  const componentsHidden = viewMode === 'browser'
  const canvasOrigin = boundCanvasOrigin()
  const nativeScale = screen.getPrimaryDisplay().scaleFactor
  for (const cv of listComponentViews()) {
    const entity = fileEntities.find((e) => e.id === cv.entityId)
    if (!entity || componentsHidden) {
      cv.lastBoundsKey = setBoundsIfChanged(cv.view, HIDDEN_BOUNDS, cv.lastBoundsKey)
      continue
    }
    const bounds = {
      x: Math.round(canvasOrigin.x + entity.canvasX * zoom + pan.x),
      y: Math.round(canvasOrigin.y + entity.canvasY * zoom + pan.y),
      width: Math.max(0, Math.round(entity.width * zoom)),
      height: Math.max(0, Math.round(entity.height * zoom)),
    }

    // Cull when fully off-screen, but stay visible during drags so a
    // component that briefly leaves the viewport doesn't blink.
    const onScreen = boundsOverlap(bounds, windowRect)
    if (!onScreen && interactionState.kind !== 'dragging-entities') {
      cv.lastBoundsKey = setBoundsIfChanged(cv.view, HIDDEN_BOUNDS, cv.lastBoundsKey)
      continue
    }

    cv.lastBoundsKey = setBoundsIfChanged(cv.view, bounds, cv.lastBoundsKey)

    // Emulate the entity's logical viewport and let canvas zoom drive the
    // paint scale. Mirrors page emulation so components reflow the same way.
    const emulationKey = `${entity.width}:${entity.height}:${zoom}:${nativeScale}`
    if (emulationKey !== cv.lastEmulationKey) {
      cv.view.webContents.enableDeviceEmulation({
        screenPosition: 'desktop',
        screenSize: { width: entity.width, height: entity.height },
        viewSize: { width: entity.width, height: entity.height },
        viewPosition: { x: 0, y: 0 },
        deviceScaleFactor: nativeScale,
        scale: zoom,
      })
      cv.lastEmulationKey = emulationKey
    }
  }

  // --- Devtools ---
  layoutDevtoolsViews()

  // --- Toolbar ---
  // While a toolbar dropdown is open the view grows to full-window bounds
  // so the menu can overflow the toolbar strip; otherwise it is just the
  // strip height.
  if (toolbarView && win) {
    const { width, height } = win.getBounds()
    layoutCache.lastToolbarBoundsKey = setBoundsIfChanged(
      toolbarView,
      uiToolbarDropdownOpen()
        ? { x: 0, y: 0, width, height }
        : { x: 0, y: 0, width, height: layoutCache.toolbarHeight },
      layoutCache.lastToolbarBoundsKey,
    )
    if (consumeDirty('toolbar')) {
      toolbarView.webContents.send('zoom-changed', Math.round(zoom * 100))
      toolbarView.webContents.send('toolbar-selection-changed', toolbarSelectionData())
      toolbarView.webContents.send('left-sidebar-changed', uiLeftSidebarOpen())
      toolbarView.webContents.send('agent-presence-changed', getPresenceCursors())
    }
  }

  // Post-layout: reconcile focus + page-cursor bridge against the
  // post-mutation world. Both observe the same predicate
  // (`currentKeyboardTargetPageId`).
  reconcileFocus()
  reconcilePageCursorBridge()

  devtoolsPanelDebug('layout:all-views-complete', {
    durationMs: Date.now() - layoutStart,
    pageCount: pages.length,
    devtoolsOpen,
    selectedPageIds,
    activeTab: devtoolsPanelTab,
  })
}

/**
 * The single public way to trigger layout. Debounces a `layoutAllViews()`
 * pass onto a 16ms timer so a burst of mutations collapses into one pass.
 * `layoutAllViews` / `layoutDevtoolsViews` are module-private — every call
 * site outside this file routes through here (invariant I1).
 */
export function requestLayout(): void {
  if (layoutCache.layoutTimer) return
  layoutCache.layoutTimer = setTimeout(() => {
    layoutCache.layoutTimer = null
    layoutAllViews()
  }, 16)
}
