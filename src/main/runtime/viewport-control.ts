// fallow-ignore-file circular-dependencies
// Suppressed: see #141. workspace-autosave → workspace-observers import viewport-control back
import { pages, pan, zoom, setPanState, setZoomState, selectedPage } from './runtime-context'
import { win } from './view-refs'
import { requestLayout } from './layout-engine'
import { markDirty } from './layout-dirty'
import {
  boundAvailableCanvasViewportRect as availableCanvasViewportRect,
  boundCanvasOriginX as canvasOriginX,
  boundEffectivePageContentSize as effectivePageContentSize,
} from './runtime-geometry'
import { scheduleWorkspaceAutosave } from './workspace-autosave'
import { safeSend } from './safe-send'
import { clampCanvasZoom } from '../../shared/zoom'
import { computeFocusZoomForBounds as computeFocusZoomForBoundsValue } from '../../shared/focus-camera'
import type { WorkspaceBounds } from '../../shared/types'
import { selectedCanvasTargets as uiSelectedCanvasTargets } from '../ui-state'
import { textEntities } from './text-entity-state'
import { fileEntities } from './file-entity-state'
import { drawingEntities } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { workspaceGroups, workspaceEdges } from './workspace-model'

export function setZoom(value: number): void {
  clearFocusReturnCamera()
  const nextZoom = clampCanvasZoom(value)
  if (nextZoom === zoom) return
  setZoomState(nextZoom)
  markDirty('canvas', 'toolbar')
  broadcastCanvasZoomToPages()
  scheduleWorkspaceAutosave()
}

export function broadcastCanvasZoomToPages(): void {
  for (const page of pages) {
    safeSend(page.pageView.webContents, 'set-canvas-zoom', zoom)
  }
}

export function setPan(x: number, y: number): void {
  clearFocusReturnCamera()
  if (pan.x === x && pan.y === y) return
  setPanState({ x, y })
  markDirty('canvas')
  scheduleWorkspaceAutosave()
}

// `requestLayout` lives in layout-engine (co-located with the private
// `layoutAllViews` it schedules); re-exported here so the viewport-control
// import surface stays stable.
export { requestLayout }

/** Pan offset that centers `bounds` in the available viewport at the current zoom. */
function panToCenterBounds(bounds: WorkspaceBounds): { x: number; y: number } {
  const viewport = availableCanvasViewportRect()
  return {
    x: Math.round(
      viewport.x + viewport.width / 2 - canvasOriginX() - (bounds.x + bounds.width / 2) * zoom,
    ),
    y: Math.round(viewport.height / 2 - (bounds.y + bounds.height / 2) * zoom),
  }
}

let focusReturnCamera: { zoom: number; pan: { x: number; y: number } } | null = null
let suppressFocusReturnClear = false

function clearFocusReturnCamera(): void {
  if (suppressFocusReturnClear) return
  focusReturnCamera = null
}

function setCameraForFocus(nextZoom: number, nextPan: { x: number; y: number }): void {
  suppressFocusReturnClear = true
  try {
    setZoom(nextZoom)
    setPan(nextPan.x, nextPan.y)
  } finally {
    suppressFocusReturnClear = false
  }
}

export function restoreFocusCamera(): boolean {
  const camera = focusReturnCamera
  if (!camera) return false
  focusReturnCamera = null
  setCameraForFocus(camera.zoom, camera.pan)
  requestLayout()
  return true
}

export function hasFocusReturnCamera(): boolean {
  return focusReturnCamera !== null
}

export function computeFocusZoomForBounds(
  bounds: WorkspaceBounds,
  viewport: { width: number; height: number },
): number {
  return computeFocusZoomForBoundsValue(bounds, viewport, zoom)
}

export function focusCanvasBounds(
  bounds: WorkspaceBounds,
  options?: { animate?: boolean },
): void {
  if (!win) return
  const target = panToCenterBounds(bounds)
  if (options?.animate) {
    animatePanTo(target.x, target.y)
    return
  }
  setPan(target.x, target.y)
  requestLayout()
}

// --- Camera animation ----------------------------------------------------
// Reusable pan tween for the main process. There's no requestAnimationFrame
// here, so we step on a ~60fps interval, easing pan from its current value to
// the target. Only one animation runs at a time; a new one (or any user-driven
// pan/zoom via `cancelCameraAnimation`) supersedes the previous.

const CAMERA_ANIMATION_DURATION_MS = 260
const CAMERA_ANIMATION_FRAME_MS = 16

let cameraAnimationTimer: ReturnType<typeof setInterval> | null = null

function easeOutCubic(t: number): number {
  return 1 - (1 - t) ** 3
}

/** Stop any in-flight camera animation. Safe to call when none is running. */
export function cancelCameraAnimation(): void {
  if (cameraAnimationTimer) {
    clearInterval(cameraAnimationTimer)
    cameraAnimationTimer = null
  }
}

