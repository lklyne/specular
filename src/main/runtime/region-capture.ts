/**
 * Captures a composited screenshot of a canvas region spanning multiple pages.
 *
 * For each page intersecting the bounding box, captures via captureFrameComposited(),
 * then composites all page captures onto a canvas-background-colored buffer.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import { nativeImage, screen as electronScreen, type WebContents } from 'electron'
import type { WorkspaceBounds } from '../../shared/types'
import {
  captureFrameComposited,
  captureViewRegion,
  type CompositedCapture,
} from './frame-compositor'
import { boundCanvasOrigin, boundsOverlap, pageBodyCanvasBounds } from './runtime-geometry'
import { pages, zoom, pan } from './runtime-context'
import { aboveView, bgView } from './view-refs'
import { win } from './window-shell'
import type { Page } from './runtime-entities'
import {
  deviceIdFromMetadata,
  deviceOrientationFromMetadata,
  showDeviceFrameFromMetadata,
} from './runtime-entities'
import { contentCornerRadiusForDevice } from '../../shared/device-catalog'

function sendCaptureMode(webContents: WebContents | undefined, active: boolean): void {
  if (!webContents || webContents.isDestroyed()) return
  webContents.send(ipcChannels.captureMode, active)
}

function captureModeTargets(): WebContents[] {
  const viewTargets = [bgView, aboveView]
    .map((view) => view?.webContents)
    .filter((webContents): webContents is WebContents => Boolean(webContents))
  const pageTargets = pages.map((page) => page.pageView.webContents)
  return [...viewTargets, ...pageTargets]
}

function setRendererCaptureMode(active: boolean): void {
  for (const webContents of captureModeTargets()) {
    sendCaptureMode(webContents, active)
  }
}

function canvasRectToScreenRect(canvasRect: WorkspaceBounds): {
  x: number
  y: number
  width: number
  height: number
} {
  const origin = boundCanvasOrigin()
  return {
    x: origin.x + canvasRect.x * zoom + pan.x,
    y: origin.y + canvasRect.y * zoom + pan.y,
    width: canvasRect.width * zoom,
    height: canvasRect.height * zoom,
  }
}

function pageCornerRadiusPx(page: Page, dpr: number): number {
  const deviceId = deviceIdFromMetadata(page.metadata)
  if (!deviceId || !showDeviceFrameFromMetadata(page.metadata)) return 0
  const orientation = deviceOrientationFromMetadata(page.metadata)
  return Math.round(contentCornerRadiusForDevice(deviceId, orientation) * zoom * dpr)
}

/**
 * Draw a view capture into the output buffer at the offset it was captured
 * from, clipping anything past the edges. `blend` alpha-composites (overlay
 * layers); without it the source replaces the destination (base layer).
 */
export function blitCapture(
  capture: CompositedCapture,
  outBuf: Buffer,
  outW: number,
  outH: number,
  opts: { blend: boolean },
): void {
  const { bitmap: src, width: srcW, height: srcH } = capture
  const offsetX = capture.offsetX ?? 0
  const offsetY = capture.offsetY ?? 0
  const rowEnd = Math.min(srcH, outH - offsetY)
  const colEnd = Math.min(srcW, outW - offsetX)

  for (let row = 0; row < rowEnd; row++) {
    for (let col = 0; col < colEnd; col++) {
      const srcIdx = (row * srcW + col) * 4
      const destIdx = ((offsetY + row) * outW + offsetX + col) * 4
      const alpha = src[srcIdx + 3] / 255
      if (opts.blend && alpha === 0) continue
      const blend = opts.blend ? alpha : 1
      outBuf[destIdx] = Math.round(src[srcIdx] * blend + outBuf[destIdx] * (1 - blend))
      outBuf[destIdx + 1] = Math.round(src[srcIdx + 1] * blend + outBuf[destIdx + 1] * (1 - blend))
      outBuf[destIdx + 2] = Math.round(src[srcIdx + 2] * blend + outBuf[destIdx + 2] * (1 - blend))
      outBuf[destIdx + 3] = 0xff
    }
  }
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

// fallow-ignore-next-line complexity
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

  // Base layer: canvas-background gray, with the bgView capture drawn over it.
  // A region larger than the window captures only its visible part, so the
  // fill stays visible around whatever pixels came back.
  const outBuf = Buffer.alloc(outW * outH * 4)
  for (let i = 0; i < outBuf.length; i += 4) {
    outBuf[i] = 0xf5; outBuf[i + 1] = 0xf5; outBuf[i + 2] = 0xf5; outBuf[i + 3] = 0xff
  }

  if (opts?.includeBgView && bgView && !bgView.webContents.isDestroyed()) {
    // Convert canvas rect to screen coordinates for the bgView crop.
    const screenRect = canvasRectToScreenRect(canvasRect)
    const bgCapture = await captureViewRegion(bgView, screenRect, { dpr })
    if (bgCapture) blitCapture(bgCapture, outBuf, outW, outH, { blend: false })
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

  // Composite above-view (entity bodies: drawings, stickies, notes, shapes) on top.
  if (aboveView && !aboveView.webContents.isDestroyed()) {
    const screenRect = canvasRectToScreenRect(canvasRect)
    const aboveCapture = await captureViewRegion(aboveView, screenRect, { dpr })
    if (aboveCapture) blitCapture(aboveCapture, outBuf, outW, outH, { blend: true })
  }

  const result = nativeImage.createFromBitmap(outBuf, { width: outW, height: outH })
  const base64 = result.toPNG().toString('base64')

  return { base64, width: outW, height: outH, intersectingPages }
}
