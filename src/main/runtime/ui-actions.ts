// Facade: re-exports UI action functions.

export {
  deselectAll,
  selectPage,
  setSelectedPages,
} from './selection-state'

export {
  dismissBrowserDevTools,
  focusAnnotation,
  openCommentsPanel,
  openDevToolsForSelectedPage,
  openInspectPanel,
  toggleLeftSidebar,
  toggleDevTools,
} from './devtools-panel'

export {
  setHoveredInspectTarget,
  setInspectNodeFromPanel,
  setSelectedInspectNodeById,
  setSelectedInspectTarget,
} from './inspect-session'

export {
  activeTool,
  clearActiveTool,
  finishOneShotPlacement,
  isAnnotateMode,
  setActiveTool,
  setDevtoolsPanelTab,
} from './tool-mode'

export { selectAdjacentPage } from './selection-state'

export { selectedPageId } from './runtime-context'

export {
  focusCanvasBounds,
  focusSelectedPage,
  focusSelection,
  refocusActiveSession,
  restoreFocusCamera,
  setFocusPresentationMode,
} from './viewport-control'

export {
  getSelectedEntityIds,
  getSelectedGroupId,
  selectEntity,
  selectPageById,
  setHoverEntity,
  setSelectedEntities,
  setHoveredPage,
  setSelectedGroupId,
} from './runtime-core'
