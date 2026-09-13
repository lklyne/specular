import { useEffect, useRef } from 'react'
import type { PageFrameMessage, PageFrameMeta, PagePopupAnchor } from '../../shared/page-frames'

export interface PageFrame {
  /** Holds one of the page's shared-texture slots until closed (ADR 0038). */
  frame: VideoFrame
  meta: PageFrameMeta
  receivedAt: number
}

export interface PagePopup extends PageFrame {
  /** Where the popup hangs in page CSS px; null until the page answers. */
  anchor: PagePopupAnchor | null
  /**
   * Set when the page painted after this popup's last paint. Electron sends
   * no close event for a popup widget; what it does send, on close, is one
   * page paint with no popup paint after it. A popup paint clears this.
   */
  closingSince: number | null
}

export interface PageFrameStore {
  frames: Map<string, PageFrame>
  popups: Map<string, PagePopup>
}

/**
 * How long a page paint may go unanswered by a popup paint before the popup
 * counts as closed. Long enough to cover the page repainting under an open
 * popup that is itself still animating; short enough that a closed picker
 * does not linger over the page.
 */
export const POPUP_CLOSE_GRACE_MS = 150

type FrameHandler = (meta: PageFrameMeta, frame: VideoFrame) => void

let frameHandler: FrameHandler | null = null
let listening = false

/**
 * One listener for the renderer's lifetime, so a frame still queued when its
 * store unmounts is closed here instead of holding a texture slot until GC.
 * The preload drops frames until this has announced itself.
 */
function listenForPageFrames(): void {
  if (listening) return
  listening = true
  window.addEventListener('message', (event) => {
    const data = event.data as PageFrameMessage | null
    if (!data || typeof data !== 'object' || data.source !== 'page-frame' || data.kind !== 'frame') {
      return
    }
    if (frameHandler) frameHandler(data.meta, data.frame)
    else data.frame.close()
  })
  const ready: PageFrameMessage = { source: 'page-frame', kind: 'ready' }
  window.postMessage(ready, '*')
}

/**
 * Holds the latest frame per page (and per page popup), closing the previous
 * one as each arrives. `onFrame` fires after each arrival so the draw loop can
 * mark itself dirty; `requestAnchor` is asked once per popup session for the
 * focused element's rect, which is the popup's position.
 *
 * A popup arriving at zero size is Electron's own signal that it closed and
 * is dropped at once; otherwise closing is inferred from paint order (see
 * `PagePopup.closingSince`), judged in the draw loop.
 */
export function usePageFrames(
  onFrame: () => void,
  requestAnchor: (pageId: string) => Promise<PagePopupAnchor | null>,
): PageFrameStore {
  const storeRef = useRef<PageFrameStore>({ frames: new Map(), popups: new Map() })
  const onFrameRef = useRef(onFrame)
  onFrameRef.current = onFrame
  const requestAnchorRef = useRef(requestAnchor)
  requestAnchorRef.current = requestAnchor

  useEffect(() => {
    const store = storeRef.current
    const resolveAnchor = (pageId: string, popup: PagePopup) => {
      void requestAnchorRef.current(pageId).then((anchor) => {
        // Only the popup session that asked gets the answer.
        if (store.popups.get(pageId) !== popup) return
        popup.anchor = anchor
        onFrameRef.current()
      })
    }
    const handleFrame: FrameHandler = (meta, frame) => {
      const now = performance.now()
      if (meta.widgetType === 'popup') {
        const previous = store.popups.get(meta.pageId)
        previous?.frame.close()
        if (meta.width === 0 || meta.height === 0) {
          store.popups.delete(meta.pageId)
          frame.close()
        } else if (previous) {
          previous.frame = frame
          previous.meta = meta
          previous.receivedAt = now
          previous.closingSince = null
        } else {
          const popup: PagePopup = { frame, meta, receivedAt: now, anchor: null, closingSince: null }
          store.popups.set(meta.pageId, popup)
          resolveAnchor(meta.pageId, popup)
        }
      } else {
        store.frames.get(meta.pageId)?.frame.close()
        store.frames.set(meta.pageId, { frame, meta, receivedAt: now })
        const popup = store.popups.get(meta.pageId)
        if (popup && popup.closingSince === null) popup.closingSince = now
      }
      onFrameRef.current()
    }
    frameHandler = handleFrame
    listenForPageFrames()
    return () => {
      if (frameHandler === handleFrame) frameHandler = null
    }
  }, [])

  return storeRef.current
}

/** Drops a popup whose page painted after it and heard nothing from it since. */
export function popupHasClosed(popup: PagePopup, now: number): boolean {
  return popup.closingSince !== null && now - popup.closingSince > POPUP_CLOSE_GRACE_MS
}

/** Closes and drops every frame/popup for a page no longer in the scene. */
export function prunePageFrames(store: PageFrameStore, liveIds: ReadonlySet<string>): void {
  for (const [id, entry] of store.frames) {
    if (liveIds.has(id)) continue
    entry.frame.close()
    store.frames.delete(id)
  }
  for (const [id, entry] of store.popups) {
    if (liveIds.has(id)) continue
    entry.frame.close()
    store.popups.delete(id)
  }
}
