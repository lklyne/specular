/**
 * A page's substrate: one hidden offscreen `BrowserWindow` per page, painting
 * GPU shared textures that main forwards to canvas-bg's main frame. The canvas
 * draws those textures, so a page occupies no native view and costs the
 * compositor nothing per page (ADR 0038).
 *
 * `BrowserWindow` rather than `WebContentsView` because an offscreen
 * WebContentsView paints at its parent window's size (electron#45864).
 */

import { BrowserWindow, screen, sharedTexture, type WebContents } from 'electron'
import type { PageFrameMeta } from '../../shared/page-frames'
import { preloadPath } from './load-renderer'

/** Chromium's OSR frame pool holds 10 textures per page; keep headroom. */
const MAX_OUTSTANDING_TEXTURES = 9

/** Rolling window for the release-latency mean. */
const LATENCY_SAMPLES = 30

export interface PageHostStats {
  pageId: string
  framesReceived: number
  popupFrames: number
  framesWithoutTexture: number
  framesDroppedForPoolPressure: number
  sendFailures: number
  outstandingTextures: number
  maxOutstandingTextures: number
  releaseLatencyMs: number | null
}

export interface PageHost {
  readonly id: string
  readonly webContents: WebContents
  /** CSS viewport size the offscreen window currently has. */
  readonly size: { width: number; height: number }
  readonly painting: boolean
  resize(size: { width: number; height: number }): void
  setPainting(painting: boolean): void
  destroy(): void
  isDestroyed(): boolean
}

/**
 * The renderer frame every page texture is sent to (bgView's mainFrame). Set
 * once from window-init after bgView exists.
 */
let frameTarget: WebContents | null = null

export function setPageFrameTarget(target: WebContents | null): void {
  frameTarget = target
}

class OffscreenPageHost implements PageHost {
  readonly id: string
  private readonly win: BrowserWindow
  private currentSize: { width: number; height: number }
  private isPainting = true
  private readonly latencies: number[] = []
  readonly stats: PageHostStats

  constructor(options: { id: string; width: number; height: number }) {
    this.id = options.id
    this.currentSize = { width: options.width, height: options.height }
    this.stats = {
      pageId: options.id,
      framesReceived: 0,
      popupFrames: 0,
      framesWithoutTexture: 0,
      framesDroppedForPoolPressure: 0,
      sendFailures: 0,
      outstandingTextures: 0,
      maxOutstandingTextures: 0,
      releaseLatencyMs: null,
    }
    this.win = new BrowserWindow({
      show: false,
      width: options.width,
      height: options.height,
      // The page's CSS viewport is the content size; a hidden window still
      // has a title bar, and the default would take it out of the height.
      useContentSize: true,
      webPreferences: {
        preload: preloadPath('page-content'),
        focusOnNavigation: false,
        contextIsolation: true,
        nodeIntegration: false,
        // An offscreen page is never "visible" to Chromium; without this it
        // would idle at background cadence the moment it loads.
        backgroundThrottling: false,
        offscreen: {
          useSharedTexture: true,
          deviceScaleFactor: screen.getPrimaryDisplay().scaleFactor,
        },
      },
    })
    this.win.webContents.on('paint', (event, dirtyRect) => {
      if (this.win.isDestroyed()) return
      const texture = (event as Electron.Event<Electron.WebContentsPaintEventParams>).texture
      if (!texture) {
        this.stats.framesWithoutTexture++
        return
      }
      this.deliver(texture, dirtyRect)
    })
  }

  get webContents(): WebContents {
    return this.win.webContents
  }

  get size(): { width: number; height: number } {
    return this.currentSize
  }

  get painting(): boolean {
    return this.isPainting
  }

  resize(size: { width: number; height: number }): void {
    const width = Math.max(1, Math.round(size.width))
    const height = Math.max(1, Math.round(size.height))
    if (width === this.currentSize.width && height === this.currentSize.height) return
    if (this.win.isDestroyed()) return
    this.currentSize = { width, height }
    this.win.setContentSize(width, height)
  }

  setPainting(painting: boolean): void {
    if (painting === this.isPainting || this.win.isDestroyed()) return
    this.isPainting = painting
    const contents = this.win.webContents
    if (painting) {
      contents.startPainting()
      // A resumed page has nothing dirty; ask for one frame so it reappears.
      contents.invalidate()
    } else {
      contents.stopPainting()
    }
  }

  destroy(): void {
    if (this.win.isDestroyed()) return
    this.win.destroy()
  }

  isDestroyed(): boolean {
    return this.win.isDestroyed()
  }

  private deliver(
    texture: Electron.OffscreenSharedTexture,
    dirtyRect: Electron.Rectangle,
  ): void {
    const paintedAt = performance.now()
    const stats = this.stats
    const target = frameTarget
    if (!target || target.isDestroyed()) {
      texture.release()
      return
    }
    if (stats.outstandingTextures >= MAX_OUTSTANDING_TEXTURES) {
      stats.framesDroppedForPoolPressure++
      texture.release()
      return
    }
    const info = texture.textureInfo
    stats.outstandingTextures++
    stats.maxOutstandingTextures = Math.max(stats.maxOutstandingTextures, stats.outstandingTextures)
    if (info.widgetType === 'popup') stats.popupFrames++
    else stats.framesReceived++

    const meta: PageFrameMeta = {
      pageId: this.id,
      widgetType: info.widgetType,
      width: info.codedSize.width,
      height: info.codedSize.height,
      cssWidth: this.currentSize.width,
      cssHeight: this.currentSize.height,
      popupRect:
        info.widgetType === 'popup'
          ? {
              x: dirtyRect.x,
              y: dirtyRect.y,
              width: dirtyRect.width,
              height: dirtyRect.height,
            }
          : null,
      frameCount: info.metadata.frameCount ?? null,
    }

    let released = false
    const finish = () => {
      if (released) return
      released = true
      stats.outstandingTextures = Math.max(0, stats.outstandingTextures - 1)
      this.latencies.push(performance.now() - paintedAt)
      if (this.latencies.length > LATENCY_SAMPLES) this.latencies.shift()
      stats.releaseLatencyMs =
        this.latencies.reduce((sum, value) => sum + value, 0) / this.latencies.length
      texture.release()
    }

    let imported: Electron.SharedTextureImported
    try {
      imported = sharedTexture.importSharedTexture({
        textureInfo: info,
        allReferencesReleased: finish,
      })
    } catch (error) {
      console.error('[page-host] importSharedTexture failed', error)
      stats.sendFailures++
      finish()
      return
    }

    void sharedTexture
      .sendSharedTexture({ frame: target.mainFrame, importedSharedTexture: imported }, meta)
      .catch((error: unknown) => {
        stats.sendFailures++
        console.error('[page-host] sendSharedTexture failed', error)
      })
      .finally(() => {
        // Main's own reference goes now; the renderer's reference keeps the
        // texture alive until it has copied the frame, then `finish` runs.
        imported.release()
      })
  }
}

const hosts = new Set<OffscreenPageHost>()

export function createPageHost(options: {
  id: string
  width: number
  height: number
}): PageHost {
  const host = new OffscreenPageHost(options)
  hosts.add(host)
  host.webContents.once('destroyed', () => hosts.delete(host))
  return host
}

export function pageHostStats(): PageHostStats[] {
  return [...hosts]
    .filter((host) => !host.isDestroyed())
    .map((host) => ({ ...host.stats }))
}
