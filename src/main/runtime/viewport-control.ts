// fallow-ignore-file circular-dependencies
// Suppressed: see #141. workspace-autosave → workspace-observers import viewport-control back
import {
  focusPresentationOverride,
  pages,
  pan,
  zoom,
  setFocusPresentationOverride,
  setPanState,
  setZoomState,
  selectedPage,
} from './runtime-context'
import { win } from './view-refs'
import { requestLayout } from './layout-engine'
import { markDirty } from './layout-dirty'
import {
  boundAvailableCanvasViewportRect as availableCanvasViewportRect,
  boundCanvasOriginX as canvasOriginX,
  boundEffectivePageContentSize as effectivePageContentSize,
  pageContentSize,
  pageVisualBoundsForContentSize,
} from './runtime-geometry'
import { scheduleWorkspaceAutosave } from './workspace-autosave'
import { safeSend } from './safe-send'
import { clampCanvasZoom } from '../../shared/zoom'
import {
  computeFocusZoomForBounds as computeFocusZoomForBoundsValue,
  computePanToCenterBoundsAtZoom as computePanToCenterBoundsAtZoomValue,
} from '../../shared/focus-camera'
import {
  CAMERA_TRANSITION_FRAME_MS,
  DEFAULT_CAMERA_TRANSITION_DURATION_MS,
  type CanvasCamera,
  interpolateCamera,
} from '../../shared/camera-transition'
import type { FocusPresentationMode, WorkspaceBounds } from '../../shared/types'
import { selectedCanvasTargets as uiSelectedCanvasTargets } from '../ui-state'
import { textEntities } from './text-entity-state'
import { fileEntities } from './file-entity-state'
import { drawingEntities } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { workspaceGroups, workspaceEdges } from './workspace-model'
import { pageUsesCustomSize } from './runtime-entities'

export function setZoom(value: number): void {
  if (!suppressCameraAnimationCancel) cancelCameraAnimation()
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
  if (!suppressCameraAnimationCancel) cancelCameraAnimation()
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
  return panToCenterBoundsAtZoom(bounds, zoom)
}

let focusReturnCamera: { zoom: number; pan: { x: number; y: number } } | null = null
let suppressFocusReturnClear = false
let suppressCameraAnimationCancel = false

