// fallow-ignore-file circular-dependencies
// reconcileBrowserDevtools lives in runtime-core, which imports requestLayout
// via viewport-control → this file. See devtools-panel.ts for the same cycle.
import { screen, type WebContentsView } from 'electron'
import {
  boundsKey,
  boundEffectivePageContentSize,
  boundScreenBoundsForPage,
  boundSelectedPage,
  boundCanvasOrigin,
} from './runtime-geometry'
import { projectToScreen } from '../../shared/scene-projection'
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
import { reconcilePageFocusEmulation } from './page-focus-emulation'
import {
  getUiState,
} from '../ui-state'
import {
  devtoolsOpen as uiDevtoolsOpen,
  devtoolsPanelTab as uiDevtoolsPanelTab,
  devtoolsWidth as uiDevtoolsWidth,
  leftSidebarOpen as uiLeftSidebarOpen,
  selectedEntityIds as uiSelectedEntityIds,
  setDevtoolsWidth as setUiDevtoolsWidth,
  toolbarDropdownOpen as uiToolbarDropdownOpen,
  toolbarTooltipOpen as uiToolbarTooltipOpen,
} from '../ui-state'
import {
  buildCanvasLayoutData,
  toolbarSelectionData,
  notifyLeftSidebarData,
} from './canvas-layout-data'
import { backgroundPageOverlays } from './page-scene-entity'
import { fileEntities } from './file-entity-state'
import { listComponentViews, syncComponentViews } from './component-page-factory'
import { getPresenceCursors } from '../presence-cursor'
import { notifyDevtoolsPanelData } from './inspect-session'
import { clampDevtoolsWidth, getHideAgentCursors } from './preferences'
import { safeAreaCssForDevice } from '../../shared/device-catalog'
import { ipcChannels } from '../../shared/ipc-contract'
import { broadcastSceneUpdate } from './runtime-patch-broadcast'
import { deviceIdFromMetadata, deviceOrientationFromMetadata, showDeviceFrameFromMetadata } from './runtime-entities'
import { isPagePresented } from '../../shared/page-presentation'
import type { Page } from './runtime-entities'
import { applyPageColorScheme } from './page-color-scheme'
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
  DEVTOOLS_HEADER_GAP,
  DEVTOOLS_HEADER_HEIGHT,
  DEVTOOLS_RESIZE_HANDLE_WIDTH,
  LEFT_SIDEBAR_WIDTH,
  DEVTOOLS_PANEL_DEBUG,
  driftWatchdogEnabled,
  devtoolsPanelDebug,
} from './runtime-constants'
import { boundsOverlap } from './runtime-geometry'

const HIDDEN_BOUNDS = { x: 0, y: 0, width: 0, height: 0 }

// Extra px the toolbar view grows by while a tooltip is open — enough for one
// tip row (sideOffset + line) below the 44px strip, no more.
const TOOLBAR_TOOLTIP_BAND = 48
/**
 * Off-screen-but-alive bounds for hidden devtools panels. Unlike a 0×0
 * cull, a 1×1 view parked off-screen keeps its renderer warm so the first
 * visible open does not pay startup + first-paint cost.
 */
const DEVTOOLS_HIDDEN_BOUNDS = { x: -10_000, y: 0, width: 1, height: 1 }

/**
 * Injects or removes safe-area CSS padding so the page matches the device
 * shell; `null` removes any previously inserted padding.
 */
