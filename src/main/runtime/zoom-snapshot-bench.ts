import { ipcMain } from 'electron'
import type { NativeImage } from 'electron'
import { ipcChannels } from '../../shared/ipc-contract'
import type {
  ZoomSnapshotBenchFrame,
  ZoomSnapshotBenchPayload,
  ZoomSnapshotBenchResult,
  ZoomSnapshotBenchVariant,
} from '../../shared/types'
import { pages } from './runtime-context'
import { safeSend } from './safe-send'
import { bgView } from './view-refs'
import { pageSnapBounds } from './runtime-geometry'
import { boundAvailableCanvasViewportRect as availableCanvasViewportRect } from './runtime-geometry'
import { panToCenterBoundsAtZoom, setViewportCamera } from './viewport-control'
import { clampCanvasZoom } from '../../shared/zoom'

/**
 * Times every link of the snapshot refresh pipeline separately so transport
 * decisions rest on numbers, not guesses:
 *
 *   capturePage → encode (main thread, sync) → IPC → renderer decode
 *
 * Each encoding variant is measured end to end against the same captures.
 * Diagnostic only; reached through POST /perf/zoom-snapshot/bench.
 */

const VARIANTS: ZoomSnapshotBenchVariant[] = [
  'png-dataurl',
  'jpeg85-dataurl',
  'jpeg70-dataurl',
  'raw-bitmap',
]

interface EncodeStats {
  variant: ZoomSnapshotBenchVariant
  encodeMsPerPage: number[]
  encodeMsTotal: number
  bytesTotal: number
  ipcMs: number | null
  decodeMs: number | null
  decodedCount: number | null
}

export interface ZoomSnapshotBenchReport {
  pageCount: number
  capturedCount: number
  captureMsPerPage: number[]
  captureWallMs: number
  capturedPixels: { width: number; height: number }[]
  variants: EncodeStats[]
}

let nextBenchId = 1
const pendingResults = new Map<
  number,
  (result: ZoomSnapshotBenchResult | null) => void
>()

export function registerZoomSnapshotBenchIpc(): void {
  ipcMain.on(
    ipcChannels.zoomSnapshotBenchResult,
    (_event, result: ZoomSnapshotBenchResult) => {
      const resolve = pendingResults.get(result.benchId)
      if (!resolve) return
      pendingResults.delete(result.benchId)
      resolve(result)
    },
  )
}

function encode(
  image: NativeImage,
  variant: ZoomSnapshotBenchVariant,
  pageId: string,
): { frame: ZoomSnapshotBenchFrame; ms: number; bytes: number } {
  const size = image.getSize()
  const start = performance.now()
  let frame: ZoomSnapshotBenchFrame
  let bytes: number
  switch (variant) {
    case 'png-dataurl': {
      const dataUrl = image.toDataURL()
      bytes = dataUrl.length
      frame = { pageId, kind: 'dataUrl', dataUrl, width: size.width, height: size.height }
      break
    }
    case 'jpeg85-dataurl': {
      const dataUrl = `data:image/jpeg;base64,${image.toJPEG(85).toString('base64')}`
      bytes = dataUrl.length
      frame = { pageId, kind: 'dataUrl', dataUrl, width: size.width, height: size.height }
      break
    }
    case 'jpeg70-dataurl': {
      const dataUrl = `data:image/jpeg;base64,${image.toJPEG(70).toString('base64')}`
      bytes = dataUrl.length
      frame = { pageId, kind: 'dataUrl', dataUrl, width: size.width, height: size.height }
      break
    }
    case 'raw-bitmap': {
      const buffer = image.toBitmap()
      bytes = buffer.byteLength
      frame = {
        pageId,
        kind: 'raw',
        pixels: new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength),
        width: size.width,
        height: size.height,
      }
      break
    }
  }
  return { frame, ms: performance.now() - start, bytes }
}

function sendAndAwaitDecode(
  payload: ZoomSnapshotBenchPayload,
  timeoutMs = 10_000,
): Promise<ZoomSnapshotBenchResult | null> {
  if (!bgView || bgView.webContents.isDestroyed()) return Promise.resolve(null)
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      pendingResults.delete(payload.benchId)
      resolve(null)
    }, timeoutMs)
    pendingResults.set(payload.benchId, (result) => {
      clearTimeout(timer)
      resolve(result)
    })
    safeSend(bgView!.webContents, ipcChannels.zoomSnapshotBench, payload)
  })
}

export async function runZoomSnapshotBench(options?: {
  variants?: ZoomSnapshotBenchVariant[]
}): Promise<ZoomSnapshotBenchReport> {
  const variants = options?.variants ?? VARIANTS

  const captureWallStart = performance.now()
  const captures = await Promise.all(
    pages.map(async (page) => {
      if (page.pageView.webContents.isDestroyed()) return null
      const bounds = page.pageView.getBounds()
      if (bounds.width <= 0 || bounds.height <= 0) return null
      const start = performance.now()
      const image = await page.pageView.webContents.capturePage()
      const ms = performance.now() - start
      if (image.isEmpty()) return null
      return { pageId: page.id, image, ms }
    }),
  )
  const captureWallMs = performance.now() - captureWallStart
  const captured = captures.filter(
    (c): c is { pageId: string; image: NativeImage; ms: number } => c !== null,
  )

  const variantStats: EncodeStats[] = []
  for (const variant of variants) {
    const frames: ZoomSnapshotBenchFrame[] = []
    const encodeMsPerPage: number[] = []
    let bytesTotal = 0
    for (const { pageId, image } of captured) {
      const { frame, ms, bytes } = encode(image, variant, pageId)
      frames.push(frame)
      encodeMsPerPage.push(ms)
      bytesTotal += bytes
    }
    const benchId = nextBenchId++
    const sentAt = Date.now()
    const result = await sendAndAwaitDecode({ benchId, variant, sentAt, frames })
    variantStats.push({
      variant,
      encodeMsPerPage,
      encodeMsTotal: encodeMsPerPage.reduce((a, b) => a + b, 0),
      bytesTotal,
      ipcMs: result ? result.receivedAt - sentAt : null,
      decodeMs: result?.decodeMs ?? null,
      decodedCount: result?.decodedCount ?? null,
    })
  }

  return {
    pageCount: pages.length,
    capturedCount: captured.length,
    captureMsPerPage: captured.map((c) => c.ms),
    captureWallMs,
    capturedPixels: captured.map((c) => c.image.getSize()),
    variants: variantStats,
  }
}

/** Frames every page in the window so each one has live pixels to capture. */
export function fitAllPagesForBench(options?: { zoom?: number }, marginPx = 40): void {
  if (pages.length === 0) return
  const rects = pages.map((page) => pageSnapBounds(page))
  const minX = Math.min(...rects.map((r) => r.x))
  const minY = Math.min(...rects.map((r) => r.y))
  const maxX = Math.max(...rects.map((r) => r.x + r.width))
  const maxY = Math.max(...rects.map((r) => r.y + r.height))
  const union = { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
  const viewport = availableCanvasViewportRect()
  const zoom = clampCanvasZoom(
    options?.zoom ?? Math.min(
      (viewport.width - marginPx * 2) / union.width,
      (viewport.height - marginPx * 2) / union.height,
    ),
  )
  setViewportCamera(zoom, panToCenterBoundsAtZoom(union, zoom))
}
