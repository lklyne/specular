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
