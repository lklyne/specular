/**
 * Shared compositing helper — captures a frame's page content with its
 * overlays alpha-blended on top, in z-order:
 *   1. page webContents
 *   2. aboveView (comments, annotations, drawing, marquee, floating UI)
 *   3. cursorOverlayWindow (agent presence cursors + particle trails)
 *
 * Used by video-recorder.ts (per-frame capture loop) and
 * app-control-server.ts (/frames/screenshot-composite endpoint).
 */

import { screen as electronScreen } from 'electron'
import type { NativeImage, WebContents, WebContentsView } from 'electron'
import { aboveView, cursorOverlayWindow } from './view-refs'
import type { Page } from './runtime-entities'
import { win } from './window-shell'
import { boundScreenBoundsForPage } from './runtime-geometry'

export interface CompositedCapture {
  bitmap: Buffer
  width: number // physical pixels
  height: number // physical pixels
  /** Where the captured pixels sit inside the requested rect, in physical
   *  pixels. Non-zero when the rect ran past the view's edge and only part of
   *  it could be captured. */
  offsetX?: number
  offsetY?: number
}

/**
 * Capture a single frame with its overlay composited on top.
 *
 * Returns null if the capture fails (destroyed webContents, empty image, etc.).
 * Callers that need a specific DPR can pass it in opts to avoid re-querying
 * the display (useful in hot loops like video recording).
 */
export async function captureFrameComposited(
  page: Page,
  opts?: { dpr?: number },
): Promise<CompositedCapture | null> {
  if (page.pageView.webContents.isDestroyed()) return null

  const dpr =
    opts?.dpr ??
    (win && !win.isDestroyed()
      ? electronScreen.getDisplayMatching(win.getBounds()).scaleFactor
      : 1)

  // Capture the annotations/comments overlay (aboveView WCV) and the agent
  // presence cursor overlay (a child BrowserWindow — lives outside the WCV
  // stack for click-through, see view-refs.ts). All three captures run in
  // parallel; we then alpha-blend in z-order: page < aboveView < cursor overlay.
  const canCapture = win && !win.isDestroyed()
  const pageRect = boundScreenBoundsForPage(page).page

  const abovePromise =
    canCapture && aboveView && !aboveView.webContents.isDestroyed()
      ? captureAndCropToRect(aboveView.webContents, aboveView.getBounds(), pageRect, dpr)
      : Promise.resolve(null)

  // BrowserWindow.getBounds() is in screen coords; aboveView.getBounds() is
  // window-relative. Translate to window-relative so both overlays share the
  // coordinate space pageRect uses.
  const cursorWc = cursorOverlayWindow?.webContents
  let cursorPromise: Promise<NativeImage | null> = Promise.resolve(null)
  if (canCapture && cursorWc && !cursorWc.isDestroyed() && cursorOverlayWindow) {
    const contentBounds = win!.getContentBounds()
    const cursorScreenBounds = cursorOverlayWindow.getBounds()
    cursorPromise = captureAndCropToRect(
      cursorWc,
      {
        x: cursorScreenBounds.x - contentBounds.x,
        y: cursorScreenBounds.y - contentBounds.y,
        width: cursorScreenBounds.width,
        height: cursorScreenBounds.height,
      },
      pageRect,
      dpr,
    )
  }

  const [pageImage, aboveOverlay, cursorOverlay] = await Promise.all([
    page.pageView.webContents.capturePage(),
    abovePromise,
    cursorPromise,
  ])

  if (pageImage.isEmpty()) return null
  const pageSize = pageImage.getSize()
  if (pageSize.width === 0 || pageSize.height === 0) return null

  const baseBitmap = pageImage.toBitmap()
  blendOnto(baseBitmap, aboveOverlay, pageSize)
  blendOnto(baseBitmap, cursorOverlay, pageSize)

  return { bitmap: baseBitmap, width: pageSize.width, height: pageSize.height }
}

/**
 * Capture an overlay WebContents and crop to the given target rect (in the
 * overlay's coordinate space). Works for both WebContentsView overlays and
 * child BrowserWindow overlays — callers pass each overlay's bounds in the
 * same coordinate space as `targetRect`.
 */
