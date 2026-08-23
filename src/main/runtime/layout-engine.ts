// fallow-ignore-file circular-dependencies
// reconcileBrowserDevtools lives in runtime-core, which imports requestLayout
// via viewport-control → this file. See devtools-panel.ts for the same cycle.
import { screen, type WebContentsView } from 'electron'
import {
  boundsKey,
  boundPageMetrics,
  boundEffectivePageContentSize,
  boundScreenBoundsForPage,
  boundSelectedPage,
  boundCanvasOrigin,
  focusFillRegion,
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
import { pageVisibilityOverride } from './page-visibility'
import { reconcileFocus } from './focus-reconciler-runtime'
import { reconcileBrowserDevtools } from './runtime-core'
import { reconcilePageCursorBridge } from './page-cursor-bridge'
import {
  automationInteractivePageCounts,
  inspectHoveredTarget,
  inspectSelectedTarget,
  pages,
  interactionState,
  pan,
  zoom,
} from './runtime-context'
import { focusSession, focusedPageId } from './focus-session'
import { shouldGateBeOpen } from './gate-predicate'
import {
  getUiState,
} from '../ui-state'
import {
  devtoolsOpen as uiDevtoolsOpen,
  devtoolsPanelTab as uiDevtoolsPanelTab,
  devtoolsWidth as uiDevtoolsWidth,
  isCommentOverlayVisible as uiCommentOverlayVisible,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedEntityIds as uiSelectedEntityIds,
  setDevtoolsWidth as setUiDevtoolsWidth,
  toolbarDropdownOpen as uiToolbarDropdownOpen,
  toolbarTooltipOpen as uiToolbarTooltipOpen,
} from '../ui-state'
import {
  backgroundPageOverlays,
  activeCanvasSelection,
  buildCanvasLayoutData,
  toolbarSelectionData,
  notifyLeftSidebarData,
} from './canvas-layout-data'
import { fileEntities } from './file-entity-state'
import { listComponentViews, syncComponentViews } from './component-page-factory'
import { getPresenceCursors } from '../presence-cursor'
import { notifyDevtoolsPanelData } from './inspect-session'
import { clampDevtoolsWidth } from './preferences'
import { contentCornerRadiusForDevice, safeAreaCssForDevice } from '../../shared/device-catalog'
import { ipcChannels } from '../../shared/ipc-contract'
import { broadcastSceneUpdate } from './runtime-patch-broadcast'
import { deviceIdFromMetadata, deviceOrientationFromMetadata, showDeviceFrameFromMetadata } from './runtime-entities'
import type { Page } from './runtime-entities'
import { applyPageColorScheme } from './page-color-scheme'
import { pageParkingFor } from './page-freeze'
import { scheduleZoomSnapshotPreparation } from './zoom-snapshot-freeze'
import { applyPageMetrics, clearPageMetrics, invalidatePageMetrics, pageRendersNatively } from './page-emulation'
import { logCrash } from '../crash-log'

let buildMsSink: ((ms: number) => void) | null = null

export function setBuildMsSink(fn: ((ms: number) => void) | null): void {
  buildMsSink = fn
}

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
  CARD_BORDER_RADIUS,
  DEVTOOLS_HEADER_GAP,
  DEVTOOLS_HEADER_HEIGHT,
  DEVTOOLS_RESIZE_HANDLE_WIDTH,
  LEFT_SIDEBAR_WIDTH,
  DEVTOOLS_PANEL_DEBUG,
  devtoolsPanelDebug,
} from './runtime-constants'
import { boundsOverlap } from './runtime-geometry'
import { isZoomInMotion, quantizeZoomForEmulation } from './zoom-motion'

const HIDDEN_BOUNDS = { x: 0, y: 0, width: 0, height: 0 }

