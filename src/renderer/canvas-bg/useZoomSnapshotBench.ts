import { useEffect } from 'react'
import type { ZoomSnapshotBenchFrame, ZoomSnapshotBenchPayload, ZoomSnapshotBenchResult } from '../../shared/types'

async function decodeFrame(frame: ZoomSnapshotBenchFrame): Promise<void> {
  if (frame.kind === 'dataUrl') {
    const image = new Image()
    image.src = frame.dataUrl
    await image.decode()
    return
  }
  // Copy into a plain ArrayBuffer: ImageData rejects the SharedArrayBuffer-typed
  // view structured-clone can hand back.
  const pixels = new Uint8ClampedArray(frame.pixels.byteLength)
  pixels.set(frame.pixels)
  const data = new ImageData(pixels, frame.width, frame.height)
  await createImageBitmap(data)
}

/** Renderer half of the snapshot pipeline bench: decode what main sent, report timings. */
export function useZoomSnapshotBench(api: {
  onZoomSnapshotBench: (cb: (payload: ZoomSnapshotBenchPayload) => void) => () => void
  zoomSnapshotBenchResult: (result: ZoomSnapshotBenchResult) => void
}): void {
  useEffect(
    () =>
      api.onZoomSnapshotBench((payload) => {
        const receivedAt = Date.now()
        const start = performance.now()
        void Promise.allSettled(payload.frames.map(decodeFrame)).then((settled) => {
          api.zoomSnapshotBenchResult({
            benchId: payload.benchId,
            receivedAt,
            decodeMs: performance.now() - start,
            decodedCount: settled.filter((s) => s.status === 'fulfilled').length,
          })
        })
      }),
    [],
  )
}