function syncSafeAreaCss(page: Page, safeAreaCss: string | null): void {
  const safeAreaKey = safeAreaCss ?? ''
  if (safeAreaKey === (page.lastSafeAreaCssKey ?? '')) return
  const wc = page.host.webContents
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
      const layoutData = buildCanvasLayoutData(pageOverlays)
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
  // aboveView covers the canvas area unconditionally: pages render offscreen,
  // so no native input ever reaches one and every pointer, wheel and key that
  // belongs to a page is forwarded from here.
  if (aboveView && win) {
    const { width, height } = win.getBounds()
    layoutCache.lastAboveViewBoundsKey = setBoundsIfChanged(
      aboveView,
      {
        x: 0,
        y: contentTopInset,
        width: Math.max(0, width - (devtoolsOpen ? devtoolsWidth : 0)),
        height: Math.max(0, height - contentTopInset),
      },
      layoutCache.lastAboveViewBoundsKey,
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
    const hasCursors = !getHideAgentCursors() && getPresenceCursors().length > 0
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

  // --- Per-page host size, painting policy, safe-area CSS ---
  const focusSessionValue = focusSession()
  const focusedPresentationPageId = focusedPageId()
  const presentationFocus = {
    active: focusSessionValue !== null,
    pageId: focusedPresentationPageId,
    mode: focusSessionValue?.mode ?? null,
    showsContext: focusSessionValue === null || focusSessionValue.annotationsVisible,
  }
  for (const page of pages) {
    const pageStart = DEVTOOLS_PANEL_DEBUG ? Date.now() : 0
    // The host's CSS viewport is the page's authored size, or the focus
    // session's region — fill focus resizes the window to the fill region, so
    // the page reflows like a real tab instead of being scaled into one.
    const contentSize = boundEffectivePageContentSize(page)
    page.host.resize(contentSize)

    const presented = isPagePresented(page.id, presentationFocus)
    // An off-screen page stops painting, except while it is being dragged (its
    // texture must keep up with the move) or driven by an agent.
    const pageScreenRect = boundScreenBoundsForPage(page).page
    const onScreen = boundsOverlap(pageScreenRect, windowRect)
    const painting =
      presented &&
      (onScreen ||
        interactionState.kind === 'dragging-entities' ||
        automationInteractivePageCounts.has(page.id))
    page.host.setPainting(painting)
    // A page earns frame rate by its size on screen. Agent-driven pages and
    // the focus session's page paint at full rate whatever the camera — an
    // agent's captures and a presented page don't follow the zoom.
    const fullRate =
      automationInteractivePageCounts.has(page.id) ||
      focusedPresentationPageId === page.id
    page.host.setDisplayScale(fullRate ? 1 : pageScreenRect.width / contentSize.width)

    if (page.colorScheme !== page.lastColorSchemeKey) {
      // Commit the key only when the override actually dispatched, so a
      // failed attach retries on the next pass.
      if (applyPageColorScheme(page, page.colorScheme ?? null)) {
        page.lastColorSchemeKey = page.colorScheme
      }
    }

    // Inject or remove safe-area CSS padding when the device shell is active.
    // Fill mode is chromeless, so it never gets device safe-area padding.
    const isFillFocus =
      focusedPresentationPageId === page.id && focusSessionValue?.mode === 'fill'
    const deviceId = deviceIdFromMetadata(page.metadata)
    const showShell = showDeviceFrameFromMetadata(page.metadata)
    const orientation = deviceOrientationFromMetadata(page.metadata)
    syncSafeAreaCss(
      page,
      !isFillFocus && deviceId && showShell ? safeAreaCssForDevice(deviceId, orientation) : null,
    )

    devtoolsPanelDebug('layout:page', {
      pageId: page.id,
      durationMs: Date.now() - pageStart,
      presented,
      painting,
      isSelected: selectedPageIds.includes(page.id),
      devtoolsOpen,
    })
  }

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
    // A `WebContentsView` is positioned in whole window pixels, so the
    // projection is rounded here rather than by the projector.
    const rect = projectToScreen(
      { x: entity.canvasX, y: entity.canvasY, width: entity.width, height: entity.height },
      { zoom, pan },
      canvasOrigin,
    )
    const bounds = {
      x: Math.round(rect.x),
      y: Math.round(rect.y),
      width: Math.max(0, Math.round(rect.width)),
      height: Math.max(0, Math.round(rect.height)),
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

  // Post-layout: reconcile focus, page focus emulation, and the page-cursor
  // bridge against the post-mutation world. All three observe the same
  // predicate (`currentKeyboardTargetPageId`).
  reconcileFocus()
  reconcilePageFocusEmulation()
  reconcileBrowserDevtools()
  reconcilePageCursorBridge()

  devtoolsPanelDebug('layout:all-views-complete', {
    durationMs: Date.now() - layoutStart,
    pageCount: pages.length,
    devtoolsOpen,
    selectedPageIds,
    activeTab: devtoolsPanelTab,
  })
}

// Who is asking for a layout pass, counted by caller and reported to
// errors.log every 2s. Broadcast counts alone can't answer this: structural
// sharing makes a pass cheap while it still fires, so the histogram is how a
// mutator that should own a slice patch instead of a geometry pass gets found.
const layoutCauses = new Map<string, number>()
let layoutCauseTimer: NodeJS.Timeout | null = null

function recordLayoutCause(): void {
  if (!driftWatchdogEnabled()) return
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
 * The default way to trigger layout. A single-flight guard defers
 * `layoutAllViews()` to the next event-loop turn, so a burst of mutations
 * within the same turn collapses into one pass (invariant I1).
 * `layoutAllViews` is exported only for the gesture paths that must place
 * views synchronously (drag freeze, viewport control).
 */
export function requestLayout(): void {
  recordLayoutCause()
  if (layoutCache.layoutTimer) return
  layoutCache.layoutTimer = setImmediate(() => {
    layoutCache.layoutTimer = null
    layoutAllViews()
  })
}
