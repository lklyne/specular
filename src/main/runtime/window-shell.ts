// Facade: re-exports window management and shell functions.

export {
  bgView,
  aboveView,

  toolbarView,
  win,
} from './view-refs'
export { setSelectionOverlayRect } from './overlay-manager'

export { broadcastTheme, isDark } from './preferences'

export {
  endDevtoolsResize,
  setCommentOverlayActive,
  setDevtoolsWidthFromScreenX,
} from './runtime-core'

export { initWindow } from './window-init'

export { rebuildWindowFromSnapshot } from './space-restore'
