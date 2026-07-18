/**
 * Resize accumulator — pure, side-effect-free math for canvas entity resize.
 *
 * Shared here so the same arithmetic can be exercised by:
 *
 *   - the canvas-pointer-router's `begin-resize` dispatcher,
 *   - unit tests, without React or DOM.
 *
 * One source of truth for aspect-ratio locking, corner/edge flip logic,
 * delta accumulation, min-size clamping, and patch shape.
 *
 * Lives in src/shared so the router and tests can both import it.
 */

import { snapToGrid } from './gesture-utils'

// These types are re-declared here so this module stays free of renderer
// imports. Renderer-facing constants (handle size, cursor maps, min sizes)
// live in `src/renderer/canvas-bg/entityConstants.ts`.
export interface EntityResizePatch {
  width: number
  height: number
  canvasX?: number
  canvasY?: number
}
export type ResizeCorner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'
export type ResizeEdge = 'top' | 'right' | 'bottom' | 'left'
export type AspectRatioResizeMode = 'off' | 'shift-unlocks' | 'shift-locks'

export type ResizeHandle =
  | 'n' | 's' | 'e' | 'w'
  | 'ne' | 'nw' | 'se' | 'sw'

/** Initial dimensions captured at gesture begin. */
export interface ResizeStart {
  width: number
  height: number
  canvasX: number
  canvasY: number
}

/** Running accumulator — mutated in place by `applyResizeDelta`. */
export interface ResizeAccumulator extends ResizeStart {
  /** Cached aspect ratio (width/height at gesture start). */
  aspect: number
  /** Stationary edges captured exactly at gesture start. */
  fixedLeft: number
  fixedTop: number
  fixedRight: number
  fixedBottom: number
}

export interface ResizeConfig {
  minWidth: number
  minHeight: number
  aspectRatioResizeMode: AspectRatioResizeMode
}

/** Per-tick delta in screen pixels — converted to canvas-space inside. */
export interface ResizeDelta {
  /** Cumulative cursor movement since the previous tick, in screen pixels. */
  screenDx: number
  screenDy: number
  /** Current zoom (canvas-space px / screen-space px). */
  zoom: number
  /** True when shift is held this tick. Combined with `aspectRatioResizeMode`
   *  to decide whether to lock aspect on this tick specifically. */
  shiftKey: boolean
}

export function startResize(start: ResizeStart): ResizeAccumulator {
  return {
    ...start,
    aspect: start.height === 0 ? 1 : start.width / start.height,
    fixedLeft: start.canvasX,
    fixedTop: start.canvasY,
    fixedRight: start.canvasX + start.width,
    fixedBottom: start.canvasY + start.height,
  }
}

/**
 * Apply a delta from a corner handle, mutating the accumulator and returning
 * the patch the caller should send to main.
 */
export function applyCornerDelta(
  acc: ResizeAccumulator,
  corner: ResizeCorner,
  delta: ResizeDelta,
  config: ResizeConfig,
): EntityResizePatch {
  const flipX = corner === 'top-left' || corner === 'bottom-left' ? -1 : 1
  const flipY = corner === 'top-left' || corner === 'top-right' ? -1 : 1
  const dx = delta.screenDx / delta.zoom
  const dy = delta.screenDy / delta.zoom
  const aspectLock = shouldLockAspect(config.aspectRatioResizeMode, delta.shiftKey)

  let newW = Math.max(config.minWidth, acc.width + dx * flipX)
  let newH = Math.max(config.minHeight, acc.height + dy * flipY)
  let primary: 'w' | 'h' = 'w'
  if (aspectLock) {
    const dxAbs = Math.abs(newW - acc.width)
    const dyAbs = Math.abs(newH - acc.height)
    if (dxAbs >= dyAbs) {
      newH = Math.max(config.minHeight, newW / acc.aspect)
      newW = newH * acc.aspect
    } else {
      primary = 'h'
      newW = Math.max(config.minWidth, newH * acc.aspect)
      newH = newW / acc.aspect
    }
  }

  const clampedDx = (newW - acc.width) * flipX
  const clampedDy = (newH - acc.height) * flipY
  acc.width = newW
  acc.height = newH
  if (flipX === -1) acc.canvasX += clampedDx
  if (flipY === -1) acc.canvasY += clampedDy

  const movesLeft = flipX === -1
  const movesTop = flipY === -1
  let horizontal: AxisBounds
  let vertical: AxisBounds
  if (!aspectLock) {
    horizontal = snapHorizontalBounds(acc, movesLeft, config.minWidth)
    vertical = snapVerticalBounds(acc, movesTop, config.minHeight)
  } else if (primary === 'w') {
    horizontal = snapHorizontalBounds(
      acc,
      movesLeft,
      Math.max(config.minWidth, config.minHeight * acc.aspect),
    )
    const height = horizontal.size / acc.aspect
    vertical = boundsFromDerivedSize(acc.fixedTop, acc.fixedBottom, height, movesTop)
  } else {
    vertical = snapVerticalBounds(
      acc,
      movesTop,
      Math.max(config.minHeight, config.minWidth / acc.aspect),
    )
    const width = vertical.size * acc.aspect
    horizontal = boundsFromDerivedSize(acc.fixedLeft, acc.fixedRight, width, movesLeft)
  }

  const patch: EntityResizePatch = { width: horizontal.size, height: vertical.size }
  if (movesLeft) patch.canvasX = horizontal.start
  if (movesTop) patch.canvasY = vertical.start
  return patch
}

/**
 * Apply a delta from an edge handle. Edges only move along one axis; the
 * orthogonal dimension follows aspect-lock if active.
 */
