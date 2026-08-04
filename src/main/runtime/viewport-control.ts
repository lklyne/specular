// fallow-ignore-file circular-dependencies
// Suppressed: see #141. space-autosave → space-observers import viewport-control back
import { ipcChannels } from '../../shared/ipc-contract'
import {
  interactivePageId,
  pages,
  pan,
  zoom,
  setCameraTransitionStartedAt,
  setInteractivePageId,
  setPanState,
  setZoomState,
  selectedPage,
} from './runtime-context'
import { sendInteractiveState } from './overlay-manager'
import {
  beginFocusSession,
  endFocusSession,
  focusSession,
  focusedPageId,
  isFocusSessionActive,
  repointFocusSession,
  setFocusSessionMode,
  setFocusAnnotationsVisible as setSessionAnnotationsVisible,
  type FocusExitReason,
  type FocusSession,
} from './focus-session'
import {
  beginEditingEntity,
  commitEditingEntity,
  currentEditingEntityId,
} from './editing-entity-runtime'
import { win } from './view-refs'
import { requestLayout } from './layout-engine'
import { markDirty } from './layout-dirty'
import {
  boundAvailableCanvasViewportRect as availableCanvasViewportRect,
  boundCanvasOrigin as canvasOrigin,
  boundCanvasOriginX as canvasOriginX,
  boundEffectivePageContentSize as effectivePageContentSize,
  pageContentSize,
  pageVisualBoundsForContentSize,
} from './runtime-geometry'
import { scheduleSpaceAutosave } from './space-autosave'
import { broadcastViewportNudge } from './viewport-nudge'
import { safeSend } from './safe-send'
import { clampCanvasZoom } from '../../shared/zoom'
import {
  FOCUS_VIEWPORT_PADDING_PX,
  computeFocusZoomForBounds as computeFocusZoomForBoundsValue,
  computePanToCenterBoundsAtZoom as computePanToCenterBoundsAtZoomValue,
  isBoundsFullyVisibleInCamera,
} from '../../shared/focus-camera'
import {
  CAMERA_TRANSITION_FRAME_MS,
  DEFAULT_CAMERA_TRANSITION_DURATION_MS,
  FOCUS_CAMERA_TRANSITION_DURATION_MS,
  type CanvasCamera,
  interpolateCamera,
} from '../../shared/camera-transition'
import type { FocusPresentationMode, WorkspaceBounds } from '../../shared/types'
import { isWorkingTool } from '../../shared/tool'
import {
  activeTool as uiActiveTool,
  selectedCanvasTargets as uiSelectedCanvasTargets,
} from '../ui-state'
import { textEntities } from './text-entity-state'
import { fileEntities, fileEntitySupportsFillFocus } from './file-entity-state'
import { drawingEntities } from './drawing-entity-state'
import { shapeEntities } from './shape-entity-state'
import { workspaceGroups, workspaceEdges } from './space-model'
import { pageUsesCustomSize } from './runtime-entities'

export function setZoom(value: number): void {
  if (!suppressCameraAnimationCancel) cancelCameraAnimation()
  endFocusOnCameraChange()
  const nextZoom = clampCanvasZoom(value)
  if (nextZoom === zoom) return
  setZoomState(nextZoom)
  markDirty('canvas', 'toolbar')
  broadcastViewportNudge()
  broadcastCanvasZoomToPages()
  if (!suppressCameraAutosave) scheduleSpaceAutosave()
}

export function broadcastCanvasZoomToPages(): void {
  for (const page of pages) {
    safeSend(page.pageView.webContents, ipcChannels.setCanvasZoom, zoom)
  }
}

export function setPan(x: number, y: number): void {
  if (!suppressCameraAnimationCancel) cancelCameraAnimation()
  endFocusOnCameraChange()
  if (pan.x === x && pan.y === y) return
  setPanState({ x, y })
  markDirty('canvas')
  broadcastViewportNudge()
  if (!suppressCameraAutosave) scheduleSpaceAutosave()
}

