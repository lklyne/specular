import { pan, zoom } from './runtime-context'
import {
  cancelCameraAnimation,
  requestLayout,
  setViewportCamera,
} from './viewport-control'
import { boundCanvasOrigin as canvasOrigin } from './runtime-geometry'
import { win } from './view-refs'
import { clampCanvasZoom } from '../../shared/zoom'

const VIEWPORT_EVENT_FRAME_MS = 16

export interface ViewportInputDelta {
  zoomDeltaY?: number
  panDeltaX?: number
  panDeltaY?: number
  mouseX?: number | null
  mouseY?: number | null
}

let pendingViewportDelta = {
  zoomDeltaY: 0,
  panDeltaX: 0,
  panDeltaY: 0,
  mouseX: null as number | null,
  mouseY: null as number | null,
}
let pendingViewportTimer: NodeJS.Timeout | null = null

export function applyViewportInputDelta({
  zoomDeltaY = 0,
  panDeltaX = 0,
  panDeltaY = 0,
  mouseX = null,
  mouseY = null,
}: ViewportInputDelta): void {
  let nextZoom = zoom
  let nextPan = { x: pan.x, y: pan.y }

  if (zoomDeltaY !== 0) {
    const oldZoom = zoom
    nextZoom = clampCanvasZoom(zoom - zoomDeltaY * 0.002)

    if (win && mouseX !== null && mouseY !== null) {
      const contentBounds = win.getContentBounds()
      const mouseClientX = mouseX - contentBounds.x
      const mouseClientY = mouseY - contentBounds.y
      const origin = canvasOrigin()
      const viewportMouseX = mouseClientX - origin.x
      const viewportMouseY = mouseClientY - origin.y
      const canvasX = (viewportMouseX - pan.x) / oldZoom
      const canvasY = (viewportMouseY - pan.y) / oldZoom

      nextPan = {
        x: viewportMouseX - canvasX * nextZoom,
        y: viewportMouseY - canvasY * nextZoom,
      }
    }
  }

  if (panDeltaX !== 0 || panDeltaY !== 0) {
    nextPan = {
      x: nextPan.x + panDeltaX,
      y: nextPan.y + panDeltaY,
    }
  }

  if (zoomDeltaY !== 0 || panDeltaX !== 0 || panDeltaY !== 0) {
    setViewportCamera(nextZoom, nextPan)
    cancelCameraAnimation()
    requestLayout()
  }
}

function flushViewportInputDelta(): void {
  pendingViewportTimer = null
  const delta = pendingViewportDelta
  pendingViewportDelta = {
    zoomDeltaY: 0,
    panDeltaX: 0,
    panDeltaY: 0,
    mouseX: null,
    mouseY: null,
  }
  applyViewportInputDelta(delta)
}

export function enqueueViewportInputDelta({
  zoomDeltaY = 0,
  panDeltaX = 0,
  panDeltaY = 0,
  mouseX,
  mouseY,
}: ViewportInputDelta): void {
  pendingViewportDelta.zoomDeltaY += zoomDeltaY
  pendingViewportDelta.panDeltaX += panDeltaX
  pendingViewportDelta.panDeltaY += panDeltaY
  if (mouseX !== undefined) pendingViewportDelta.mouseX = mouseX
  if (mouseY !== undefined) pendingViewportDelta.mouseY = mouseY
  if (pendingViewportTimer) return
  pendingViewportTimer = setTimeout(flushViewportInputDelta, VIEWPORT_EVENT_FRAME_MS)
}
