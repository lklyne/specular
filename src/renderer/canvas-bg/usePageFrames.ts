import { useEffect, useRef } from 'react'
import type { PageFrameMessage, PageFrameMeta } from '../../shared/page-frames'

export interface PageFrame {
  bitmap: ImageBitmap
  meta: PageFrameMeta
  receivedAt: number
}

export interface PageFrameStore {
  frames: Map<string, PageFrame>
  popups: Map<string, PageFrame>
}

/**
 * Electron never signals a popup widget (a `<select>` dropdown, an
 * autofill menu) closing over the shared-texture channel — the only
 * available signal is that its owning page stopped repainting it. A popup
 * this stale is treated as closed.
 */
export const POPUP_STALE_MS = 1500

/**
 * Holds the latest frame per page (and per page popup), replacing and
 * closing bitmaps as they arrive. `onFrame` fires after each arrival so the
 * draw loop can mark itself dirty.
 *
 * A page repainting does not imply its popup closed — a `<select>` stays
 * open while the page underneath it keeps animating — so a frame arrival
 * never clears `popups`. A popup arriving at zero size is Electron's own
 * signal that it closed, and is dropped immediately; a popup that simply
 * stops arriving is caught by the draw loop's `POPUP_STALE_MS` check instead.
 */
export function usePageFrames(onFrame: () => void): PageFrameStore {
  const storeRef = useRef<PageFrameStore>({ frames: new Map(), popups: new Map() })
  const onFrameRef = useRef(onFrame)
  onFrameRef.current = onFrame

  useEffect(() => {
    const store = storeRef.current
    const handleMessage = (event: MessageEvent) => {
      const data = event.data as (PageFrameMessage & { bitmap: ImageBitmap }) | null
      if (!data || typeof data !== 'object' || data.source !== 'page-frame' || data.kind !== 'frame') {
        return
      }
      const { meta, bitmap } = data
      if (meta.widgetType === 'popup') {
        store.popups.get(meta.pageId)?.bitmap.close()
        if (meta.width === 0 || meta.height === 0) {
          store.popups.delete(meta.pageId)
          bitmap.close()
        } else {
          store.popups.set(meta.pageId, { bitmap, meta, receivedAt: performance.now() })
        }
      } else {
        store.frames.get(meta.pageId)?.bitmap.close()
        store.frames.set(meta.pageId, { bitmap, meta, receivedAt: performance.now() })
      }
      onFrameRef.current()
    }
    window.addEventListener('message', handleMessage)
    return () => window.removeEventListener('message', handleMessage)
  }, [])

  return storeRef.current
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
