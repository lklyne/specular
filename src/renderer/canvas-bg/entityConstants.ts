// Shared constants for canvas entity resize and layout.

// The resize ADT lives in src/shared/resize-accumulator.ts so renderer
// hooks and the canvas-pointer-router agree on shapes.
import type {
  AspectRatioResizeMode,
  ResizeCorner,
  ResizeEdge,
} from '../../shared/resize-accumulator'
export type {
  ResizeCorner,
  ResizeEdge,
  AspectRatioResizeMode,
}

export {
  IMAGE_EXTENSIONS,
  VIDEO_EXTENSIONS,
  MARKDOWN_EXTENSIONS,
  WIREFRAME_EXTENSIONS,
  HTML_EXTENSIONS,
} from '../../shared/file-extensions'
import { IMAGE_EXTENSIONS, VIDEO_EXTENSIONS, MARKDOWN_EXTENSIONS, WIREFRAME_EXTENSIONS } from '../../shared/file-extensions'
import { RESIZE_HANDLE_VISUAL_PX } from '../../shared/canvas-hit-geometry'

export function fileDisplayName(file: string): string {
  const base = file.split('/').pop() ?? file
  if (WIREFRAME_EXTENSIONS.test(file)) return base.replace(/\.wireframe\.json$/i, '')
  if (MARKDOWN_EXTENSIONS.test(file)) return base.replace(/\.md$/i, '')
  return base
}

/** Images/videos: lock aspect unless Shift. Other files: free resize unless Shift (then lock). */
export function aspectRatioResizeModeForCanvasFile(filePath: string): AspectRatioResizeMode {
  if (IMAGE_EXTENSIONS.test(filePath) || VIDEO_EXTENSIONS.test(filePath)) return 'shift-unlocks'
  return 'shift-locks'
}

export const HANDLE_SIZE = RESIZE_HANDLE_VISUAL_PX

export const MIN_GROUP_WIDTH = 120
export const MIN_GROUP_HEIGHT = 80
export const MIN_TEXT_WIDTH = 100
export const MIN_TEXT_HEIGHT = 60
export const MIN_FILE_WIDTH = 80
export const MIN_FILE_HEIGHT = 80
export const MIN_SHAPE_WIDTH = 24
export const MIN_SHAPE_HEIGHT = 24

export const CORNER_CURSORS: Record<ResizeCorner, string> = {
  'top-left': 'nwse-resize',
  'top-right': 'nesw-resize',
  'bottom-left': 'nesw-resize',
  'bottom-right': 'nwse-resize',
}

export const EDGE_CURSORS: Record<ResizeEdge, string> = {
  'top': 'ns-resize',
  'right': 'ew-resize',
  'bottom': 'ns-resize',
  'left': 'ew-resize',
}
