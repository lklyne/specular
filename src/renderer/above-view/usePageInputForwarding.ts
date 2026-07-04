import { useCallback, useEffect, useRef } from 'react'
import type {
  CanvasScenePageEntity,
  LayoutUpdateData,
} from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { isOverlayUiTarget } from '../../shared/gesture-utils'
import { pointerOverPageContent } from '../../shared/page-hit-test'

/** Map Electron's `cursor-changed` type strings onto CSS cursor values.
 *  Electron uses Blink-era names where `pointer` is the arrow and `hand` is
 *  the link hand — the opposite of CSS. Most other types match CSS 1:1;
 *  panning variants and unknown/custom types collapse to a sensible default. */
function electronCursorToCss(type: string | null): string {
  if (!type || type === 'custom' || type === 'null') return ''
  if (type === 'pointer') return 'default'
  if (type === 'hand') return 'pointer'
  if (type === 'iBeam') return 'text'
  if (type.endsWith('-panning')) return 'all-scroll'
  return type
}

export interface UsePageInputForwardingOptions {
  api: CanvasBgElectronAPI
  layoutRef: React.MutableRefObject<LayoutUpdateData>
  pendingPlacement: LayoutUpdateData['pendingPlacement']
  hoverForwardingEnabled: boolean
  setPlacementCursor: (cursor: { clientX: number; clientY: number } | null) => void
}

/**
 * Owns aboveView's no-button page input forwarding: the placement-cursor +
 * hover-forward pointermove listener, the focused-page cursor-style mirror, and
 * the no-button pointer-forward into the single-selected page. The router's
 * `runForwardPointer` handles moves while a button is held, so the no-button
 * listener only fires with `buttons === 0` to avoid double-dispatch.
 */
export function usePageInputForwarding({
  api,
  layoutRef,
  pendingPlacement,
  hoverForwardingEnabled,
  setPlacementCursor,
}: UsePageInputForwardingOptions): void {
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

  // One window pointermove handler drives both placement-preview cursor and
  // hover forwarding. When above-view intercepts events (gate open), canvas-bg
  // never sees mouseenter/leave, so we dedupe and forward via api.hoverPage.
  const lastHoverIdRef = useRef<string | null>(null)
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
  }, [api, hitTestHoverTarget, hoverForwardingEnabled, layoutRef, pendingPlacement, setPlacementCursor])

  // PoC: mirror the focused page's `cursor-changed` onto aboveView's body so
  // the OS shows the right cursor (hand on links, I-beam on text, etc.). The
  // OS picks cursor from the topmost WCV at the pointer location, which is
  // aboveView whenever the canvas-mode gate is open.
  useEffect(() => {
    return api.onPageCursorChange(({ type }) => {
      document.body.style.cursor = electronCursorToCss(type)
    })
  }, [api])

  // PoC: continuous hover forwarding into the single-selected page's body so
  // cursor styling (link → hand, text → I-beam) and hover-driven UI react
  // without requiring a button-down. The router's `runForwardPointer` already
  // forwards moves while a button is held, so this listener only fires when
  // no buttons are pressed to avoid double-dispatch. When the pointer leaves
  // the focused page's body (or selection drops below one page), reset
  // body cursor so the hand/I-beam doesn't bleed into canvas chrome.
  useEffect(() => {
    let cursorIsForwarded = false
    const resetCursor = () => {
      if (!cursorIsForwarded) return
      cursorIsForwarded = false
      document.body.style.cursor = ''
    }
    const onMove = (event: PointerEvent) => {
      if (event.buttons !== 0) return
      const layout = layoutRef.current
      const selected = layout.selectedEntityIds
      if (selected.length !== 1) return resetCursor()
      const pageId = selected[0]
      const page = layout.entities.find(
        (entity): entity is CanvasScenePageEntity =>
          entity.kind === 'page' && entity.id === pageId,
      )
      if (!page) return resetCursor()
      const windowY = event.clientY + layout.canvasOrigin.y
      if (!pointerOverPageContent(page, { x: event.clientX, y: windowY })) return resetCursor()
      cursorIsForwarded = true
      api.forwardPointerToPage(pageId, {
        kind: 'move',
        windowX: event.clientX,
        windowY,
        button: 'left',
        shiftKey: event.shiftKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        metaKey: event.metaKey,
      })
    }
    window.addEventListener('pointermove', onMove)
    return () => {
      window.removeEventListener('pointermove', onMove)
      resetCursor()
    }
  }, [api, layoutRef])
}
