/**
 * Captures a composited screenshot of a canvas region spanning multiple pages.
 *
 * For each page intersecting the bounding box, captures via captureFrameComposited(),
 * then composites all page captures onto a canvas-background-colored buffer.
 */

import { nativeImage, screen as electronScreen } from 'electron'
import type { WorkspaceBounds } from '../../shared/types'
import { captureFrameComposited, captureViewRegion } from './frame-compositor'
import { boundCanvasOrigin, boundsOverlap, pageBodyCanvasBounds } from './runtime-geometry'
import { aboveView, bgView } from './view-refs'

function setRendererCaptureMode(active: boolean): void {
  for (const view of [bgView, aboveView]) {
    if (view && !view.webContents.isDestroyed()) {
      view.webContents.send('capture-mode', active)
    }
  }
}
import { pages, zoom, pan } from './runtime-context'
import { win } from './window-shell'
import type { Page } from './runtime-entities'
import {
  deviceIdFromMetadata,
  deviceOrientationFromMetadata,
  showDeviceFrameFromMetadata,
} from './runtime-entities'
import { contentCornerRadiusForDevice } from '../../shared/device-catalog'

function pageCornerRadiusPx(page: Page, dpr: number): number {
  const deviceId = deviceIdFromMetadata(page.metadata)
  if (!deviceId || !showDeviceFrameFromMetadata(page.metadata)) return 0
  const orientation = deviceOrientationFromMetadata(page.metadata)
  return Math.round(contentCornerRadiusForDevice(deviceId, orientation) * zoom * dpr)
}

interface RegionCaptureResult {
  base64: string
  width: number
  height: number
  intersectingPages: Page[]
}

export interface RegionCaptureOptions {
  /** Capture the canvas background view (text notes, page chrome, grid). */
  includeBgView?: boolean
}

/**
 * Capture a composited screenshot of a canvas region.
 *
 * Returns the composited PNG base64 and the list of pages that intersected the region.
 * When `includeBgView` is true, the canvas background (text notes, page chrome, grid)
 * is used as the base layer instead of a solid fill.
 */
export async function captureRegion(
  canvasRect: WorkspaceBounds,
  opts?: RegionCaptureOptions,
): Promise<RegionCaptureResult> {
  if (!win || win.isDestroyed()) {
    throw new Error('Window not available')
  }

  const display = electronScreen.getDisplayMatching(win.getBounds())
  const dpr = display.scaleFactor

  setRendererCaptureMode(true)
  try {
    // Allow renderers one page to hide transient UI (selection outlines,
    // marquee, region composer, etc.) before capture.
    await new Promise((r) => setTimeout(r, 32))
    return await captureRegionInternal(canvasRect, opts, dpr)
  } finally {
    setRendererCaptureMode(false)
  }
}

