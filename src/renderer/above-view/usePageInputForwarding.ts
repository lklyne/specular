import { useCallback, useEffect, useRef, useState } from 'react'
import { isOverlayUiTarget } from '../../shared/gesture-utils'
import type { CanvasBgElectronAPI, LayoutUpdateData } from '../../shared/types'

export interface PlacementCursor {
  clientX: number
  clientY: number
}

/**
 * One window-level `pointermove` listener drives both the placement-preview
 * ghost's cursor position and continuous page-hover forwarding. When
 * aboveView intercepts events (gate open), canvas-bg never sees
 * mouseenter/leave, so this dedupes and forwards via `api.hoverPage`.
 *
 * The cursor starts null and is set by the first pointermove rather than
 * seeded from main, because polling the OS cursor at layout time risks
 * capturing toolbar coordinates and re-snapping the ghost on every layout
 * broadcast.
 */
export function usePageInputForwarding({
  api,
  layoutRef,
  pendingPlacement,
  activeToolKind,
}: {
  api: CanvasBgElectronAPI
  layoutRef: React.MutableRefObject<LayoutUpdateData>
  pendingPlacement: LayoutUpdateData['pendingPlacement']
  activeToolKind: LayoutUpdateData['activeTool']['kind']
}): PlacementCursor | null {
  const [placementCursor, setPlacementCursor] = useState<PlacementCursor | null>(null)
  useEffect(() => {
    if (!pendingPlacement) setPlacementCursor(null)
  }, [pendingPlacement])

  const hitTestHoverTarget = useCallback(
    (clientX: number, clientY: number) => {
      const layout = layoutRef.current
      const windowY = clientY + layout.canvasOrigin.y
      for (let i = layout.entities.length - 1; i >= 0; i--) {
        const entity = layout.entities[i]
        if (entity.kind === 'group' || entity.kind === 'drawing') continue
        if (
          clientX >= entity.screenX &&
          clientX <= entity.screenX + entity.screenWidth &&
          windowY >= entity.screenY &&
          windowY <= entity.screenY + entity.screenHeight
        ) {
          return entity.id
        }
      }
      return null
    },
    [layoutRef],
  )

  const lastHoverIdRef = useRef<string | null>(null)
  const hoverForwardingEnabled = activeToolKind !== 'draw' && activeToolKind !== 'comment'
  useEffect(() => {
    const clearHover = () => {
      setPlacementCursor(null)
      if (lastHoverIdRef.current === null) return
      lastHoverIdRef.current = null
      api.hoverPage(null)
    }
    if (!pendingPlacement && !hoverForwardingEnabled) {
      clearHover()
      return
    }
    const handleMove = (event: PointerEvent) => {
      if (isOverlayUiTarget(event.target)) {
        clearHover()
        return
      }
      if (pendingPlacement) {
        setPlacementCursor({
          clientX: event.clientX,
          clientY: event.clientY + layoutRef.current.canvasOrigin.y,
        })
      }
      // During placement the placeholder owns the cursor; page hover would flicker.
      if (hoverForwardingEnabled && !pendingPlacement) {
        const nextId = hitTestHoverTarget(event.clientX, event.clientY)
        if (nextId !== lastHoverIdRef.current) {
          lastHoverIdRef.current = nextId
          api.hoverPage(nextId)
        }
      }
    }
    // The top toolbar is a sibling WebContentsView, so when the cursor moves
    // up into it the above-view stops receiving pointer events without
    // pointerleave firing. mouseleave on documentElement is the reliable
    // "cursor left this webcontents" signal in Electron's multi-view layout.
    const docEl = document.documentElement
    window.addEventListener('pointermove', handleMove)
    // eslint-disable-next-line local/no-mouse-events
    docEl.addEventListener('mouseleave', clearHover)
    window.addEventListener('blur', clearHover)
    return () => {
      window.removeEventListener('pointermove', handleMove)
      // eslint-disable-next-line local/no-mouse-events
      docEl.removeEventListener('mouseleave', clearHover)
      window.removeEventListener('blur', clearHover)
      clearHover()
    }
  }, [api, hitTestHoverTarget, hoverForwardingEnabled, layoutRef, pendingPlacement])

  return placementCursor
}
