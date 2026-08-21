export function snapshotCaptureStillValid({
  captureLeaseAtStart,
  currentCaptureLease,
  signatureAtStart,
  currentSignature,
}: {
  captureLeaseAtStart: number
  currentCaptureLease: number
  signatureAtStart: string
  currentSignature: string
}): boolean {
  return (
    captureLeaseAtStart === currentCaptureLease &&
    signatureAtStart === currentSignature
  )
}

/**
 * Longest edge, in device pixels, a settle capture renders at. Bounds the
 * decoded bitmap (~16MB at this edge for a 16:10 page) while leaving enough
 * pixels to zoom a desktop page back to 1:1 on a Retina display.
 */
export const SNAPSHOT_MAX_EDGE_PX = 2048

/**
 * The emulation scale a settle capture should render at: the scale that puts
 * the page's longest edge at `SNAPSHOT_MAX_EDGE_PX`, never above `maxZoom`,
 * and never below the live zoom (which the on-screen surface already has).
 */
export function snapshotTargetScale({
  zoom,
  cssWidth,
  cssHeight,
  devicePixelRatio,
  maxZoom,
  maxEdgePx = SNAPSHOT_MAX_EDGE_PX,
}: {
  zoom: number
  cssWidth: number
  cssHeight: number
  devicePixelRatio: number
  maxZoom: number
  maxEdgePx?: number
}): number {
  const longEdge = Math.max(cssWidth, cssHeight) * devicePixelRatio
  if (longEdge <= 0) return zoom
  return Math.max(zoom, Math.min(maxZoom, maxEdgePx / longEdge))
}

export interface SnapshotFrameLike {
  contentKey: string
  capturedWidth: number
  capturedHeight: number
}

/**
 * A frame is a picture of one content state; how big the page is on screen
 * is irrelevant to its validity. When both show the same content, the one
 * with more pixels stays, so a zoomed-out settle never downgrades a frame a
 * zoomed-in settle produced.
 */
export function pickBetterFrame<T extends SnapshotFrameLike>(existing: T | undefined, incoming: T): T {
  if (!existing || existing.contentKey !== incoming.contentKey) return incoming
  const existingPixels = existing.capturedWidth * existing.capturedHeight
  const incomingPixels = incoming.capturedWidth * incoming.capturedHeight
  return existingPixels > incomingPixels ? existing : incoming
}

/** Whether `frame` already has the pixels a capture at `targetScale` would produce. */
export function frameMeetsTarget(
  frame: SnapshotFrameLike | undefined,
  {
    contentKey,
    cssWidth,
    devicePixelRatio,
    targetScale,
  }: { contentKey: string; cssWidth: number; devicePixelRatio: number; targetScale: number },
): boolean {
  if (!frame || frame.contentKey !== contentKey) return false
  return frame.capturedWidth >= Math.floor(cssWidth * targetScale * devicePixelRatio)
}
