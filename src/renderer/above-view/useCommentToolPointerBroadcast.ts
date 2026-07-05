/**
 * Comment-tool page-paints contract (ADR 0006). While the comment tool is
 * active, broadcast the pointer's window-coord position to main so each page
 * can paint a hover preview directly in its own DOM. The marquee rect is
 * piped in by the in-flight drag handler via `setRegionRect`, and held
 * across the post-drag composer via `holdRegionRect` so each page keeps
 * outlining contained items while the user types.
 *
 * Lifecycle:
 *   - tool active → emit on every pointermove + region-rect update
 *   - tool deactivated, pointer leaves window, or component unmounts →
 *     emit a single `null` clear so pages drop their overlays
 */

import { useCallback, useEffect, useRef } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { clientYToWindowY } from '../../shared/coords'

type PointerStateApi = Pick<CanvasBgElectronAPI, 'setCommentToolPointerState'>

export type CommentToolPointerBroadcast = {
  setRegionRect: (
    rect: { x: number; y: number; width: number; height: number } | null,
  ) => void
  clear: () => void
}

export function useCommentToolPointerBroadcast({
  api,
  layoutRef,
  active,
  heldRegionRect,
}: {
  api: PointerStateApi
  layoutRef: React.MutableRefObject<LayoutUpdateData>
  active: boolean
  /**
   * Region rect to keep broadcasting while no in-flight drag rect is set —
   * used for the post-drag composer so each page keeps outlining contained
   * items. In window coords. Pass `null` when there is no held rect.
   */
  heldRegionRect: { x: number; y: number; width: number; height: number } | null
}): CommentToolPointerBroadcast {
  const lastPointerRef = useRef<{ windowX: number; windowY: number } | null>(null)
  const dragRegionRectRef = useRef<{ x: number; y: number; width: number; height: number } | null>(null)
  const heldRegionRectRef = useRef<typeof heldRegionRect>(null)
  const lastSentRef = useRef<string | null>(null)
  const activeRef = useRef(active)

  const send = useCallback(
    (
      pointer: { windowX: number; windowY: number } | null,
      region: { x: number; y: number; width: number; height: number } | null,
    ) => {
      if (!pointer && !region) {
        if (lastSentRef.current === null) return
        lastSentRef.current = null
        api.setCommentToolPointerState(null)
        return
      }
      // Page paints need at least one of (pointer, region). When only the
      // region is available (post-drag composer with no recent pointermove),
      // synthesise a pointer outside any page so main still forwards the
      // region snapshot to every page.
      const effectivePointer =
        pointer ?? { windowX: -1, windowY: -1 }
      const payload = {
        windowX: effectivePointer.windowX,
        windowY: effectivePointer.windowY,
        regionRect: region,
      }
      const key = `${payload.windowX}:${payload.windowY}:${
        region ? `${region.x},${region.y},${region.width},${region.height}` : 'null'
      }`
      if (key === lastSentRef.current) return
      lastSentRef.current = key
      api.setCommentToolPointerState(payload)
    },
    [api],
  )

  const currentRegion = useCallback(
    () => dragRegionRectRef.current ?? heldRegionRectRef.current,
    [],
  )

  useEffect(() => {
    heldRegionRectRef.current = heldRegionRect
    if (!activeRef.current) return
    send(lastPointerRef.current, currentRegion())
  }, [heldRegionRect, send, currentRegion])

  useEffect(() => {
    activeRef.current = active
    if (!active) {
      lastPointerRef.current = null
      dragRegionRectRef.current = null
      send(null, null)
      return
    }
    const onMove = (event: PointerEvent) => {
      const layout = layoutRef.current
      const pointer = {
        windowX: event.clientX,
        windowY: clientYToWindowY(event.clientY, layout),
      }
      lastPointerRef.current = pointer
      send(pointer, currentRegion())
    }
    const onLeave = () => {
      lastPointerRef.current = null
      send(null, currentRegion())
    }
    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerleave', onLeave)
    window.addEventListener('blur', onLeave)
    // Initial flush — covers the case where `active` flips on with a held
    // region rect already set (e.g. transition from in-flight drag to the
    // post-drag composer happens within the same tick).
    send(lastPointerRef.current, currentRegion())
    return () => {
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerleave', onLeave)
      window.removeEventListener('blur', onLeave)
      lastPointerRef.current = null
      dragRegionRectRef.current = null
      send(null, null)
    }
  }, [active, layoutRef, send, currentRegion])

  const setRegionRect = useCallback(
    (rect: { x: number; y: number; width: number; height: number } | null) => {
      dragRegionRectRef.current = rect
      if (!activeRef.current) return
      send(lastPointerRef.current, currentRegion())
    },
    [send, currentRegion],
  )

  const clear = useCallback(() => {
    lastPointerRef.current = null
    dragRegionRectRef.current = null
    send(null, null)
  }, [send])

  // Stable returned object — callers can list it in useEffect deps without
  // tearing down on every render.
  const apiRef = useRef<CommentToolPointerBroadcast>({ setRegionRect, clear })
  apiRef.current.setRegionRect = setRegionRect
  apiRef.current.clear = clear
  return apiRef.current
}
