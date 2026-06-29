/**
 * Shared layout and debug constants used across runtime modules.
 */

// --- Layout geometry ---
export const CARD_BORDER_WIDTH = 1
export const CARD_BORDER_RADIUS = 0
export { CHROME_HEADER_HEIGHT } from '../../shared/entity-chrome-slots'
export const CHROME_PAGE_GAP = 0
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
export const COMMENT_BADGE_DEBUG = process.env.CANVAS_DEBUG_COMMENT_BADGES === '1'
export const DEVTOOLS_PANEL_DEBUG = process.env.CANVAS_DEBUG_DEVTOOLS_PANEL === '1'

export function selectionDebug(event: string, details?: Record<string, unknown>): void {
  if (!SELECTION_DEBUG) return
  console.log('[selection-debug:main]', { ts: Date.now(), event, ...details })
}

export function devtoolsPanelDebug(event: string, details?: Record<string, unknown>): void {
  if (!DEVTOOLS_PANEL_DEBUG) return
  console.log('[devtools-panel-debug:main]', { ts: Date.now(), event, ...details })
}
