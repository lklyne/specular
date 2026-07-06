/**
 * Tool defaults — per-tool, persistent app settings (ADR 0008 §9).
 *
 * Stored in the user's preferences file (not in `.canvas`, not in Y.Doc, not
 * in undo/redo). The next entity created by a tool stamps these values; the
 * tool-mode popup reads/writes them.
 *
 * Plain-text color remains nullable so text can inherit canvas foreground /
 * `currentColor` by default. Sticky owns its own first-class defaults now that
 * it is a separate creation tool.
 */

import type { DrawingBrushType, ShapeKind } from './types'
import { isShapeKind } from './shapes'

export interface ToolDefaults {
  'add-text': {
    color: string | null
    textSize: number
  }
  'add-sticky': {
    color: string
    textSize: number
  }
  'add-shape': {
    shapeKind: ShapeKind
    color: string
    strokeWidth: number
    textSize: number
  }
  draw: {
    brushType: DrawingBrushType
    color: string
    strokeWidth: number
  }
}

/**
 * First-time-launch defaults. ADR 0008 §"Tool defaults" originally specified
 * shape/draw at black; we shifted both to red so a brand-new canvas draws in
 * colour rather than indistinguishable-from-text black.
 *   sticky yellow, plain transparent/inherit (null), text small,
 *   shape rectangle/red/2px (legacy preset → muted red),
 *   draw pen/red/2px (punchy red — the pen brush inks in the vivid palette).
 */
export const DEFAULT_TOOL_DEFAULTS: ToolDefaults = {
  'add-text': {
    color: null,
    textSize: 14,
  },
  'add-sticky': {
    color: '3', // yellow preset
    textSize: 14,
  },
  'add-shape': {
    shapeKind: 'rectangle',
    color: '1', // red preset
    strokeWidth: 2,
    textSize: 14,
  },
  draw: {
    brushType: 'pen',
    color: '1', // red preset — resolves to vivid red for the pen brush
    strokeWidth: 2,
  },
}

/**
 * Merge a partial (possibly malformed) persisted blob over the defaults so
 * older preference files without certain keys still load cleanly.
 */
export function normalizeToolDefaults(
  raw: unknown,
): ToolDefaults {
  if (!raw || typeof raw !== 'object') return cloneToolDefaults(DEFAULT_TOOL_DEFAULTS)
  const merged = cloneToolDefaults(DEFAULT_TOOL_DEFAULTS)
  const obj = raw as Partial<ToolDefaults>
  if (obj['add-text'] && typeof obj['add-text'] === 'object') {
    const t = obj['add-text']
    if (typeof t.color === 'string' || t.color === null) merged['add-text'].color = t.color
    if (typeof t.textSize === 'number' && Number.isFinite(t.textSize))
      merged['add-text'].textSize = t.textSize
    const legacy = t as { 'plain.color'?: unknown }
    if (typeof legacy['plain.color'] === 'string' || legacy['plain.color'] === null)
      merged['add-text'].color = legacy['plain.color']
  }
  if (obj['add-sticky'] && typeof obj['add-sticky'] === 'object') {
    const s = obj['add-sticky']
    if (typeof s.color === 'string') merged['add-sticky'].color = s.color
    if (typeof s.textSize === 'number' && Number.isFinite(s.textSize))
      merged['add-sticky'].textSize = s.textSize
  } else if (obj['add-text'] && typeof obj['add-text'] === 'object') {
    const legacy = obj['add-text'] as { 'sticky.color'?: unknown }
    if (typeof legacy['sticky.color'] === 'string') {
      merged['add-sticky'].color = legacy['sticky.color']
    }
  }
  if (obj['add-shape'] && typeof obj['add-shape'] === 'object') {
    const s = obj['add-shape']
    if (isShapeKind(s.shapeKind)) merged['add-shape'].shapeKind = s.shapeKind
    if (typeof s.color === 'string') merged['add-shape'].color = s.color
    if (typeof s.strokeWidth === 'number' && Number.isFinite(s.strokeWidth))
      merged['add-shape'].strokeWidth = s.strokeWidth
    if (typeof s.textSize === 'number' && Number.isFinite(s.textSize))
      merged['add-shape'].textSize = s.textSize
  }
  if (obj.draw && typeof obj.draw === 'object') {
    const d = obj.draw
    if (d.brushType === 'pen' || d.brushType === 'highlight') merged.draw.brushType = d.brushType
    if (typeof d.color === 'string') merged.draw.color = d.color
    if (typeof d.strokeWidth === 'number' && Number.isFinite(d.strokeWidth))
      merged.draw.strokeWidth = d.strokeWidth
  }
  return merged
}

function cloneToolDefaults(src: ToolDefaults): ToolDefaults {
  return {
    'add-text': { ...src['add-text'] },
    'add-sticky': { ...src['add-sticky'] },
    'add-shape': { ...src['add-shape'] },
    draw: { ...src.draw },
  }
}

/**
 * A typed patch for `setToolDefault`. Each variant updates one scope's value.
 * Keeping this discriminated lets IPC carry typed updates and the main-side
 * setter narrow without parsing.
 */
export type ToolDefaultPatch =
  | { scope: 'add-text'; key: 'color'; value: string | null }
  | { scope: 'add-text'; key: 'textSize'; value: number }
  | { scope: 'add-sticky'; key: 'color'; value: string }
  | { scope: 'add-sticky'; key: 'textSize'; value: number }
  | { scope: 'add-shape'; key: 'shapeKind'; value: ShapeKind }
  | { scope: 'add-shape'; key: 'color'; value: string }
  | { scope: 'add-shape'; key: 'strokeWidth'; value: number }
  | { scope: 'add-shape'; key: 'textSize'; value: number }
  | { scope: 'draw'; key: 'brushType'; value: DrawingBrushType }
  | { scope: 'draw'; key: 'color'; value: string }
  | { scope: 'draw'; key: 'strokeWidth'; value: number }