/**
 * Smoothly pan the canvas to `(targetX, targetY)` while keeping the current
 * zoom. Reusable wherever the camera should glide rather than jump.
 */
export function animatePanTo(
  targetX: number,
  targetY: number,
  durationMs = CAMERA_ANIMATION_DURATION_MS,
): void {
  cancelCameraAnimation()
  const startX = pan.x
  const startY = pan.y
  const dx = targetX - startX
  const dy = targetY - startY
  if (dx === 0 && dy === 0) return
  const duration = Math.max(1, durationMs)
  const start = Date.now()
  cameraAnimationTimer = setInterval(() => {
    const t = Math.min(1, (Date.now() - start) / duration)
    const eased = easeOutCubic(t)
    setPan(Math.round(startX + dx * eased), Math.round(startY + dy * eased))
    requestLayout()
    if (t >= 1) cancelCameraAnimation()
  }, CAMERA_ANIMATION_FRAME_MS)
}

export function focusSelectedPage(): boolean {
  const page = selectedPage()
  if (!page || !win) return false
  const pageSize = effectivePageContentSize(page)
  const viewport = availableCanvasViewportRect()
  const targetX = Math.round(
    viewport.x + (viewport.width - pageSize.width * zoom) / 2 - canvasOriginX() - page.canvasX * zoom,
  )
  const targetY = Math.round(
    (viewport.height - pageSize.height * zoom) / 2 - page.canvasY * zoom,
  )
  setPan(targetX, targetY)
  requestLayout()
  return true
}

function resolveEntityBounds(entityId: string): WorkspaceBounds | null {
  const page = pages.find((p) => p.id === entityId)
  if (page) {
    const size = effectivePageContentSize(page)
    return { x: page.canvasX, y: page.canvasY, width: size.width, height: size.height }
  }
  const text = textEntities.find((e) => e.id === entityId)
  if (text) return { x: text.canvasX, y: text.canvasY, width: text.width, height: text.height }
  const file = fileEntities.find((e) => e.id === entityId)
  if (file) return { x: file.canvasX, y: file.canvasY, width: file.width, height: file.height }
  const drawing = drawingEntities.find((e) => e.id === entityId)
  if (drawing) return { x: drawing.canvasX, y: drawing.canvasY, width: drawing.width, height: drawing.height }
  const shape = shapeEntities.find((e) => e.id === entityId)
  if (shape) return { x: shape.canvasX, y: shape.canvasY, width: shape.width, height: shape.height }
  const group = workspaceGroups.find((g) => g.id === entityId)
  if (group) return { x: group.canvasX, y: group.canvasY, width: group.width, height: group.height }
  return null
}

function unionBounds(boundsArr: WorkspaceBounds[]): WorkspaceBounds | null {
  if (!boundsArr.length) return null
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const b of boundsArr) {
    minX = Math.min(minX, b.x)
    minY = Math.min(minY, b.y)
    maxX = Math.max(maxX, b.x + b.width)
    maxY = Math.max(maxY, b.y + b.height)
  }
  return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
}

export function focusSelection(options?: { storeReturnCamera?: boolean }): boolean {
  if (!win) return false
  const targets = uiSelectedCanvasTargets()
  if (!targets.length) return false

  const allBounds: WorkspaceBounds[] = []
  for (const { id, kind } of targets) {
    if (kind === 'edge') {
      const edge = workspaceEdges.find((e) => e.id === id)
      if (edge) {
        const fromBounds = resolveEntityBounds(edge.fromEntityId)
        const toBounds = resolveEntityBounds(edge.toEntityId)
        if (fromBounds) allBounds.push(fromBounds)
        if (toBounds) allBounds.push(toBounds)
      }
      continue
    }
    const bounds = resolveEntityBounds(id)
    if (bounds) allBounds.push(bounds)
  }

  const combined = unionBounds(allBounds)
  if (!combined) return false

  const viewport = availableCanvasViewportRect()
  const nextZoom = computeFocusZoomForBounds(combined, viewport)
  if (options?.storeReturnCamera !== false) {
    focusReturnCamera = { zoom, pan: { ...pan } }
  }
  setCameraForFocus(nextZoom, panToCenterBoundsAtZoom(combined, nextZoom))
  requestLayout()
  return true
}

function panToCenterBoundsAtZoom(bounds: WorkspaceBounds, targetZoom: number): { x: number; y: number } {
  const viewport = availableCanvasViewportRect()
  return {
    x: Math.round(
      viewport.x + viewport.width / 2 - canvasOriginX() - (bounds.x + bounds.width / 2) * targetZoom,
    ),
    y: Math.round(viewport.height / 2 - (bounds.y + bounds.height / 2) * targetZoom),
  }
}