function clearFocusReturnCamera(): void {
  if (suppressFocusReturnClear) return
  focusReturnCamera = null
  setFocusPresentationOverride(null)
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
  setFocusPresentationOverride(null)
  animateCameraTo(camera, {
    durationMs: DEFAULT_CAMERA_TRANSITION_DURATION_MS / 2,
    preserveFocusSession: true,
  })
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

function computeFocusZoomForPresentation(
  bounds: WorkspaceBounds,
  viewport: { width: number; height: number },
  mode: FocusPresentationMode | null,
): number {
  if (mode !== 'fit') return computeFocusZoomForBoundsValue(bounds, viewport, zoom)
  if (bounds.width <= 0 || bounds.height <= 0) return Math.min(zoom, 1)
  const availableWidth = Math.max(1, viewport.width - 128)
  const availableHeight = Math.max(1, viewport.height - 128)
  return clampCanvasZoom(Math.min(availableWidth / bounds.width, availableHeight / bounds.height))
}

export function focusCanvasBounds(
  bounds: WorkspaceBounds,
  options?: { animate?: boolean },
): void {
  if (!win) return
  moveCameraTo(
    { zoom, pan: panToCenterBoundsAtZoom(bounds, zoom) },
    { animate: options?.animate ?? false },
  )
}

// --- Camera animation ----------------------------------------------------
// Reusable camera tween for the main process. There's no requestAnimationFrame
// here, so we step on a ~60fps interval, easing zoom + pan together. Only one
// animation runs at a time; a new one (or any user-driven pan/zoom via
// `cancelCameraAnimation`) supersedes the previous.

let cameraAnimationTimer: ReturnType<typeof setInterval> | null = null

export type CameraMoveOptions = {
  animate?: boolean
  durationMs?: number
  preserveFocusSession?: boolean
}

function currentCamera(): CanvasCamera {
  return { zoom, pan: { ...pan } }
}

function normalizeCamera(camera: CanvasCamera): CanvasCamera {
  return {
    zoom: clampCanvasZoom(camera.zoom),
    pan: {
      x: Math.round(camera.pan.x),
      y: Math.round(camera.pan.y),
    },
  }
}

function camerasEqual(a: CanvasCamera, b: CanvasCamera): boolean {
  return a.zoom === b.zoom && a.pan.x === b.pan.x && a.pan.y === b.pan.y
}

function applyCamera(camera: CanvasCamera, preserveFocusSession: boolean): void {
  suppressCameraAnimationCancel = true
  try {
    if (preserveFocusSession) {
      setCameraForFocus(camera.zoom, camera.pan)
      return
    }
    setZoom(camera.zoom)
    setPan(camera.pan.x, camera.pan.y)
  } finally {
    suppressCameraAnimationCancel = false
  }
}

/** Stop any in-flight camera animation. Safe to call when none is running. */
export function cancelCameraAnimation(): void {
  if (cameraAnimationTimer) {
    clearInterval(cameraAnimationTimer)
    cameraAnimationTimer = null
  }
}

/**
 * Move the canvas camera, optionally animating zoom + pan together. Use this
 * for new programmatic camera moves so callers only choose the target and
 * whether the move should preserve an active focus-return session.
 */
export function moveCameraTo(targetCamera: CanvasCamera, options: CameraMoveOptions = {}): void {
  cancelCameraAnimation()
  const target = normalizeCamera(targetCamera)
  const startCamera = currentCamera()
  const preserveFocusSession = options.preserveFocusSession === true
  const duration = Math.max(0, options.durationMs ?? DEFAULT_CAMERA_TRANSITION_DURATION_MS)
  if (!options.animate || duration === 0 || camerasEqual(startCamera, target)) {
    applyCamera(target, preserveFocusSession)
    requestLayout()
    return
  }

  const start = Date.now()
  cameraAnimationTimer = setInterval(() => {
    const t = Math.min(1, (Date.now() - start) / duration)
    applyCamera(interpolateCamera(startCamera, target, t), preserveFocusSession)
    requestLayout()
    if (t >= 1) cancelCameraAnimation()
  }, CAMERA_TRANSITION_FRAME_MS)
}

export function animateCameraTo(
  targetCamera: CanvasCamera,
  options: Omit<CameraMoveOptions, 'animate'> = {},
): void {
  moveCameraTo(targetCamera, { ...options, animate: true })
}

/**
 * Smoothly pan the canvas to `(targetX, targetY)` while keeping the current
 * zoom. Kept as a small compatibility wrapper around the universal camera
 * transition helper.
 */
export function animatePanTo(
  targetX: number,
  targetY: number,
  durationMs = DEFAULT_CAMERA_TRANSITION_DURATION_MS,
): void {
  animateCameraTo({ zoom, pan: { x: targetX, y: targetY } }, { durationMs })
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
  moveCameraTo({ zoom, pan: { x: targetX, y: targetY } })
  return true
}

function resolveEntityBounds(entityId: string): WorkspaceBounds | null {
  const page = pages.find((p) => p.id === entityId)
  if (page) {
    return focusPresentationOverride?.pageId === page.id
      ? focusPageBounds(page.id, focusPresentationOverride.mode)
      : pageVisualBoundsForContentSize(page, effectivePageContentSize(page))
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

function defaultFocusPresentationMode(page: { metadata?: Record<string, unknown> }): FocusPresentationMode {
  return pageUsesCustomSize(page.metadata) ? 'responsive' : 'fit'
}

function responsiveFocusPageSize(): { width: number; height: number } {
  const viewport = availableCanvasViewportRect()
  return {
    width: Math.max(320, Math.round(viewport.width - 128)),
    height: Math.max(200, Math.round(viewport.height - 128)),
  }
}

function focusPageContentSize(
  page: Parameters<typeof pageContentSize>[0] & { id?: string },
  mode: FocusPresentationMode,
): { width: number; height: number } {
  if (mode === 'responsive') return responsiveFocusPageSize()
  return pageContentSize(page)
}

function focusPageBounds(pageId: string, mode: FocusPresentationMode): WorkspaceBounds | null {
  const page = pages.find((candidate) => candidate.id === pageId)
  if (!page) return null
  const size = focusPageContentSize(page, mode)
  return pageVisualBoundsForContentSize(page, size)
}

export function setFocusPresentationMode(mode: FocusPresentationMode): boolean {
  const current = focusPresentationOverride
  if (!current || !focusReturnCamera) return false
  const bounds = focusPageBounds(current.pageId, mode)
  if (!bounds) return false
  setFocusPresentationOverride({ pageId: current.pageId, mode })
  recenterFocusPresentation()
  return true
}

export function recenterFocusPresentation(
  pageId?: string,
  options?: { animate?: boolean },
): boolean {
  const current = focusPresentationOverride
  if (!current || (pageId && current.pageId !== pageId)) return false
  const bounds = focusPageBounds(current.pageId, current.mode)
  if (!bounds) return false
  const viewport = availableCanvasViewportRect()
  const nextZoom = computeFocusZoomForPresentation(bounds, viewport, current.mode)
  moveCameraTo(
    { zoom: nextZoom, pan: panToCenterBoundsAtZoom(bounds, nextZoom) },
    {
      animate: options?.animate ?? true,
      preserveFocusSession: true,
    },
  )
  return true
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

export function focusSelection(options?: { storeReturnCamera?: boolean; animate?: boolean }): boolean {
  if (!win) return false
  const targets = uiSelectedCanvasTargets()
  if (!targets.length) return false

  const singlePageTarget =
    targets.length === 1 && targets[0]?.kind === 'page'
      ? pages.find((page) => page.id === targets[0]!.id) ?? null
      : null
  const shouldCreateFocusSession = options?.storeReturnCamera !== false
  const focusMode = singlePageTarget && shouldCreateFocusSession
    ? defaultFocusPresentationMode(singlePageTarget)
    : null
  if (singlePageTarget && focusMode) {
    setFocusPresentationOverride({ pageId: singlePageTarget.id, mode: focusMode })
  } else {
    setFocusPresentationOverride(null)
  }

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
  const nextZoom = computeFocusZoomForPresentation(combined, viewport, focusMode)
  if (options?.storeReturnCamera !== false) {
    focusReturnCamera = { zoom, pan: { ...pan } }
  }
  moveCameraTo(
    { zoom: nextZoom, pan: panToCenterBoundsAtZoom(combined, nextZoom) },
    {
      animate: options?.animate ?? true,
      preserveFocusSession: options?.storeReturnCamera !== false,
    },
  )
  return true
}

function panToCenterBoundsAtZoom(bounds: WorkspaceBounds, targetZoom: number): { x: number; y: number } {
  return computePanToCenterBoundsAtZoomValue({
    bounds,
    viewport: availableCanvasViewportRect(),
    canvasOriginX: canvasOriginX(),
    zoom: targetZoom,
  })
}
