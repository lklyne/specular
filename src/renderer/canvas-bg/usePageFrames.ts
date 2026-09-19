import { useEffect, useRef } from 'react'
import type { PageFrameMessage, PageFrameMeta, PagePopupAnchor } from '../../shared/page-frames'

export interface PageFrame {
  bitmap: ImageBitmap
  meta: PageFrameMeta
}

export interface PagePopup extends PageFrame {
  /** Where the popup hangs in page CSS px; null until the page answers. */
  anchor: PagePopupAnchor | null
  /**
   * Set when the page painted after this popup's last paint *and* had been
   * sent input since. Electron sends no close event for a popup widget; what
   * it does send, on close, is one page paint with no popup paint after it.
   * A page that animates paints for its own reasons, so the input the frame
   * carries is what separates a dismissal from a spinner. A popup paint
   * clears this.
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

/**
 * Holds the latest frame per page (and per page popup), replacing and
 * closing bitmaps as they arrive. `onFrame` fires after each arrival so the
 * draw loop can mark itself dirty; `requestAnchor` is asked once per popup
 * session for the focused element's rect, which is the popup's position.
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
    const handleMessage = (event: MessageEvent) => {
      const data = event.data as PageFrameMessage | null
      if (data?.source !== 'page-frame') return
      const { meta, bitmap } = data
      if (meta.widgetType === 'popup') {
        const previous = store.popups.get(meta.pageId)
        previous?.bitmap.close()
        if (meta.width === 0 || meta.height === 0) {
          store.popups.delete(meta.pageId)
          bitmap.close()
        } else if (previous) {
          previous.bitmap = bitmap
          previous.meta = meta
          previous.closingSince = null
        } else {
          const popup: PagePopup = { bitmap, meta, anchor: null, closingSince: null }
          store.popups.set(meta.pageId, popup)
          resolveAnchor(meta.pageId, popup)
        }
      } else {
        store.frames.get(meta.pageId)?.bitmap.close()
        store.frames.set(meta.pageId, { bitmap, meta })
        const popup = store.popups.get(meta.pageId)
        if (popup && pageFrameDismissesPopup(popup, meta)) popup.closingSince = performance.now()
      }
      onFrameRef.current()
    }
    window.addEventListener('message', handleMessage)
    return () => window.removeEventListener('message', handleMessage)
  }, [])

  return storeRef.current
}

/**
 * Whether a page frame that just arrived is evidence its popup was dismissed:
 * the page painted after the popup's last paint, and the user did something in
 * between that could have closed it. Without the second half, a page that
 * animates would close its own picker a frame after it opened.
 */
export function pageFrameDismissesPopup(popup: PagePopup, frame: PageFrameMeta): boolean {
  return popup.closingSince === null && frame.inputSeq > popup.meta.inputSeq
}

/** Drops a popup whose page painted after it and heard nothing from it since. */
export function popupHasClosed(popup: PagePopup, now: number): boolean {
  return popup.closingSince !== null && now - popup.closingSince > POPUP_CLOSE_GRACE_MS
}

/** Closes and drops every frame/popup for a page no longer in the scene. */
export function prunePageFrames(store: PageFrameStore, liveIds: ReadonlySet<string>): void {
  for (const [id, frame] of store.frames) {
    if (liveIds.has(id)) continue
    frame.bitmap.close()
    store.frames.delete(id)
  }
  for (const [id, frame] of store.popups) {
    if (liveIds.has(id)) continue
    frame.bitmap.close()
    store.popups.delete(id)
  }
}