export function applyEdgeDelta(
  acc: ResizeAccumulator,
  edge: ResizeEdge,
  delta: ResizeDelta,
  config: ResizeConfig,
): EntityResizePatch {
  const isHorizontal = edge === 'left' || edge === 'right'
  const flip = edge === 'left' || edge === 'top' ? -1 : 1
  const dx = delta.screenDx / delta.zoom
  const dy = delta.screenDy / delta.zoom
  const axisDelta = isHorizontal ? dx : dy
  const aspectLock = shouldLockAspect(config.aspectRatioResizeMode, delta.shiftKey)

  let newW: number
  let newH: number
  if (isHorizontal) {
    newW = Math.max(config.minWidth, acc.width + axisDelta * flip)
    newH = aspectLock ? newW / acc.aspect : acc.height
  } else {
    newH = Math.max(config.minHeight, acc.height + axisDelta * flip)
    newW = aspectLock ? newH * acc.aspect : acc.width
  }
  newW = Math.max(config.minWidth, newW)
  newH = Math.max(config.minHeight, newH)

  const dw = newW - acc.width
  const dh = newH - acc.height
  acc.width = newW
  acc.height = newH
  if (edge === 'left') acc.canvasX -= dw
  if (edge === 'top') acc.canvasY -= dh

  return isHorizontal
    ? horizontalEdgePatch(acc, edge === 'left', aspectLock, config)
    : verticalEdgePatch(acc, edge === 'top', aspectLock, config)
}

/**
 * Apply a delta from a corner-or-edge ResizeHandle (`nw`/`n`/`ne`/`e`/...).
 * Convenience for callers that already have the compact ADR-0001 handle ADT.
 */
export function applyHandleDelta(
  acc: ResizeAccumulator,
  handle: ResizeHandle,
  delta: ResizeDelta,
  config: ResizeConfig,
): EntityResizePatch {
  const corner = handleToCorner(handle)
  if (corner) return applyCornerDelta(acc, corner, delta, config)
  return applyEdgeDelta(acc, handleToEdge(handle), delta, config)
}

// --- Helpers ---

function shouldLockAspect(mode: AspectRatioResizeMode, shiftKey: boolean): boolean {
  if (mode === 'off') return false
  if (mode === 'shift-unlocks') return !shiftKey
  return shiftKey
}

interface AxisBounds {
  start: number
  size: number
}

function horizontalEdgePatch(
  acc: ResizeAccumulator,
  movesLeft: boolean,
  aspectLock: boolean,
  config: ResizeConfig,
): EntityResizePatch {
  const minWidth = aspectLock
    ? Math.max(config.minWidth, config.minHeight * acc.aspect)
    : config.minWidth
  const horizontal = snapHorizontalBounds(acc, movesLeft, minWidth)
  const height = aspectLock ? horizontal.size / acc.aspect : acc.height
  const patch: EntityResizePatch = { width: horizontal.size, height }
  if (movesLeft) patch.canvasX = horizontal.start
  return patch
}

function verticalEdgePatch(
  acc: ResizeAccumulator,
  movesTop: boolean,
  aspectLock: boolean,
  config: ResizeConfig,
): EntityResizePatch {
  const minHeight = aspectLock
    ? Math.max(config.minHeight, config.minWidth / acc.aspect)
    : config.minHeight
  const vertical = snapVerticalBounds(acc, movesTop, minHeight)
  const width = aspectLock ? vertical.size * acc.aspect : acc.width
  const patch: EntityResizePatch = { width, height: vertical.size }
  if (movesTop) patch.canvasY = vertical.start
  return patch
}

function snapHorizontalBounds(
  acc: ResizeAccumulator,
  movesLeft: boolean,
  minWidth: number,
): AxisBounds {
  const rawEdge = movesLeft ? acc.canvasX : acc.canvasX + acc.width
  const snappedEdge = snapToGrid(rawEdge)
  if (movesLeft) {
    const left = Math.min(snappedEdge, acc.fixedRight - minWidth)
    return { start: left, size: acc.fixedRight - left }
  }
  const right = Math.max(snappedEdge, acc.fixedLeft + minWidth)
  return { start: acc.fixedLeft, size: right - acc.fixedLeft }
}

function snapVerticalBounds(
  acc: ResizeAccumulator,
  movesTop: boolean,
  minHeight: number,
): AxisBounds {
  const rawEdge = movesTop ? acc.canvasY : acc.canvasY + acc.height
  const snappedEdge = snapToGrid(rawEdge)
  if (movesTop) {
    const top = Math.min(snappedEdge, acc.fixedBottom - minHeight)
    return { start: top, size: acc.fixedBottom - top }
  }
  const bottom = Math.max(snappedEdge, acc.fixedTop + minHeight)
  return { start: acc.fixedTop, size: bottom - acc.fixedTop }
}

function boundsFromDerivedSize(
  fixedStart: number,
  fixedEnd: number,
  size: number,
  movesStart: boolean,
): AxisBounds {
  return {
    start: movesStart ? fixedEnd - size : fixedStart,
    size,
  }
}

function handleToCorner(handle: ResizeHandle): ResizeCorner | null {
  switch (handle) {
    case 'nw': return 'top-left'
    case 'ne': return 'top-right'
    case 'sw': return 'bottom-left'
    case 'se': return 'bottom-right'
    default: return null
  }
}

function handleToEdge(handle: ResizeHandle): ResizeEdge {
  switch (handle) {
    case 'n': return 'top'
    case 's': return 'bottom'
    case 'e': return 'right'
    case 'w': return 'left'
    default:
      // Unreachable given handleToCorner exhausts the corners.
      throw new Error(`handleToEdge: unexpected handle ${handle}`)
  }
}
