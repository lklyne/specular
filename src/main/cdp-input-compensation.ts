import { getZoom } from './runtime/runtime-context'

// Shared CSS→physical coordinate compensation for CDP `Input.dispatchMouseEvent`.
// Coordinates handed to the debugger are in the emulated CSS viewport space, but
// `Input.dispatchMouseEvent` interprets them in the pre-scale physical view
// space. With `enableDeviceEmulation({ scale })`, Chromium maps physical→CSS by
// dividing by scale, so to land on a CSS target at (x, y) we dispatch
// (x * scale, y * scale). Both the agent CDP proxy (app-control-server) and the
// interaction-sync peer dispatcher run through this one implementation.

/** Holds the emulation scale snapshotted for an in-flight click. */
export interface ClickScaleSnapshot {
  value: number | null
}

export function createClickScaleSnapshot(): ClickScaleSnapshot {
  return { value: null }
}

/**
 * Compensate a dispatch point for the current emulation scale. The scale is
 * snapshotted on `mousePressed` and reused for the paired `mouseReleased`, so a
 * mid-click zoom change can't split the press/release pair across two scales.
 */
export function compensateMousePointForDispatch(
  snapshot: ClickScaleSnapshot,
  cdpType: string,
  point: { x: number; y: number },
): { x: number; y: number } {
  if (cdpType === 'mousePressed') snapshot.value = getZoom()
  const emulationScale = snapshot.value ?? getZoom()
  if (cdpType === 'mouseReleased') snapshot.value = null
  if (emulationScale === 1) return { x: point.x, y: point.y }
  return { x: point.x * emulationScale, y: point.y * emulationScale }
}
