/**
 * Shared layout and debug constants used across runtime modules.
 */

// --- Layout geometry ---
export const CARD_BORDER_WIDTH = 1
/**
 * SPECULAR_NO_FRAME_VIEW=1 skips the per-page about:blank frameView (one
 * renderer process each). The chrome canvas ring is then the only card
 * border, and the layout pass sends renderer positions after the native
 * setBounds loop so both land in the same frame.
 */
export const NO_FRAME_VIEW = process.env.SPECULAR_NO_FRAME_VIEW === '1'
/**
 * SPECULAR_DRAG_FREEZE=1 captures dragged pages into a bitmap at drag start,
 * parks their native views hidden, and has aboveView draw the bitmap at the
 * layout position instead of moving the native view per tick. See
 * `drag-freeze.ts`.
 */
export const DRAG_FREEZE = process.env.SPECULAR_DRAG_FREEZE === '1'
export const CARD_BORDER_RADIUS = 0
export { TOOLBAR_HEIGHT } from '../../shared/constants'
export const TOOLBAR_BORDER_LIGHT = '#d4d4d8'
export const TOOLBAR_BORDER_DARK = '#3f3f46'
export const LEFT_SIDEBAR_WIDTH = 256

// --- Toolbar padding (mirrors `src/renderer/toolbar/App.tsx`) ---
// Mac reserves space at the left for traffic-light buttons; other platforms
// pad evenly. Kept here so main can compute the toolbar's tool-center x
// without having to ask the renderer.
export const TOOLBAR_PAD_LEFT_MAC = 86
export const TOOLBAR_PAD_RIGHT_MAC = 16
export const TOOLBAR_PAD_LEFT_OTHER = 16
export const TOOLBAR_PAD_RIGHT_OTHER = 16

// --- DevTools panel ---
export const DEVTOOLS_DEFAULT_WIDTH = 400
export const DEVTOOLS_MIN_WIDTH = 280
export const DEVTOOLS_MAX_WIDTH = 960
export const DEVTOOLS_RESIZE_HANDLE_WIDTH = 12
export const DEVTOOLS_PANEL_PADDING = 4
export const DEVTOOLS_HEADER_HEIGHT = 34
export const DEVTOOLS_HEADER_GAP = 0

// --- Preferences ---
export const PREFERENCES_FILE = 'preferences.json'

// --- Debug ---
export const SELECTION_DEBUG = process.env.CANVAS_DEBUG_SELECTION === '1'
export const DEVTOOLS_PANEL_DEBUG = process.env.CANVAS_DEBUG_DEVTOOLS_PANEL === '1'

export function selectionDebug(event: string, details?: Record<string, unknown>): void {
  if (!SELECTION_DEBUG) return
  console.log('[selection-debug:main]', { ts: Date.now(), event, ...details })
}

export function devtoolsPanelDebug(event: string, details?: Record<string, unknown>): void {
  if (!DEVTOOLS_PANEL_DEBUG) return
  console.log('[devtools-panel-debug:main]', { ts: Date.now(), event, ...details })
}
