import { useEffect, useRef } from 'react'
import type { OsrLabFrameMessage, OsrLabFrameMeta } from '../../shared/osr-lab'

export interface LabFrame {
  bitmap: ImageBitmap
  meta: OsrLabFrameMeta
  copyMs: number
  receivedAt: number
}

export interface LabFrameStore {
  frames: Map<string, LabFrame>
  popups: Map<string, LabFrame>
  /** Frames received in the last second, updated once per second. */
  framesPerSecond: number
  /** Total frames received since the store was created. */
  totalFrames: number
  /** Mean texture → ImageBitmap copy time over the last second, ms. */
  copyMsMean: number | null
}

/**
 * Holds the latest frame per page (and per page popup), replacing bitmaps as
 * they arrive and closing the ones they replace. `onFrame` fires after each
 * arrival so the draw loop can mark itself dirty.
 */
export function useLabFrames(onFrame: () => void): LabFrameStore {
  const storeRef = useRef<LabFrameStore>({
    frames: new Map(),
    popups: new Map(),
    framesPerSecond: 0,
    totalFrames: 0,
    copyMsMean: null,
  })
  const onFrameRef = useRef(onFrame)
  onFrameRef.current = onFrame

  useEffect(() => {
    const store = storeRef.current
    let windowCount = 0
    let windowCopyMs = 0
    const handleMessage = (event: MessageEvent) => {
      const data = event.data as (OsrLabFrameMessage & { bitmap: ImageBitmap }) | null
      if (!data || typeof data !== 'object' || data.source !== 'osr-lab' || data.kind !== 'frame') return
      const bucket = data.meta.widgetType === 'popup' ? store.popups : store.frames
      bucket.get(data.meta.pageId)?.bitmap.close()
      bucket.set(data.meta.pageId, {
        bitmap: data.bitmap,
        meta: data.meta,
        copyMs: data.copyMs,
        receivedAt: data.receivedAt,
      })
      store.totalFrames++
      windowCount++
      windowCopyMs += data.copyMs
      onFrameRef.current()
    }
    const tick = setInterval(() => {
      store.framesPerSecond = windowCount
      store.copyMsMean = windowCount > 0 ? windowCopyMs / windowCount : null
      windowCount = 0
      windowCopyMs = 0
    }, 1000)
    window.addEventListener('message', handleMessage)
    return () => {
      window.removeEventListener('message', handleMessage)
      clearInterval(tick)
    }
  }, [])

  return storeRef.current
}

export function clearLabFrames(store: LabFrameStore): void {
  for (const frame of store.frames.values()) frame.bitmap.close()
  for (const frame of store.popups.values()) frame.bitmap.close()
  store.frames.clear()
  store.popups.clear()
  store.totalFrames = 0
}