async function captureAndCropToRect(
  wc: WebContents,
  overlayBounds: { x: number; y: number; width: number; height: number },
  targetRect: { x: number; y: number; width: number; height: number },
  dpr: number,
): Promise<NativeImage | null> {
  const cropX = Math.round(Math.max(0, targetRect.x - overlayBounds.x) * dpr)
  const cropY = Math.round(Math.max(0, targetRect.y - overlayBounds.y) * dpr)
  const cropW = Math.round(
    Math.min(targetRect.width, overlayBounds.width - (targetRect.x - overlayBounds.x)) * dpr,
  )
  const cropH = Math.round(
    Math.min(targetRect.height, overlayBounds.height - (targetRect.y - overlayBounds.y)) * dpr,
  )
  if (cropW <= 0 || cropH <= 0) return null

  const full = await wc.capturePage()
  if (full.isEmpty()) return null
  const fullSize = full.getSize()
  const safeW = Math.min(cropW, fullSize.width - cropX)
  const safeH = Math.min(cropH, fullSize.height - cropY)
  if (safeW <= 0 || safeH <= 0) return null
  return full.crop({ x: cropX, y: cropY, width: safeW, height: safeH })
}

function blendOnto(
  baseBitmap: Buffer,
  overlay: NativeImage | null,
  pageSize: { width: number; height: number },
): void {
  if (!overlay || overlay.isEmpty()) return
  const overlaySize = overlay.getSize()
  if (overlaySize.width !== pageSize.width || overlaySize.height !== pageSize.height) return
  const overBitmap = overlay.toBitmap()
  for (let i = 0; i < baseBitmap.length; i += 4) {
    const alpha = overBitmap[i + 3] / 255
    if (alpha === 0) continue
    baseBitmap[i] = Math.round(overBitmap[i] * alpha + baseBitmap[i] * (1 - alpha))
    baseBitmap[i + 1] = Math.round(overBitmap[i + 1] * alpha + baseBitmap[i + 1] * (1 - alpha))
    baseBitmap[i + 2] = Math.round(overBitmap[i + 2] * alpha + baseBitmap[i + 2] * (1 - alpha))
    baseBitmap[i + 3] = Math.max(baseBitmap[i + 3], overBitmap[i + 3])
  }
}

/**
 * Capture a rectangular region from an arbitrary WebContentsView.
 *
 * Captures the full view, then crops to the specified screen-coordinate rect.
 * Useful for capturing bgView (text notes, frame chrome, grid) or any overlay.
 */
export async function captureViewRegion(
  view: WebContentsView,
  screenRect: { x: number; y: number; width: number; height: number },
  opts?: { dpr?: number },
): Promise<CompositedCapture | null> {
  if (view.webContents.isDestroyed()) return null

  const dpr =
    opts?.dpr ??
    (win && !win.isDestroyed()
      ? electronScreen.getDisplayMatching(win.getBounds()).scaleFactor
      : 1)

  const fullImage = await view.webContents.capturePage()
  if (fullImage.isEmpty()) return null

  const viewBounds = view.getBounds()
  const cropX = Math.round(Math.max(0, screenRect.x - viewBounds.x) * dpr)
  const cropY = Math.round(Math.max(0, screenRect.y - viewBounds.y) * dpr)
  // A rect starting left of / above the view captures from the view's origin,
  // so those pixels belong that far into the requested rect — not at 0,0.
  const offsetX = Math.round(Math.max(0, viewBounds.x - screenRect.x) * dpr)
  const offsetY = Math.round(Math.max(0, viewBounds.y - screenRect.y) * dpr)
  const cropW = Math.round(screenRect.width * dpr) - offsetX
  const cropH = Math.round(screenRect.height * dpr) - offsetY

  if (cropW <= 0 || cropH <= 0) return null

  const fullSize = fullImage.getSize()
  const safeW = Math.min(cropW, fullSize.width - cropX)
  const safeH = Math.min(cropH, fullSize.height - cropY)
  if (safeW <= 0 || safeH <= 0) return null

  const cropped = fullImage.crop({ x: cropX, y: cropY, width: safeW, height: safeH })
  return { bitmap: cropped.toBitmap(), width: safeW, height: safeH, offsetX, offsetY }
}