async function captureRegionInternal(
  canvasRect: WorkspaceBounds,
  opts: RegionCaptureOptions | undefined,
  dpr: number,
): Promise<RegionCaptureResult> {

  // Find pages whose body bounds intersect the region.
  const intersectingPages = pages.filter((page) => {
    const bounds = pageBodyCanvasBounds(page)
    return boundsOverlap(canvasRect, bounds)
  })

  // Output buffer dimensions in physical pixels.
  const outW = Math.round(canvasRect.width * zoom * dpr)
  const outH = Math.round(canvasRect.height * zoom * dpr)

  if (outW <= 0 || outH <= 0) {
    throw new Error('Region has zero dimensions')
  }

  // Base layer: either bgView capture or solid gray fill.
  let outBuf: Buffer

  if (opts?.includeBgView && bgView && !bgView.webContents.isDestroyed()) {
    // Convert canvas rect to screen coordinates for the bgView crop.
    const origin = boundCanvasOrigin()
    const screenRect = {
      x: origin.x + canvasRect.x * zoom + pan.x,
      y: origin.y + canvasRect.y * zoom + pan.y,
      width: canvasRect.width * zoom,
      height: canvasRect.height * zoom,
    }
    const bgCapture = await captureViewRegion(bgView, screenRect, { dpr })
    if (bgCapture && bgCapture.width === outW && bgCapture.height === outH) {
      outBuf = bgCapture.bitmap
    } else {
      // Fallback: solid gray fill if bgView capture doesn't match dimensions.
      outBuf = Buffer.alloc(outW * outH * 4)
      for (let i = 0; i < outBuf.length; i += 4) {
        outBuf[i] = 0xf5; outBuf[i + 1] = 0xf5; outBuf[i + 2] = 0xf5; outBuf[i + 3] = 0xff
      }
    }
  } else {
    outBuf = Buffer.alloc(outW * outH * 4)
    for (let i = 0; i < outBuf.length; i += 4) {
      outBuf[i] = 0xf5; outBuf[i + 1] = 0xf5; outBuf[i + 2] = 0xf5; outBuf[i + 3] = 0xff
    }
  }

  // Capture each intersecting page and blit into output buffer.
  for (const page of intersectingPages) {
    const capture = await captureFrameComposited(page, { dpr })
    if (!capture) continue

    const pageBounds = pageBodyCanvasBounds(page)

    const offsetX = Math.round((pageBounds.x - canvasRect.x) * zoom * dpr)
    const offsetY = Math.round((pageBounds.y - canvasRect.y) * zoom * dpr)

    // Blit the page capture into the output buffer.
    const srcW = capture.width
    const srcH = capture.height
    const src = capture.bitmap

    // Clip loop bounds upfront to avoid per-pixel branching.
    const rowStart = Math.max(0, -offsetY)
    const rowEnd = Math.min(srcH, outH - offsetY)
    const colStart = Math.max(0, -offsetX)
    const colEnd = Math.min(srcW, outW - offsetX)

    // Corner radius mask — WebContentsView's setBorderRadius clips the view
    // visually, but capturePage returns the unclipped rectangular bitmap.
    // Skip pixels outside the rounded rect so device-framed pages render
    // with their rounded interior.
    const radius = pageCornerRadiusPx(page, dpr)
    const rMax = Math.min(radius, Math.floor(Math.min(srcW, srcH) / 2))

    for (let row = rowStart; row < rowEnd; row++) {
      const destRow = offsetY + row
      for (let col = colStart; col < colEnd; col++) {
        if (rMax > 0) {
          const dxCorner =
            col < rMax ? rMax - col - 1 : col >= srcW - rMax ? col - (srcW - rMax) : 0
          const dyCorner =
            row < rMax ? rMax - row - 1 : row >= srcH - rMax ? row - (srcH - rMax) : 0
          if (dxCorner > 0 && dyCorner > 0 && dxCorner * dxCorner + dyCorner * dyCorner > rMax * rMax) {
            continue
          }
        }
        const srcIdx = (row * srcW + col) * 4
        const alpha = src[srcIdx + 3] / 255
        if (alpha === 0) continue
        const destIdx = (destRow * outW + (offsetX + col)) * 4
        outBuf[destIdx] = Math.round(src[srcIdx] * alpha + outBuf[destIdx] * (1 - alpha))
        outBuf[destIdx + 1] = Math.round(src[srcIdx + 1] * alpha + outBuf[destIdx + 1] * (1 - alpha))
        outBuf[destIdx + 2] = Math.round(src[srcIdx + 2] * alpha + outBuf[destIdx + 2] * (1 - alpha))
        outBuf[destIdx + 3] = 0xff
      }
    }
  }

  // Composite above-view (drawing strokes, annotation overlays) on top.
  if (aboveView && !aboveView.webContents.isDestroyed()) {
    const origin = boundCanvasOrigin()
    const screenRect = {
      x: origin.x + canvasRect.x * zoom + pan.x,
      y: origin.y + canvasRect.y * zoom + pan.y,
      width: canvasRect.width * zoom,
      height: canvasRect.height * zoom,
    }
    const aboveCapture = await captureViewRegion(aboveView, screenRect, { dpr })
    if (aboveCapture && aboveCapture.width === outW && aboveCapture.height === outH) {
      const src = aboveCapture.bitmap
      for (let i = 0; i < outBuf.length; i += 4) {
        const alpha = src[i + 3] / 255
        if (alpha === 0) continue
        outBuf[i] = Math.round(src[i] * alpha + outBuf[i] * (1 - alpha))
        outBuf[i + 1] = Math.round(src[i + 1] * alpha + outBuf[i + 1] * (1 - alpha))
        outBuf[i + 2] = Math.round(src[i + 2] * alpha + outBuf[i + 2] * (1 - alpha))
        outBuf[i + 3] = 0xff
      }
    }
  }

  const result = nativeImage.createFromBitmap(outBuf, { width: outW, height: outH })
  const base64 = result.toPNG().toString('base64')

  return { base64, width: outW, height: outH, intersectingPages }
}
