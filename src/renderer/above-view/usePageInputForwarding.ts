import type { LayoutSnapshotRef } from '../shared/hooks/useProjectedLayoutRef'
import type {
  ProjectedLayoutData,
  ProjectedPageEntity,
  ProjectedSceneEntity,
} from '../../shared/scene-projection'
import { useCallback, useEffect, useRef } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { clientYToWindowY, isOverlayUiTarget } from '../../shared/gesture-utils'
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
  layoutRef: LayoutSnapshotRef
  pendingPlacement: ProjectedLayoutData['pendingPlacement']
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
      const windowY = clientYToWindowY(clientY, layout)
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
      // A held button means a drag is in flight (marquee, item drag). Hover
      // would paint a per-item highlight that fights the marquee's own
      // "will be selected" preview, so suppress it for the duration.
      if (event.buttons !== 0) {
        clearHover()
        return
      }
      if (pendingPlacement) {
        setPlacementCursor({
          clientX: event.clientX,
          clientY: clientYToWindowY(event.clientY, layoutRef.current),
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

  // Mirror the focused page's `cursor-changed` onto aboveView's body so the OS
  // shows the right cursor (hand on links, I-beam on text, etc.). The OS picks
  // the cursor from aboveView, which sits above every page.
  useEffect(() => {
    return api.onPageCursorChange(({ type }) => {
      document.body.style.cursor = electronCursorToCss(type)
    })
  }, [api])

  // Continuous hover forwarding into a page's body so cursor styling
  // (link → hand, text → I-beam) and hover-driven UI react without requiring a
  // button-down. The router's `runForwardPointer` already forwards moves while
  // a button is held, so this listener only fires when no buttons are pressed
  // to avoid double-dispatch. When the pointer leaves the page's body (or
  // selection drops below one page), reset body cursor so the hand/I-beam
  // doesn't bleed into canvas chrome.
  //
  // Which page receives the move depends on the tool: normally the single
  // selected one, but the inspect eyedropper reads the DOM of whatever page is
  // under the pointer, so it follows the pointer across pages.
  useEffect(() => {
    let cursorIsForwarded = false
    let lastForwardedPageId: string | null = null
    const resetCursor = () => {
      if (!cursorIsForwarded) return
      cursorIsForwarded = false
      document.body.style.cursor = ''
    }
    const forwardMove = (pageId: string, event: PointerEvent, windowY: number) => {
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
    const onMove = (event: PointerEvent) => {
      if (event.buttons !== 0) return
      const layout = layoutRef.current
      const windowY = clientYToWindowY(event.clientY, layout)
      const point = { x: event.clientX, y: windowY }
      const inspecting = layout.activeTool.kind === 'inspect'
      const page = inspecting
        ? topPageUnderPoint(layout.entities, point)
        : singleSelectedPageUnder(layout, point)

      // Leaving a page: one more move at the pointer's new position, which
      // lands outside that page's own viewport, so its hit-test comes back
      // empty and the inspect highlight clears. Nothing native reaches an
      // offscreen page, so it never gets a mouseleave of its own.
      if (lastForwardedPageId && lastForwardedPageId !== page?.id) {
        forwardMove(lastForwardedPageId, event, windowY)
        lastForwardedPageId = null
      }
      if (!page) return resetCursor()
      cursorIsForwarded = true
      lastForwardedPageId = page.id
      forwardMove(page.id, event, windowY)
    }
    window.addEventListener('pointermove', onMove)
    return () => {
      window.removeEventListener('pointermove', onMove)
      resetCursor()
    }
  }, [api, layoutRef])
}

/** The single-selected page, when the pointer is over its content. */
function singleSelectedPageUnder(
  layout: ProjectedLayoutData,
  point: { x: number; y: number },
): ProjectedPageEntity | null {
  const selected = layout.selectedEntityIds
  if (selected.length !== 1) return null
  const page = layout.entities.find(
    (entity): entity is ProjectedPageEntity =>
      entity.kind === 'page' && entity.id === selected[0],
  )
  if (!page) return null
  return pointerOverPageContent(page, point) ? page : null
}

/** Topmost page whose content the point lands in — entity order is back-to-front. */
function topPageUnderPoint(
  entities: readonly ProjectedSceneEntity[],
  point: { x: number; y: number },
): ProjectedPageEntity | null {
  for (let i = entities.length - 1; i >= 0; i--) {
    const entity = entities[i]
    if (entity.kind !== 'page') continue
    if (pointerOverPageContent(entity, point)) return entity
  }
  return null
}
