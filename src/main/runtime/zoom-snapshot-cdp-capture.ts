import type { Page } from './runtime-entities'
import { requestLayout } from './layout-engine'

// Once the DevTools emulation handler owns metrics for a page, a debugger
// detach clears them along with Electron's emulation. The next layout pass
// re-applies it.
const detachListenerAttached = new WeakSet<Electron.WebContents>()

export interface CdpCapture {
  /** JPEG bytes, encoded in the page's renderer process. */
  jpeg: Buffer
  width: number
  height: number
  ms: number
}

/**
 * Rasters the page's viewport at `scale` times its emulated size through
 * `Page.captureScreenshot`, in the renderer process. Unlike `capturePage`,
 * which copies the compositor surface at on-screen size, this produces pixels
 * the surface never had, so a page zoomed far out can still yield a frame
 * crisp enough to zoom back into. Encoding happens in the renderer too, which
 * keeps the main thread free of the JPEG cost.
 */
export async function captureViaCdp(
  page: Page,
  options: {
    scale: number
    quality?: number
    cssWidth: number
    cssHeight: number
    /** The emulation the page is showing: restored after the capture. */
    emulation: { deviceScaleFactor: number; scale: number }
  },
): Promise<CdpCapture> {
  const wc = page.pageView.webContents
  const startedAt = performance.now()
  if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')
  if (!detachListenerAttached.has(wc)) {
    detachListenerAttached.add(wc)
    wc.debugger.on('detach', () => {
      detachListenerAttached.delete(wc)
      page.lastPageEmulationKey = undefined
      requestLayout()
    })
  }
  // captureScreenshot installs a temporary metrics override and then restores
  // the DevTools handler's own previous params. Electron's enableDeviceEmulation
  // sets the same widget state but not through that handler, so without this
  // seed the restore lands on "no emulation" and the page snaps to native scale.
  await wc.debugger.sendCommand('Emulation.setDeviceMetricsOverride', {
    width: options.cssWidth,
    height: options.cssHeight,
    deviceScaleFactor: options.emulation.deviceScaleFactor,
    scale: options.emulation.scale,
    mobile: false,
    screenWidth: options.cssWidth,
    screenHeight: options.cssHeight,
    positionX: 0,
    positionY: 0,
  })
  const result = (await wc.debugger.sendCommand('Page.captureScreenshot', {
    format: 'jpeg',
    quality: options.quality ?? 85,
    clip: {
      x: 0,
      y: 0,
      width: options.cssWidth,
      height: options.cssHeight,
      scale: options.scale,
    },
    fromSurface: true,
    captureBeyondViewport: false,
  })) as { data: string }
  const jpeg = Buffer.from(result.data, 'base64')
  const { width, height } = jpegDimensions(jpeg)
  return { jpeg, width, height, ms: performance.now() - startedAt }
}

/** Reads the SOF0/SOF2 frame header; avoids a decode just to learn the size. */
export function jpegDimensions(buf: Buffer): { width: number; height: number } {
  let i = 2
  while (i < buf.length) {
    if (buf[i] !== 0xff) { i += 1; continue }
    const marker = buf[i + 1]
    if (marker === 0xc0 || marker === 0xc1 || marker === 0xc2) {
      return { height: buf.readUInt16BE(i + 5), width: buf.readUInt16BE(i + 7) }
    }
    if (marker === 0xd8 || marker === 0x01 || (marker >= 0xd0 && marker <= 0xd7)) { i += 2; continue }
    i += 2 + buf.readUInt16BE(i + 2)
  }
  return { width: 0, height: 0 }
}