// Extra px the toolbar view grows by while a tooltip is open — enough for one
// tip row (sideOffset + line) below the 44px strip, no more.
const TOOLBAR_TOOLTIP_BAND = 48
/**
 * Off-screen-but-alive bounds for hidden devtools panels. Unlike a 0×0
 * cull, a 1×1 view parked off-screen keeps its renderer warm so the first
 * visible open does not pay startup + first-paint cost. Page culling still
 * uses HIDDEN_BOUNDS — culled pages should not stay warm.
 */
const DEVTOOLS_HIDDEN_BOUNDS = { x: -10_000, y: 0, width: 1, height: 1 }
/**
 * Warm park: the view keeps a one-pixel column inside the window's left
 * edge. viz stops issuing BeginFrames to a surface that falls entirely
 * outside the window (or is fully covered by another view), so a view
 * parked fully off-screen stops laying out and presenting:
 * requestAnimationFrame never fires and the settle handoff cannot tell when
 * the page has rendered at its new scale.
 */
function warmParkBounds(
  bounds: { x: number; y: number; width: number; height: number },
  windowHeight: number,
): { x: number; y: number; width: number; height: number } {
  return {
    ...bounds,
    x: 1 - bounds.width,
    y: Math.min(Math.max(bounds.y, 1 - bounds.height), windowHeight - 1),
  }
}

/**
 * Where a visible page's view goes this pass. Fill focus sits below the flush
 * focus chrome bar and fills the rest of the canvas area (focusFillRegion()
 * is the shared source of truth). Warm parking is the settle handoff: the
 * view rasters at its settled size and scale while hidden behind the frozen
 * bitmap, so the reveal shows a frame that already matches instead of the
 * pre-gesture surface stretched into the new bounds.
 */
function placedPageBounds(
  canvasBounds: { x: number; y: number; width: number; height: number },
  isFillFocus: boolean,
  parking: ReturnType<typeof pageParkingFor>,
  windowHeight: number,
): { x: number; y: number; width: number; height: number } {
  if (isFillFocus) return focusFillRegion()
  if (parking === 'warm') return warmParkBounds(canvasBounds, windowHeight)
  return canvasBounds
}

/**
 * Injects or removes safe-area CSS padding so the page matches the device
 * shell; `null` removes any previously inserted padding.
 */
function syncSafeAreaCss(page: Page, safeAreaCss: string | null): void {
  const safeAreaKey = safeAreaCss ?? ''
  if (safeAreaKey === (page.lastSafeAreaCssKey ?? '')) return
  const wc = page.pageView.webContents
  if (page.lastSafeAreaCssId) {
    wc.removeInsertedCSS(page.lastSafeAreaCssId).catch(() => {})
    page.lastSafeAreaCssId = undefined
  }
  if (safeAreaCss) {
    wc.insertCSS(safeAreaCss).then((id) => {
      page.lastSafeAreaCssId = id
    }).catch(() => {})
  }
  page.lastSafeAreaCssKey = safeAreaKey
}

/**
 * Applies the page's canvas metrics at the current zoom. Mid-gesture the
 * scale is quantized so re-raster fires at bucket crossings only; the settle
 * pass restores the exact zoom.
 */
export function ensurePageEmulation(page: Page): void {
  const metrics = boundPageMetrics(page)
  if (isZoomInMotion()) metrics.scale = quantizeZoomForEmulation(metrics.scale)
  applyPageMetrics(page.pageView.webContents, metrics)
}

/**
 * Re-emulates `page` for a freshly committed document. Fill focus renders
 * natively and is left alone; the layout pass owns that switch.
 */
export function applyNavigationEmulation(page: Page): void {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return
  if (pageRendersNatively(wc)) return
  invalidatePageMetrics(wc)
  ensurePageEmulation(page)
}

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

/**
 * Reconciles a page's content-view visibility against its override.
 *
 * Runs before the bounds branches because visibility is independent of where
 * a page sits: a page can be on-screen and hidden, or culled and awake.
 */
