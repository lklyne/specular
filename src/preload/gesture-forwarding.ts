import { ipcChannels } from '../shared/ipc-contract'
import { classifyViewportWheel, middleDragDelta } from '../shared/gesture-utils'
import { ipcRenderer } from 'electron'

const PAGE_OVERLAY_ROOT_SELECTORS = [
  '[data-overlay-ui]',
  '#__canvas-comment-preview-layer',
  '#__canvas-blocking-overlay',
  '#__canvas-resize-handle',
  // `elementsFromPoint` returns elements regardless of `pointer-events: none`,
  // so the comment tool's click resolver would otherwise land on these
  // Specular-painted overlays instead of the page element underneath.
  '[id^="__canvas-dom-inspection-"]',
]

export function isPageOverlayTarget(target: Element | null): boolean {
  if (!target) return false
  return PAGE_OVERLAY_ROOT_SELECTORS.some((selector) => target.closest(selector))
}

/**
 * Forward a wheel event from a page content view to the canvas.
 * Device emulation inflates delta values by 1/zoom (emulated CSS pixels are
 * smaller than screen pixels), so we multiply by `emulationScale` (the canvas
 * zoom) to convert back to screen-space deltas.
 */
export function forwardViewportWheel(event: WheelEvent, emulationScale = 1): void {
  const action = classifyViewportWheel(event)
  if (action.kind === 'zoom') {
    ipcRenderer.send(ipcChannels.canvasZoom, {
      deltaY: action.deltaY * emulationScale,
      mouseX: action.mouseX,
      mouseY: action.mouseY,
    })
    return
  }
  ipcRenderer.send(ipcChannels.canvasPan, {
    deltaX: action.deltaX * emulationScale,
    deltaY: action.deltaY * emulationScale,
  })
}

export function forwardMiddleDragPan(
  previous: { screenX: number; screenY: number },
  next: MouseEvent,
): { screenX: number; screenY: number } {
  const delta = middleDragDelta(previous, next)
  ipcRenderer.send(ipcChannels.canvasPan, delta)
  return {
    screenX: next.screenX,
    screenY: next.screenY,
  }
}