// `requestLayout` lives in layout-engine (co-located with the private
// `layoutAllViews` it schedules); re-exported here so the viewport-control
// import surface stays stable.
export { requestLayout }

/** Pan offset that centers `bounds` in the available viewport at the current zoom. */
function panToCenterBounds(bounds: WorkspaceBounds): { x: number; y: number } {
  return panToCenterBoundsAtZoom(bounds, zoom)
}

let suppressFocusReturnClear = false
let suppressCameraAnimationCancel = false
// Per-frame camera tweens shouldn't fire scheduleSpaceAutosave() (full
// runtime→Y.Doc diff-sync + debounce reset) ~20× per transition. applyCamera
// suppresses it; moveCameraTo schedules a single autosave when the move lands.
let suppressCameraAutosave = false

// A user camera move (setZoom/setPan) ends the focus session — except:
//  - programmatic focus moves set `suppressFocusReturnClear` (they reframe the
//    focused page, they don't leave it);
//  - while a working tool is active, you're annotating/placing, not leaving, so
//    the session survives the camera change (ADR 0021 tightening).
function endFocusOnCameraChange(): void {
  if (suppressFocusReturnClear) return
  if (!isFocusSessionActive()) return
  if (isWorkingTool(uiActiveTool())) return
  exitFocusSession('camera-change')
}

// The single teardown path for leaving a focus session: end it, close the note
// editor the session opened, and drop page interactivity back to selected-only.
// Every exit needs all three, so they live together here.
function exitFocusSession(reason: FocusExitReason): void {
  commitEditIfSessionFile(endFocusSession(reason))
  syncInteractiveToFocus()
}

// A file-target session hands the note to the inline markdown editor on entry;
// leaving the session has to close that editor so the edit lands in the doc.
function commitEditIfSessionFile(session: FocusSession | null): void {
  if (session?.target.kind !== 'file') return
  if (currentEditingEntityId() !== session.target.id) return
  commitEditingEntity()
}

