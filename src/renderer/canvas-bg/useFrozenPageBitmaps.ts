import { useEffect, useState } from 'react'
import type { FrozenPagesState } from '../../shared/types'

export type FrozenPageBitmaps = ReadonlyMap<string, ImageBitmap>

const EMPTY: FrozenPageBitmaps = new Map()

/**
 * Decodes the frozen-page frames into GPU-ready bitmaps, keyed by page id,
 * and acks the revision to main once every frame is drawable. Main waits on
 * that ack before parking the live views, so the canvas never shows a gap
 * between "view hidden" and "raster drawn".
 */
export function useFrozenPageBitmaps(
  snapshot: FrozenPagesState,
  onReady: (revision: number) => void,
): FrozenPageBitmaps {
  const [bitmaps, setBitmaps] = useState<FrozenPageBitmaps>(EMPTY)

  // A published map stays drawable until its replacement is ready; closing
  // it any earlier detaches bitmaps the chrome canvas may still redraw.
  const publish = (next: FrozenPageBitmaps) =>
    setBitmaps((prev) => {
      for (const bitmap of prev.values()) bitmap.close()
      return next
    })

  useEffect(() => {
    if (snapshot.frames.length === 0) {
      publish(EMPTY)
      return
    }
    let cancelled = false
    let published = false
    const decoded = new Map<string, ImageBitmap>()
    void Promise.allSettled(
      snapshot.frames.map(async (frame) => {
        const image = new Image()
        image.src = frame.dataUrl
        await image.decode()
        const bitmap = await createImageBitmap(image)
        if (cancelled) bitmap.close()
        else decoded.set(frame.pageId, bitmap)
      }),
    ).then(() => {
      if (cancelled) return
      published = true
      publish(decoded)
      onReady(snapshot.revision)
    })
    return () => {
      cancelled = true
      if (!published) for (const bitmap of decoded.values()) bitmap.close()
    }
  }, [snapshot.revision])

  return bitmaps
}
