import { useEffect, useRef } from 'react'
import {
  blocksViewportGesture,
  classifyViewportWheel,
  middleDragDelta,
  shouldStartMouseViewportPan,
} from '../../../shared/gesture-utils'

interface ViewportWheelAndMiddlePanApi {
  canvasZoom: (deltaY: number, mouseX: number, mouseY: number) => void
  canvasPan: (deltaX: number, deltaY: number) => void
}

export function useViewportWheelAndMiddlePan(
  enabled: boolean,
  api: ViewportWheelAndMiddlePanApi,
  /** PoC: optional wheel pre-router. Return true to indicate the event has
   *  been forwarded into a page's page; the hook then skips its canvas
   *  zoom/pan branch. Cmd/Ctrl+wheel zooms the canvas regardless (the
   *  classifier checks before this is consulted). */
  routeWheel?: (event: WheelEvent) => boolean,
  /** Optional predicate. Return true to let the browser scroll the event's
   *  target natively (e.g. a selected note's body) instead of panning the
   *  canvas. Unlike `routeWheel`, the hook does NOT preventDefault, so the
   *  native scroll proceeds. Cmd/Ctrl+wheel still zooms the canvas. */
  yieldWheelToNativeScroll?: (event: WheelEvent) => boolean,
) {
  const routeWheelRef = useRef(routeWheel)
  routeWheelRef.current = routeWheel
  const yieldWheelRef = useRef(yieldWheelToNativeScroll)
  yieldWheelRef.current = yieldWheelToNativeScroll
  useEffect(() => {
    if (!enabled) return

    let middleDrag: { screenX: number; screenY: number } | null = null

    const onWheel = (event: WheelEvent) => {
      if (blocksViewportGesture(event.target)) return
      const action = classifyViewportWheel(event)
      if (action.kind === 'pan' && yieldWheelRef.current?.(event)) return
      if (action.kind === 'pan' && routeWheelRef.current?.(event)) {
        event.preventDefault()
        return
      }
      event.preventDefault()
      if (action.kind === 'zoom') {
        api.canvasZoom(action.deltaY, action.mouseX, action.mouseY)
        return
      }
      api.canvasPan(action.deltaX, action.deltaY)
    }

    let middleDragPointerId: number | null = null

    const onPointerDown = (event: PointerEvent) => {
      if (blocksViewportGesture(event.target)) return
      if (!shouldStartMouseViewportPan(event)) return
      middleDragPointerId = event.pointerId
      middleDrag = { screenX: event.screenX, screenY: event.screenY }
      event.preventDefault()
    }

    const onPointerMove = (event: PointerEvent) => {
      if (event.pointerId !== middleDragPointerId || !middleDrag) return
      if ((event.buttons & 4) === 0) {
        middleDrag = null
        middleDragPointerId = null
        return
      }
      const delta = middleDragDelta(middleDrag, event)
      middleDrag = { screenX: event.screenX, screenY: event.screenY }
      api.canvasPan(delta.deltaX, delta.deltaY)
    }

    const onPointerUp = (event: PointerEvent) => {
      if (event.pointerId === middleDragPointerId) {
        middleDrag = null
        middleDragPointerId = null
      }
    }

    const onPointerCancel = (event: PointerEvent) => {
      if (event.pointerId === middleDragPointerId) {
        middleDrag = null
        middleDragPointerId = null
      }
    }

    const cleanup = () => {
      middleDrag = null
      middleDragPointerId = null
    }

    const onVisibilityChange = () => {
      if (document.visibilityState !== 'visible') cleanup()
    }

    window.addEventListener('wheel', onWheel, { passive: false })
    window.addEventListener('pointerdown', onPointerDown)
    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', onPointerUp)
    window.addEventListener('pointercancel', onPointerCancel)
    window.addEventListener('blur', cleanup)
    document.addEventListener('visibilitychange', onVisibilityChange)
    return () => {
      window.removeEventListener('wheel', onWheel)
      window.removeEventListener('pointerdown', onPointerDown)
      window.removeEventListener('pointermove', onPointerMove)
      window.removeEventListener('pointerup', onPointerUp)
      window.removeEventListener('pointercancel', onPointerCancel)
      window.removeEventListener('blur', cleanup)
      document.removeEventListener('visibilitychange', onVisibilityChange)
      cleanup()
    }
  }, [api, enabled])
}