// Focus is the second click (#124): entering a focus session immediately makes
// the focused page interactive — no extra deliberate click — and leaving focus
// drops it back to selected-only. Call right after any beginFocusSession /
// endFocusSession so `interactivePageId` tracks the session's page.
function syncInteractiveToFocus(): void {
  // A file-target session has no page, so this clears interactivity.
  const nextPageId = focusedPageId()
  if (interactivePageId() === nextPageId) return
  setInteractivePageId(nextPageId)
  sendInteractiveState()
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

// How far to zoom out when exiting focus. Easy in-focus navigation makes the
// pre-focus camera position usually irrelevant, so we keep where the user is
// and just pull back a touch instead of restoring the stored camera.
const FOCUS_EXIT_ZOOM_OUT = 0.85

export function restoreFocusCamera(): boolean {
  // The graceful, camera-restoring exit (X button / Escape / dimmed-click).
  if (!focusSession()) return false
  exitFocusSession('dismiss')
  // Keep the current camera position; zoom out slightly, anchored on the
  // viewport center so the focused content stays put as we pull back.
  const viewport = availableCanvasViewportRect()
  const nextZoom = clampCanvasZoom(zoom * FOCUS_EXIT_ZOOM_OUT)
  const anchorX = viewport.x + viewport.width / 2 - canvasOriginX()
  const anchorY = viewport.height / 2
  const ratio = nextZoom / zoom
  animateCameraTo(
    {
      zoom: nextZoom,
      pan: {
        x: anchorX - (anchorX - pan.x) * ratio,
        y: anchorY - (anchorY - pan.y) * ratio,
      },
    },
    {
      durationMs: DEFAULT_CAMERA_TRANSITION_DURATION_MS,
      preserveFocusSession: true,
    },
  )
  return true
}

export function hasFocusReturnCamera(): boolean {
  return isFocusSessionActive()
}

function computeFocusZoomForPresentation(
  bounds: WorkspaceBounds,
  viewport: { width: number; height: number },
  mode: FocusPresentationMode | null,
): number {
  if (mode === 'fill') return clampCanvasZoom(1)
  if (mode === 'device') {
    if (bounds.width <= 0 || bounds.height <= 0) return Math.min(zoom, 1)
    const availableWidth = Math.max(1, viewport.width - FOCUS_VIEWPORT_PADDING_PX * 2)
    const availableHeight = Math.max(1, viewport.height - FOCUS_VIEWPORT_PADDING_PX * 2)
    return clampCanvasZoom(Math.min(availableWidth / bounds.width, availableHeight / bounds.height))
  }
  // 'fit' (and multi-select null): reflowed page already sized to the padded
  // area, so this resolves to ~100%.
  return computeFocusZoomForBoundsValue(bounds, viewport, zoom)
}

export function focusCanvasBounds(
  bounds: WorkspaceBounds,
  options?: { animate?: boolean },
): void {
  if (!win) return
  const viewport = availableCanvasViewportRect()
  if (
    isBoundsFullyVisibleInCamera({
      bounds,
      viewport,
      canvasOrigin: canvasOrigin(),
      zoom,
      pan,
    })
  ) {
    // A repeated reveal should also stop an older camera intent from carrying
    // the now-visible target back out of frame.
    cancelCameraAnimation()
    return
  }
  moveCameraTo(
    { zoom, pan: panToCenterBoundsAtZoom(bounds, zoom) },
    {
      animate: options?.animate ?? true,
      durationMs: FOCUS_CAMERA_TRANSITION_DURATION_MS,
    },
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
  suppressCameraAutosave = true
  try {
    if (preserveFocusSession) {
      setCameraForFocus(camera.zoom, camera.pan)
      return
    }
    setZoom(camera.zoom)
    setPan(camera.pan.x, camera.pan.y)
  } finally {
    suppressCameraAnimationCancel = false
    suppressCameraAutosave = false
  }
}

/** Stop any in-flight camera animation. Safe to call when none is running. */
export function cancelCameraAnimation(): void {
  if (cameraAnimationTimer) {
    clearInterval(cameraAnimationTimer)
    cameraAnimationTimer = null
  }
  setCameraTransitionStartedAt(null)
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
    scheduleSpaceAutosave()
    requestLayout()
    return
  }

  const start = Date.now()
  setCameraTransitionStartedAt(start)
  cameraAnimationTimer = setInterval(() => {
    const t = Math.min(1, (Date.now() - start) / duration)
    applyCamera(interpolateCamera(startCamera, target, t), preserveFocusSession)
    requestLayout()
    if (t >= 1) {
      cancelCameraAnimation()
      scheduleSpaceAutosave()
    }
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
    const focus = focusSession()
    return focus?.target.kind === 'page' && focus.target.id === page.id
      ? focusPageBounds(page.id, focus.mode)
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
  return pageUsesCustomSize(page.metadata) ? 'fit' : 'device'
}

function fitFocusPageSize(): { width: number; height: number } {
  const viewport = availableCanvasViewportRect()
  return {
    width: Math.max(320, Math.round(viewport.width - FOCUS_VIEWPORT_PADDING_PX * 2)),
    height: Math.max(200, Math.round(viewport.height - FOCUS_VIEWPORT_PADDING_PX * 2)),
  }
}

function fillFocusPageSize(): { width: number; height: number } {
  const viewport = availableCanvasViewportRect()
  return { width: Math.round(viewport.width), height: Math.round(viewport.height) }
}

function focusPageContentSize(
  page: Parameters<typeof pageContentSize>[0] & { id?: string },
  mode: FocusPresentationMode,
): { width: number; height: number } {
  if (mode === 'fit') return fitFocusPageSize()
  if (mode === 'fill') return fillFocusPageSize()
  return pageContentSize(page)
}

function focusPageBounds(pageId: string, mode: FocusPresentationMode): WorkspaceBounds | null {
  const page = pages.find((candidate) => candidate.id === pageId)
  if (!page) return null
  const size = focusPageContentSize(page, mode)
  return pageVisualBoundsForContentSize(page, size)
}

export function setFocusPresentationMode(mode: FocusPresentationMode): boolean {
  const current = focusSession()
  if (!current) return false
  // Mode switching is a page affordance; a note session is created directly
  // in 'fill' and has nothing to switch between.
  if (current.target.kind !== 'page') return false
  const bounds = focusPageBounds(current.target.id, mode)
  if (!bounds) return false
  setFocusSessionMode(mode)
  recenterFocusPresentation()
  return true
}

export function setFocusAnnotationsVisible(visible: boolean): boolean {
  const current = focusSession()
  if (!current) return false
  if (current.annotationsVisible === visible) return true
  setSessionAnnotationsVisible(visible)
  // Mark canvas dirty so the next layout pass actually broadcasts layout-update
  // (the broadcast is gated on the 'canvas' dirty flag) — otherwise the eye
  // state only reaches the renderer on the next unrelated dirtying event.
  markDirty('canvas')
  requestLayout()
  return true
}

/**
 * Retarget an *active* focus session to a different page, keeping the session's
 * current mode and return camera. Returns false (no-op) when no session is
 * active, so callers can fall back to a plain reveal. This is what makes focus
 * persist across page switches: the session outlives any single target page.
 */
export function refocusActiveSession(pageId: string, options?: { animate?: boolean }): boolean {
  const current = focusSession()
  if (!current) return false
  if (!pages.some((page) => page.id === pageId)) return false
  commitEditIfSessionFile(current)
  repointFocusSession(pageId)
  return recenterFocusPresentation(pageId, options)
}

export function recenterFocusPresentation(
  pageId?: string,
  options?: { animate?: boolean },
): boolean {
  const current = focusSession()
  if (!current) return false
  if (pageId && (current.target.kind !== 'page' || current.target.id !== pageId)) return false
  const isFile = current.target.kind === 'file'
  const bounds = isFile
    ? resolveEntityBounds(current.target.id)
    : focusPageBounds(current.target.id, current.mode)
  if (!bounds) return false
  const viewport = availableCanvasViewportRect()
  // A note has no authored viewport to fill or emulate — frame it like a plain
  // fit-and-center reveal.
  const nextZoom = computeFocusZoomForPresentation(bounds, viewport, isFile ? null : current.mode)
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

  const shouldCreateFocusSession = options?.storeReturnCamera !== false
  const singlePageTarget =
    targets.length === 1 && targets[0]?.kind === 'page'
      ? pages.find((page) => page.id === targets[0]!.id) ?? null
      : null
  // A single note (or any renderer claiming fillFocus) gets the same session
  // machinery as a page, always in 'fill'.
  const singleFillFocusFileId =
    !singlePageTarget &&
    shouldCreateFocusSession &&
    targets.length === 1 &&
    targets[0]?.kind === 'file' &&
    fileEntitySupportsFillFocus(targets[0].id)
      ? targets[0].id
      : null
  const focusMode = singlePageTarget && shouldCreateFocusSession
    ? defaultFocusPresentationMode(singlePageTarget)
    : null
  if (singlePageTarget && focusMode) {
    beginFocusSession({
      target: { kind: 'page', id: singlePageTarget.id },
      mode: focusMode,
      annotationsVisible: false,
    })
  } else if (singleFillFocusFileId) {
    beginFocusSession({
      target: { kind: 'file', id: singleFillFocusFileId },
      mode: 'fill',
      annotationsVisible: false,
    })
  } else {
    exitFocusSession('re-focus')
  }
  syncInteractiveToFocus()

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
  moveCameraTo(
    { zoom: nextZoom, pan: panToCenterBoundsAtZoom(combined, nextZoom) },
    {
      animate: options?.animate ?? true,
      preserveFocusSession: shouldCreateFocusSession,
    },
  )
  // Focusing a note goes straight into its markdown editor — the fullscreen
  // read and the edit are the same state for a note.
  if (singleFillFocusFileId) beginEditingEntity(singleFillFocusFileId)
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
