/**
 * Multi-resize accumulator — pure proportional-scale math for the multi-
 * selection bounding box drag.
 *
 * Sister module to `resize-accumulator.ts`. The single-entity accumulator
 * carries aspect-lock + min-size config for one entity; this module operates
 * on a bbox over many entities and rescales each one's position + dimensions
 * by the bbox's scale ratio.
 *
 * Pure: no Electron, no DOM. The router calls `startMultiResize` once at
 * pointerdown, then `applyMultiHandleDelta` per move tick to produce the
 * `MultiResizeEntry[]` payload for `api.resizeMultiSelection`.
 */

import type { AnnotationDrawingStroke, CanvasSceneEntity } from './types'
import type { ResizeHandle } from './resize-accumulator'
import { scaleStrokes } from './scale-strokes'

/** Kinds that the `resizeMultiSelection` IPC accepts. Groups own a separate
 *  selection overlay and are excluded from the multi-bbox gesture. */
export type MultiResizableKind = 'page' | 'text' | 'file' | 'drawing' | 'shape'

/** Bbox cannot collapse below this canvas-space size while resizing. */
export const MIN_MULTI_BBOX = 20

export interface MultiResizeEntity {
  id: string
  kind: MultiResizableKind
  canvasX: number
  canvasY: number
  width: number
  height: number
  /** Initial strokes snapshot for drawing entities — undefined for all other kinds. */
  strokes?: AnnotationDrawingStroke[]
}

export interface MultiResizeBbox {
  x: number
  y: number
  width: number
  height: number
}

export interface MultiResizeStart {
  entities: readonly MultiResizeEntity[]
  bbox: MultiResizeBbox
}

export interface MultiResizeAccumulator {
  initialEntities: readonly MultiResizeEntity[]
  initialBbox: MultiResizeBbox
  /** Running bbox in canvas space — mutated by `applyMultiHandleDelta`. */
  accX: number
  accY: number
  accW: number
  accH: number
}

export interface MultiResizeEntry {
  id: string
  kind: MultiResizableKind
  canvasX: number
  canvasY: number
  width: number
  height: number
  /** Scaled strokes for drawing entities — undefined for all other kinds. */
  strokes?: AnnotationDrawingStroke[]
}

export interface MultiResizeDelta {
  /** Cumulative cursor movement since the previous tick, in screen pixels. */
  screenDx: number
  screenDy: number
  /** Current zoom (canvas-space px / screen-space px). */
  zoom: number
}

/**
 * Aggregate the canvas-space bbox of the given entity ids. Returns null
 * when fewer than two ids are selected or none match — multi-resize is a
 * 2+-entity gesture, so callers that get null should fall through to the
 * single-entity path.
 */
export function computeMultiSelectionBbox(
  entities: readonly CanvasSceneEntity[],
  selectedEntityIds: readonly string[],
): { bbox: MultiResizeBbox; entities: MultiResizeEntity[] } | null {
  if (selectedEntityIds.length < 2) return null
  const idSet = new Set(selectedEntityIds)
  const out: MultiResizeEntity[] = []
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const e of entities) {
    if (!idSet.has(e.id)) continue
    if (e.kind === 'group') continue // groups have their own selection overlay
    out.push({
      id: e.id,
      kind: e.kind satisfies MultiResizableKind,
      canvasX: e.canvasX,
      canvasY: e.canvasY,
      width: e.width,
      height: e.height,
      strokes: e.kind === 'drawing' ? e.strokes : undefined,
    })
    minX = Math.min(minX, e.canvasX)
    minY = Math.min(minY, e.canvasY)
    maxX = Math.max(maxX, e.canvasX + e.width)
    maxY = Math.max(maxY, e.canvasY + e.height)
  }
  if (out.length < 2) return null
  return {
    entities: out,
    bbox: { x: minX, y: minY, width: maxX - minX, height: maxY - minY },
  }
}

export function startMultiResize(start: MultiResizeStart): MultiResizeAccumulator {
  return {
    initialEntities: start.entities.map((e) => ({ ...e })),
    initialBbox: { ...start.bbox },
    accX: start.bbox.x,
    accY: start.bbox.y,
    accW: start.bbox.width,
    accH: start.bbox.height,
  }
}

/**
 * Apply a screen-space delta to the accumulator and return the proportional
 * placement of every initial entity inside the new bbox.
 *
 * Mutates `acc` so successive ticks compose like the single-entity accumulator.
 */
export function applyMultiHandleDelta(
  acc: MultiResizeAccumulator,
  handle: ResizeHandle,
  delta: MultiResizeDelta,
): MultiResizeEntry[] {
  const dx = delta.screenDx / delta.zoom
  const dy = delta.screenDy / delta.zoom

  const flipX = handle === 'nw' || handle === 'sw' || handle === 'w' ? -1 : 1
  const flipY = handle === 'nw' || handle === 'ne' || handle === 'n' ? -1 : 1
  const usesX = handle !== 'n' && handle !== 's'
  const usesY = handle !== 'e' && handle !== 'w'

  if (usesX) {
    const newW = Math.max(MIN_MULTI_BBOX, acc.accW + dx * flipX)
    const dw = newW - acc.accW
    acc.accW = newW
    if (flipX === -1) acc.accX -= dw
  }
  if (usesY) {
    const newH = Math.max(MIN_MULTI_BBOX, acc.accH + dy * flipY)
    const dh = newH - acc.accH
    acc.accH = newH
    if (flipY === -1) acc.accY -= dh
  }

  const scaleX = acc.initialBbox.width > 0 ? acc.accW / acc.initialBbox.width : 1
  const scaleY = acc.initialBbox.height > 0 ? acc.accH / acc.initialBbox.height : 1

  return acc.initialEntities.map((entity) => {
    const entry: MultiResizeEntry = {
      id: entity.id,
      kind: entity.kind,
      width: Math.round(Math.max(1, entity.width * scaleX)),
      height: Math.round(Math.max(1, entity.height * scaleY)),
      canvasX: Math.round(acc.accX + (entity.canvasX - acc.initialBbox.x) * scaleX),
      canvasY: Math.round(acc.accY + (entity.canvasY - acc.initialBbox.y) * scaleY),
    }
    if (entity.strokes) {
      entry.strokes = scaleStrokes(entity.strokes, scaleX, scaleY)
    }
    return entry
  })
}
