import type { WorkspaceBounds } from './types'

export const FOCUS_VIEWPORT_PADDING_PX = 64

export function computeFocusZoomForBounds(
  bounds: WorkspaceBounds,
  viewport: { width: number; height: number },
  currentZoom = 1,
): number {
  if (bounds.width <= 0 || bounds.height <= 0) return Math.min(currentZoom, 1)
  const availableWidth = Math.max(1, viewport.width - FOCUS_VIEWPORT_PADDING_PX * 2)
  const availableHeight = Math.max(1, viewport.height - FOCUS_VIEWPORT_PADDING_PX * 2)
  const fitZoom = Math.min(availableWidth / bounds.width, availableHeight / bounds.height)
  return Math.min(1, fitZoom)
}

export function computePanToCenterBoundsAtZoom(input: {
  bounds: WorkspaceBounds
  viewport: { x: number; y?: number; width: number; height: number }
  canvasOriginX: number
  zoom: number
}): { x: number; y: number } {
  const { bounds, viewport, canvasOriginX, zoom } = input
  return {
    x: Math.round(
      viewport.x + viewport.width / 2 - canvasOriginX - (bounds.x + bounds.width / 2) * zoom,
    ),
    y: Math.round(viewport.height / 2 - (bounds.y + bounds.height / 2) * zoom),
  }
}

export function isBoundsFullyVisibleInCamera(input: {
  bounds: WorkspaceBounds
  viewport: { x: number; y: number; width: number; height: number }
  canvasOrigin: { x: number; y: number }
  zoom: number
  pan: { x: number; y: number }
}): boolean {
  const { bounds, viewport, canvasOrigin, zoom, pan } = input
  if (!Number.isFinite(zoom) || zoom <= 0) return false

  const left = canvasOrigin.x + bounds.x * zoom + pan.x
  const top = canvasOrigin.y + bounds.y * zoom + pan.y
  const right = left + bounds.width * zoom
  const bottom = top + bounds.height * zoom

  return (
    left >= viewport.x &&
    top >= viewport.y &&
    right <= viewport.x + viewport.width &&
    bottom <= viewport.y + viewport.height
  )
}
