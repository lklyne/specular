import type { PageConfig } from './types'

// Device dimensions are defined in device-catalog.ts (single source of truth).
export { VIEWPORT_PRESETS, DESKTOP_PRESET_INDEX } from './device-catalog'

export const PLAIN_TEXT_PLACEHOLDER = 'Add text'

/**
 * A sticky's height at the default text size — what a fresh note is created
 * at, and the floor its measured height is held to so an empty or one-line
 * note stays note-shaped instead of hugging its text down to a strip of
 * padding.
 *
 * Not a hard minimum. The floor scales with the note's text size, because
 * scaling is what a corner or n/s handle drag means for a sticky: the font
 * tracks the drag and the box follows the text. A fixed floor would stop the
 * box at this height while the font kept shrinking under it.
 */
export const STICKY_BASE_HEIGHT = 200

export const TOOLBAR_HEIGHT = 44
export const GRID_SIZE = 20
export const NUDGE_STEP = 5
export const USER_GROUP_PADDING = 24
export const CLUSTER_HORIZONTAL_GUTTER = 80
export const CLUSTER_VERTICAL_GUTTER = 80
export const CLUSTER_OUTER_MARGIN = 80
export const PLACEMENT_SCAN_STEP = GRID_SIZE
export const ANCHOR_OFFSET_X = 80
export const ANCHOR_OFFSET_Y = 0
export const DEFAULT_REMOTE_DEBUGGING_PORT = 9333
export const APP_CONTROL_PORT = 29979
export const APP_CONTROL_VERSION = '1'
export const APP_CONTROL_DISCOVERY_FILE = 'specular-mcp.json'

export const DEFAULT_PAGES: PageConfig[] = [
  {
    url: 'https://tailwindcss.com',
    presetIndex: 1, // iPhone 14 Pro
    canvasX: 40,
    canvasY: 40,
    source: 'manual',
  },
  {
    url: 'https://tailwindcss.com',
    presetIndex: 6, // Laptop
    canvasX: 500,
    canvasY: 40,
    source: 'manual',
  },
]

export const DEFAULT_BREAKPOINT_PRESET_LABELS = [
  'iPhone 14 Pro',
  'iPad Mini',
  'Desktop',
]
