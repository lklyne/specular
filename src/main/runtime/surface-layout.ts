// Facade: re-exports layout, rendering, and view management functions.

export {
  bgView,
  aboveView,

  leftSidebarView,

  toolbarView,
  win,
} from './view-refs'
export { layoutCache } from './layout-cache'
export {
  pan,
  zoom,
} from './runtime-context'

export {
  getCanvasLayoutData,
  getLeftSidebarData,
} from './canvas-layout-data'

export {
  boundCanvasOrigin as canvasOrigin,
  boundScreenBoundsForPage as screenBoundsForPage,
  pageBodyCanvasBounds,
  pageSnapBounds,
  pageVisualBounds,
  pageContentSize,
  frameChromeCanvasBounds,
  projectFramePointToCanvas,
} from './runtime-geometry'

export { snapToGrid } from '../../shared/gesture-utils'

export { broadcastTheme, isDark } from './preferences'

export {
  requestLayout,
  setPan,
  setZoom,
  focusCanvasBounds,
  restoreFocusCamera,
  recenterFocusPresentation,
  animateCameraTo,
  animatePanTo,
  cancelCameraAnimation,
  moveCameraTo,
} from './viewport-control'
