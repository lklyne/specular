import { useEffect, useRef } from 'react'
import type { PageFrameMeta } from '../../../shared/page-frames'
import type { PageVideoFrameMessage } from '../../../shared/page-surface-spike'

export interface VideoFrameEntry {
  /** Holds one of the page's shared-texture slots until closed (ADR 0038). */
  frame: VideoFrame
  meta: PageFrameMeta
}

export type VideoFrameStore = Map<string, VideoFrameEntry>

type FrameHandler = (pageId: string, frame: VideoFrame, meta: PageFrameMeta) => void

let frameHandler: FrameHandler | null = null
let listening = false

/**
 * One listener for the renderer's lifetime, so a frame that arrives with no
 * mounted handler (spike surface unmounted, factory still resolving, arm
 * switched away) is closed here instead of holding a texture slot until GC.
 * The preload drops frames until this has announced itself.
 */
function listenForVideoFrames(): void {
  if (listening) return
  listening = true
  window.addEventListener('message', (event) => {
    const data = event.data as PageVideoFrameMessage | null
    if (data?.source !== 'page-video-frame' || data.kind !== 'frame') return
    if (frameHandler) frameHandler(data.meta.pageId, data.frame, data.meta)
    else data.frame.close()
  })
  const ready: PageVideoFrameMessage = { source: 'page-video-frame', kind: 'ready' }
  window.postMessage(ready, '*')
}

/**
 * The VideoFrame equivalent of `usePageFrames`: holds the single latest
 * frame per page, closing the previous one as each replacement arrives.
 * `onFrame` fires after the store is updated.
 */
export function useVideoFrameStore(
  onFrame: (pageId: string, frame: VideoFrame, meta: PageFrameMeta) => void,
): VideoFrameStore {
  const storeRef = useRef<VideoFrameStore>(new Map())
  const onFrameRef = useRef(onFrame)
  onFrameRef.current = onFrame

  useEffect(() => {
    const store = storeRef.current
    const handle: FrameHandler = (pageId, frame, meta) => {
      store.get(pageId)?.frame.close()
      store.set(pageId, { frame, meta })
      onFrameRef.current(pageId, frame, meta)
    }
    frameHandler = handle
    listenForVideoFrames()
    return () => {
      if (frameHandler === handle) frameHandler = null
    }
  }, [])

  return storeRef.current
}

/** Closes and drops every frame for a page no longer live, calling `onRemoved` for each. */
export function pruneVideoFrames(
  store: VideoFrameStore,
  liveIds: ReadonlySet<string>,
  onRemoved: (pageId: string) => void,
): void {
  for (const [id, entry] of store) {
    if (liveIds.has(id)) continue
    entry.frame.close()
    store.delete(id)
    onRemoved(id)
  }
}