function applyPageVisibility(page: Page): void {
  if (typeof page.pageView.setVisible !== 'function') return
  const desired = pageVisibilityOverride(page.id) ?? true
  if (page.lastVisibleApplied === desired) return
  page.pageView.setVisible(desired)
  page.lastVisibleApplied = desired
}

export { layoutAllViews }

// Inherited from main: the audit keys findings by exceeded dimension, and
// exporting this function moved its estimated CRAP under the threshold, so the
// (smaller) finding reads as new.
// fallow-ignore-next-line complexity
function layoutAllViews(): void {
  if (!win || win.isDestroyed()) return
  const layoutStart = DEVTOOLS_PANEL_DEBUG ? Date.now() : 0

  const devtoolsOpen = uiDevtoolsOpen()
  const devtoolsWidth = uiDevtoolsWidth()
  const devtoolsPanelTab = uiDevtoolsPanelTab()
  const selectedPageIds = uiSelectedEntityIds()
  const contentTopInset = layoutCache.toolbarHeight

  const pageOverlays = backgroundPageOverlays()
  const nextActiveSelection = activeCanvasSelection()
  // Renderer positions ship after every native setBounds call below, so
  // the DOM chrome never leads the page views.
  let pendingLayoutData: ReturnType<typeof buildCanvasLayoutData> | null = null

  // --- Canvas background + annotation overlay ---
  if (bgView && win) {
    const { width, height } = win.getBounds()
    const bgWidth = Math.max(0, width - (devtoolsOpen ? devtoolsWidth : 0))
    layoutCache.lastBackgroundBoundsKey = setBoundsIfChanged(bgView, { x: 0, y: 0, width: bgWidth, height }, layoutCache.lastBackgroundBoundsKey)
    if (consumeDirty('canvas')) {
      const buildStart = performance.now()
      const layoutData = buildCanvasLayoutData(pageOverlays, nextActiveSelection)
      layoutData.buildMs = performance.now() - buildStart
      buildMsSink?.(layoutData.buildMs)
      pendingLayoutData = layoutData
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
    const shouldCover = shouldGateBeOpen({
      activeTool: getUiState().activeTool,
      commentOverlayActive: uiCommentOverlayVisible(),
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
  // content bounds + the toolbar inset. Shown only when click-through
  // screen overlays exist and the main window is focused. Showing an
  // OS-level child window while the app is in the background can raise the
  // application on macOS even when showInactive() leaves keyboard focus alone.
  if (cursorOverlayWindow && !cursorOverlayWindow.isDestroyed() && win) {
    const hasCursors = getPresenceCursors().length > 0
    const hasInspectPopover =
      getUiState().activeTool.kind === 'inspect' &&
      Boolean(inspectHoveredTarget ?? inspectSelectedTarget)
    if ((!hasCursors && !hasInspectPopover) || !win.isFocused()) {
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
  const focusSessionValue = focusSession()
  const focusedPresentationPageId = focusedPageId()
  // Eye on: other pages' live content returns as surrounding context, subject
  // to normal culling. Eye off: only the focused page shows. Binary show/hide,
  // never dimmed (ADR 0021). A page session in 'fill' mode is the exception —
  // the focused page covers the viewport, so context never returns. A file
  // session (always 'fill') frames a note drawn in the aboveView overlay and has
  // no focused page id, so every page is context and the eye governs all of
  // them; without that, native page layers float over the note backdrop
  // (ADR 0021 Amendment 2).
  const showOtherPagesInFocus =
    (focusedPresentationPageId === null || focusSessionValue?.mode !== 'fill') &&
    (focusSessionValue?.annotationsVisible ?? false)
  for (const page of pages) {
    const pageStart = DEVTOOLS_PANEL_DEBUG ? Date.now() : 0
    applyPageVisibility(page)
    const bounds = boundScreenBoundsForPage(page)

    const parking = pageParkingFor(page.id)
    if (parking === 'hidden') {
      page.lastPageBoundsKey = setBoundsIfChanged(
        page.pageView,
        HIDDEN_BOUNDS,
        page.lastPageBoundsKey,
      )
      continue
    }

    if (
      focusSessionValue !== null &&
      page.id !== focusedPresentationPageId &&
      !showOtherPagesInFocus
    ) {
      page.lastPageBoundsKey = setBoundsIfChanged(page.pageView, HIDDEN_BOUNDS, page.lastPageBoundsKey)
      devtoolsPanelDebug('layout:page', {
        pageId: page.id,
        durationMs: Date.now() - pageStart,
        visible: false,
        hiddenByFocusPresentation: true,
        isSelected: selectedPageIds.includes(page.id),
        devtoolsOpen,
      })
      continue
    }

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

    const deviceId = deviceIdFromMetadata(page.metadata)
    const showShell = showDeviceFrameFromMetadata(page.metadata)
    // 'fill' focus is the browser mode: page fills the canvas viewport edge to
    // edge with no chrome header, no bezel, and square corners.
    const isFillFocus =
      focusedPresentationPageId === page.id && focusSessionValue?.mode === 'fill'
    const borderRadius = isFillFocus
      ? 0
      : deviceId && showShell
        ? Math.round(contentCornerRadiusForDevice(deviceId, deviceOrientationFromMetadata(page.metadata)) * zoom)
        : CARD_BORDER_RADIUS
    page.pageView.setBorderRadius(borderRadius)
    page.lastPageBoundsKey = setBoundsIfChanged(
      page.pageView,
      placedPageBounds(bounds.page, isFillFocus, parking, winBounds.height),
      page.lastPageBoundsKey,
    )

    if (isFillFocus) {
      // Fill is the browser mode: render natively at 100% with no device
      // emulation, so the page reflows to the real view size and viewport-aware
      // layout (sticky headers, 100vh, visualViewport) behaves like a real tab.
      // Emulation at scale 1 leaves pages in a stale layout until a scroll.
      if (clearPageMetrics(page.pageView.webContents)) {
        page.pageView.webContents.setZoomFactor(1)
      }
    } else {
      ensurePageEmulation(page)
    }

    if (page.colorScheme !== page.lastColorSchemeKey) {
      // Commit the key only when the override actually dispatched, so a
      // failed attach retries on the next pass.
      if (applyPageColorScheme(page, page.colorScheme ?? null)) {
        page.lastColorSchemeKey = page.colorScheme
      }
    }

    // Inject or remove safe-area CSS padding when the device shell is active.
    // Fill mode is chromeless, so it never gets device safe-area padding.
    const orientation = deviceOrientationFromMetadata(page.metadata)
    syncSafeAreaCss(
      page,
      !isFillFocus && deviceId && showShell ? safeAreaCssForDevice(deviceId, orientation) : null,
    )

    devtoolsPanelDebug('layout:page', {
      pageId: page.id,
      durationMs: Date.now() - pageStart,
      visible: true,
      isSelected: selectedPageIds.includes(page.id),
      devtoolsOpen,
    })
  }

  // (above-view bounds are now handled in the consolidated block above)

  if (pendingLayoutData) broadcastSceneUpdate(pendingLayoutData)

  // --- Per-component bounds + emulation ---
  // Reconcile the component-view set against the current file entities,
  // then position each view to match its entity's canvas footprint.
  syncComponentViews(fileEntities)

  // Child-list reconcile runs here — after syncComponentViews so component
  // views created this pass are attached the same pass — and owns the full
  // ordered child list (bgView → pages → components → overlays → toolbar).
  applyStack()

  const canvasOrigin = boundCanvasOrigin()
  const nativeScale = screen.getPrimaryDisplay().scaleFactor
  for (const cv of listComponentViews()) {
    const entity = fileEntities.find((e) => e.id === cv.entityId)
    if (!entity) {
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
  // The toolbar view is normally just the strip height. A dropdown grows it
  // to full-window so the menu can overflow; a tooltip grows it by a shallow
  // band so the tip paints just below the strip while keeping the transparent
  // click-swallow region over the canvas small.
  if (toolbarView && win) {
    const { width, height } = win.getBounds()
    const tooltipBandHeight = Math.min(height, layoutCache.toolbarHeight + TOOLBAR_TOOLTIP_BAND)
    const toolbarHeight = uiToolbarDropdownOpen()
      ? height
      : uiToolbarTooltipOpen()
        ? tooltipBandHeight
        : layoutCache.toolbarHeight
    layoutCache.lastToolbarBoundsKey = setBoundsIfChanged(
      toolbarView,
      { x: 0, y: 0, width, height: toolbarHeight },
      layoutCache.lastToolbarBoundsKey,
    )
    if (consumeDirty('toolbar')) {
      toolbarView.webContents.send(ipcChannels.zoomChanged, Math.round(zoom * 100))
      toolbarView.webContents.send(ipcChannels.toolbarSelectionChanged, toolbarSelectionData())
      toolbarView.webContents.send(ipcChannels.leftSidebarChanged, uiLeftSidebarOpen())
      toolbarView.webContents.send(ipcChannels.agentPresenceChanged, getPresenceCursors())
    }
  }

  // Post-layout: reconcile focus + page-cursor bridge against the
  // post-mutation world. Both observe the same predicate
  // (`currentKeyboardTargetPageId`).
  reconcileFocus()
  reconcileBrowserDevtools()
  reconcilePageCursorBridge()

  devtoolsPanelDebug('layout:all-views-complete', {
    durationMs: Date.now() - layoutStart,
    pageCount: pages.length,
    devtoolsOpen,
    selectedPageIds,
    activeTab: devtoolsPanelTab,
  })
  scheduleZoomSnapshotPreparation()
}

// TEMP instrument (plan: diffed-runtime-store) — who is asking for a layout
// pass, counted by caller and reported to errors.log every 2s. Broadcast
// counts alone can't answer this: structural sharing makes a pass cheap while
// it still fires, so the histogram is how a migrated slice is proven gone.
const layoutCauses = new Map<string, number>()
let layoutCauseTimer: NodeJS.Timeout | null = null

function recordLayoutCause(): void {
  const frames = new Error().stack?.split('\n')
  if (!frames) return
  // Frame 0 is the Error line, 1 is this function, 2 is requestLayout; the
  // first frame past those that isn't this module is the caller worth naming.
  let cause = 'unknown'
  for (const frame of frames.slice(3)) {
    if (frame.includes('layout-engine')) continue
    const token = frame.trim().replace(/^at /, '').split(' ')[0] || 'anonymous'
    // An anonymous frame reports a path instead of a name; its basename is
    // the useful half.
    cause = token.includes('/') ? (token.split('/').pop() ?? token) : token
    break
  }
  layoutCauses.set(cause, (layoutCauses.get(cause) ?? 0) + 1)
  if (layoutCauseTimer) return
  layoutCauseTimer = setInterval(() => {
    if (layoutCauses.size === 0) return
    const rows = [...layoutCauses.entries()]
      .sort((a, b) => b[1] - a[1])
      .map(([name, count]) => `${name}=${count}`)
    layoutCauses.clear()
    logCrash('requestLayout-causes', rows.join(' '))
  }, 2000)
  layoutCauseTimer.unref?.()
}

/**
 * The default way to trigger layout. Debounces a `layoutAllViews()` pass
 * onto a 16ms timer so a burst of mutations collapses into one pass
 * (invariant I1). `layoutAllViews` is exported only for the gesture paths
 * that must place views synchronously (drag freeze, viewport control).
 */
export function requestLayout(): void {
  recordLayoutCause()
  if (layoutCache.layoutTimer) return
  layoutCache.layoutTimer = setTimeout(() => {
    layoutCache.layoutTimer = null
    layoutAllViews()
  }, 16)
}
