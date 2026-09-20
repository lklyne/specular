/**
 * A page's substrate: one hidden offscreen `BrowserWindow` per page, painting
 * GPU shared textures that main forwards to canvas-bg's main frame. The canvas
 * draws those textures, so a page occupies no native view and costs the
 * compositor nothing per page (ADR 0038).
 *
 * `BrowserWindow` rather than `WebContentsView` because an offscreen
 * WebContentsView paints at its parent window's size (electron#45864).
 */

import { BrowserWindow, screen, sharedTexture, type NativeImage, type WebContents } from 'electron'
import type { PageFrameMeta } from '../../shared/page-frames'
import { forgetInputCountForPage, inputCountForPage } from './page-input-counter'
import { frameRateForDisplayScale } from './page-frame-rate'
import {
  FULL_TEXTURE_SCALE,
  TEXTURE_SIZE_TOLERANCE_PX,
  paintedCssLength,
  textureScaleForDisplayScale,
} from './page-texture-scale'
import { ensurePageDebugger } from './page-debugger'
import { preloadPath } from './load-renderer'

/**
 * Frames a page may have in flight to canvas-bg before new ones are dropped.
 * A cap, not an allocation: past it the renderer is behind, and a queued frame
 * is a stale one. Well under Chromium's OSR frame pool of 10 per page.
 */
const MAX_OUTSTANDING_TEXTURES = 6

/** Rolling window for the release-latency mean. */
const LATENCY_SAMPLES = 30

/**
 * Frame rate of a page nobody is watching — idle app or culled offscreen.
 * `setFrameRate` throttles the offscreen compositor's BeginFrame cadence,
 * which is what paces `requestAnimationFrame` — so this quiets a page's own
 * animation loop, not just texture delivery, and it is fully reversible.
 * `stopPainting` alone is not enough: it only stops texture delivery, and an
 * animating page keeps compositing full frames at 60fps for nobody. It is
 * the quieting lever for offscreen pages because `Page.setWebLifecycleState`
 * is not: thawing a never-shown window leaves its compositor without frames,
 * and every document it loads afterwards starts hidden (ADR 0035, offscreen
 * postmortem).
 */
const IDLE_FRAME_RATE = 1

/**
 * How long the camera must rest before a page's view is resized to a new
 * texture scale. A resize re-rasters the page, and a zoom gesture crossing a
 * tier boundary would otherwise resize every page on screen mid-pinch.
 * Growing waits only for the gesture to pause — the page is blurry until it
 * happens; shrinking is only a saving and can wait for the camera to settle.
 */
const TEXTURE_GROW_SETTLE_MS = 120
const TEXTURE_SHRINK_SETTLE_MS = 600

/**
 * How often a page mid-transition is asked for a frame, in case the resize
 * itself produced none to prove it landed.
 */
const TEXTURE_RESIZE_NUDGE_MS = 500

/** How long after a transition's clean frame a second one is asked for. */
const TEXTURE_CLEAN_FRAME_RETRY_MS = 250

/** Grace for a view resize to reach the page before its override is cleared. */
const CLEAR_OVERRIDE_AFTER_RESIZE_MS = 300

/**
 * How long after a failed texture transfer the page is asked for another
 * frame — long enough for a swamped canvas to have drained.
 */
const SEND_FAILURE_RETRY_MS = 500

/** How long a full-resolution capture waits for the re-rastered surface. */
const FULL_RESOLUTION_CAPTURE_TIMEOUT_MS = 1_000
const FULL_RESOLUTION_CAPTURE_POLL_MS = 30

export interface PageHostStats {
  pageId: string
  framesReceived: number
  popupFrames: number
  framesWithoutTexture: number
  framesDroppedForPoolPressure: number
  sendFailures: number
  /** Present on a snapshot from `pageHostStats`, not on the live counters. */
  painting?: boolean
  textureScale?: number
  textureTransitionInFlight?: boolean
  outstandingTextures: number
  maxOutstandingTextures: number
  releaseLatencyMs: number | null
}

export interface PageHost {
  readonly id: string
  readonly webContents: WebContents
  /** The layout pass's verdict: is anyone looking at this page? */
  readonly painting: boolean
  /** The idle policy's verdict: is anyone looking at the app? */
  readonly idle: boolean
  resize(size: { width: number; height: number }): void
  setPainting(painting: boolean): void
  /** Quiet the page while the app is idle; restore it in full on wake. */
  setIdle(idle: boolean): void
  /**
   * View px per CSS px: below 1 while the page paints a reduced texture
   * (page-texture-scale.ts). Input sent to the page is in view px.
   */
  readonly textureScale: number
  /**
   * The layout pass's report of how large the page shows on screen, as
   * screen px per CSS px. Grades the page's frame rate (page-frame-rate.ts)
   * and texture resolution (page-texture-scale.ts); pass 1 for a page that
   * must paint in full regardless of the camera (agent-driven, focus
   * session) — full scale applies at once rather than on camera settle.
   */
  setDisplayScale(scale: number): void
  /**
   * `capturePage` of the page at its full CSS viewport × device scale,
   * whatever texture scale the camera has it at. For callers that want the
   * page's own pixels (an agent's screenshot) rather than its on-screen
   * rendition — a capture that is only ever shown at canvas size can call
   * `webContents.capturePage()` and take the texture as it is.
   */
  captureFullResolution(): Promise<NativeImage>
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
  private isIdle = false
  /** The LOD verdict for this page's rate; painting resumes at it on wake. */
  private tierFrameRate: number
  /** The scale the view is sized to now. */
  private appliedTextureScale = FULL_TEXTURE_SCALE
  /** The LOD verdict for the view's scale, applied once the camera rests. */
  private wantedTextureScale = FULL_TEXTURE_SCALE
  private lastDisplayScale = 1
  private textureScaleTimer: NodeJS.Timeout | null = null
  private fullResolutionHolds = 0
  /** Whether a metrics override is installed on the page's debugger session. */
  private viewportOverridden = false
  /** Bumped per transition, so a superseded one abandons its later steps. */
  private textureScaleGeneration = 0
  private clearOverrideTimer: NodeJS.Timeout | null = null
  /**
   * While the view is changing scale its frames cannot be trusted: one can
   * show the document scaled into a corner of the old view, cropped by it, or
   * — the capturer's first copy of a resized surface — blank. They are
   * dropped and the canvas holds the last good frame. The first frame at the
   * new size proves the resize has landed (sixty views resizing at once take
   * their time); it is dropped too, and a clean one asked for in its place.
   */
  private textureScaleInFlight = false
  private textureNudgeTimer: NodeJS.Timeout | null = null
  private sendRetryTimer: NodeJS.Timeout | null = null
  private readonly deviceScaleFactor: number
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
    this.deviceScaleFactor = screen.getPrimaryDisplay().scaleFactor
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
          deviceScaleFactor: this.deviceScaleFactor,
        },
      },
    })
    // A page is a window only as an implementation detail of rendering it
    // offscreen, so nothing a document does may close it: `window.close()`
    // succeeds on a window with a single history entry (an OAuth "you can
    // close this now" page is the common one), which would destroy the host
    // out from under the `Page` record that owns it. Our own teardown goes
    // through `destroy()`, which closes without raising this event.
    this.win.on('close', (event) => event.preventDefault())
    this.tierFrameRate = this.win.webContents.getFrameRate()
    this.win.webContents.on('paint', (event) => {
      if (this.win.isDestroyed()) return
      const texture = (event as Electron.Event<Electron.WebContentsPaintEventParams>).texture
      if (!texture) {
        this.stats.framesWithoutTexture++
        return
      }
      this.deliver(texture)
    })
  }

  get webContents(): WebContents {
    return this.win.webContents
  }

  get painting(): boolean {
    return this.isPainting
  }

  get idle(): boolean {
    return this.isIdle
  }

  get textureScale(): number {
    return this.appliedTextureScale
  }

  textureState(): Pick<PageHostStats, 'painting' | 'textureScale' | 'textureTransitionInFlight'> {
    return {
      painting: this.isPainting && !this.isIdle,
      textureScale: this.appliedTextureScale,
      textureTransitionInFlight: this.textureScaleInFlight,
    }
  }

  resize(size: { width: number; height: number }): void {
    const width = Math.max(1, Math.round(size.width))
    const height = Math.max(1, Math.round(size.height))
    if (width === this.currentSize.width && height === this.currentSize.height) return
    if (this.win.isDestroyed()) return
    this.currentSize = { width, height }
    this.applyTextureScale()
  }

  setPainting(painting: boolean): void {
    if (painting === this.isPainting || this.win.isDestroyed()) return
    this.isPainting = painting
    // Nobody saw the page at its old scale, so there is no resize to hide
    // from them: it wakes at the scale it is owed. A zoom-out that wakes every
    // page at full size floods the canvas with full-size textures for the
    // length of the settle wait — enough, at sixty pages, to time out their
    // transfer (see the `sendSharedTexture` failure path in `deliver`).
    if (painting) this.reconcileTextureScale(true)
    this.applyPainting()
  }

  setIdle(idle: boolean): void {
    if (idle === this.isIdle || this.win.isDestroyed()) return
    this.isIdle = idle
    this.applyPainting()
  }

  setDisplayScale(scale: number): void {
    if (this.win.isDestroyed()) return
    // Only a camera that moved restarts the settle wait; the layout pass
    // runs for many reasons that leave the page where it was.
    if (scale !== this.lastDisplayScale) {
      this.lastDisplayScale = scale
      this.wantedTextureScale = textureScaleForDisplayScale(scale, this.appliedTextureScale)
      this.reconcileTextureScale()
    }
    const next = frameRateForDisplayScale(scale, this.tierFrameRate)
    if (next === this.tierFrameRate) return
    this.tierFrameRate = next
    this.applyPainting()
  }

  async captureFullResolution(): Promise<NativeImage> {
    const contents = this.win.webContents
    if (this.appliedTextureScale === FULL_TEXTURE_SCALE && this.fullResolutionHolds === 0) {
      return contents.capturePage()
    }
    this.fullResolutionHolds++
    this.reconcileTextureScale()
    try {
      // The view is full size at once, but the surface a capture copies
      // follows a re-raster later: until then a capture comes back at the old
      // width, or rejects outright (`UnknownVizError`) for racing the resize.
      const fullWidth = this.currentSize.width
      const deadline = performance.now() + FULL_RESOLUTION_CAPTURE_TIMEOUT_MS
      for (;;) {
        const lastTry = performance.now() >= deadline
        const image = await contents.capturePage().catch((error: unknown) => {
          if (lastTry) throw error
          return null
        })
        if (image && (image.getSize().width >= fullWidth || lastTry)) return image
        await new Promise((resolve) => setTimeout(resolve, FULL_RESOLUTION_CAPTURE_POLL_MS))
      }
    } finally {
      this.fullResolutionHolds--
      this.reconcileTextureScale()
    }
  }

  /**
   * Move the view toward the scale it is owed. Full size is never delayed
   * when something needs the page's real pixels — a capture's hold, a page
   * the layout pass pinned at scale 1 — and a page waking from a cull moves
   * `atOnce`; everything else waits for the camera to rest.
   */
  private reconcileTextureScale(atOnce = false): void {
    if (this.textureScaleTimer) clearTimeout(this.textureScaleTimer)
    this.textureScaleTimer = null
    if (this.win.isDestroyed()) return
    const mustBeFull = this.fullResolutionHolds > 0
    const target = mustBeFull ? FULL_TEXTURE_SCALE : this.wantedTextureScale
    if (target === this.appliedTextureScale) return
    if (mustBeFull || atOnce || this.lastDisplayScale >= 1) {
      this.appliedTextureScale = target
      this.applyTextureScale()
      return
    }
    const settleMs =
      target > this.appliedTextureScale ? TEXTURE_GROW_SETTLE_MS : TEXTURE_SHRINK_SETTLE_MS
    this.textureScaleTimer = setTimeout(() => {
      this.textureScaleTimer = null
      if (this.win.isDestroyed()) return
      this.appliedTextureScale = target
      this.applyTextureScale()
    }, settleMs)
  }

  /**
   * Size the view to the CSS viewport × the texture scale. Below full scale
   * a CDP metrics override lays the document out at the full CSS viewport and
   * scales it into the smaller view, so the page cannot tell — `innerWidth`,
   * media queries, and `devicePixelRatio` are unchanged. CDP rather than
   * `enableDeviceEmulation` because that is one shot at the current render
   * widget: a navigation replaces the widget, and the next document would lay
   * out against the shrunken view. The debugger session re-applies its
   * override before a new document's first layout.
   *
   * Order is what keeps the page from ever seeing the small view: the
   * override lands first (at the new scale, against the old view), then the
   * view resizes under it. Frames painted in between are dropped.
   */
  private applyTextureScale(): void {
    const generation = ++this.textureScaleGeneration
    if (this.clearOverrideTimer) clearTimeout(this.clearOverrideTimer)
    this.clearOverrideTimer = null
    const { width, height } = this.currentSize
    const scale = this.appliedTextureScale
    const contents = this.win.webContents
    const resizeView = () =>
      this.win.setContentSize(
        Math.max(1, Math.round(width * scale)),
        Math.max(1, Math.round(height * scale)),
      )
    if (scale === FULL_TEXTURE_SCALE && !this.viewportOverridden) {
      resizeView()
      return
    }
    const awaitResizedFrame = () => {
      this.textureScaleInFlight = true
      if (this.textureNudgeTimer) clearInterval(this.textureNudgeTimer)
      this.textureNudgeTimer = setInterval(() => this.requestFrame(), TEXTURE_RESIZE_NUDGE_MS)
    }
    const abandonToFullSize = () => {
      if (this.win.isDestroyed()) return
      this.viewportOverridden = false
      this.appliedTextureScale = FULL_TEXTURE_SCALE
      this.win.setContentSize(this.currentSize.width, this.currentSize.height)
      awaitResizedFrame()
    }
    // A detached session takes its override with it, leaving the document
    // facing the small view; full size is the only safe place without one.
    if (!ensurePageDebugger(contents, abandonToFullSize)) {
      abandonToFullSize()
      return
    }
    this.viewportOverridden = true
    awaitResizedFrame()
    contents.debugger
      .sendCommand('Emulation.setDeviceMetricsOverride', {
        width,
        height,
        // An offscreen window reports its own size as the screen's.
        screenWidth: width,
        screenHeight: height,
        deviceScaleFactor: 0,
        mobile: false,
        scale,
        dontSetVisibleSize: true,
      })
      .then(() => {
        if (generation !== this.textureScaleGeneration || this.win.isDestroyed()) return
        resizeView()
        if (scale !== FULL_TEXTURE_SCALE) return
        // The override is now an identity; drop it once the resize has
        // landed, so a full-size page is back on the plain path.
        this.clearOverrideTimer = setTimeout(() => {
          this.clearOverrideTimer = null
          if (generation !== this.textureScaleGeneration || this.win.isDestroyed()) return
          this.viewportOverridden = false
          contents.debugger.sendCommand('Emulation.clearDeviceMetricsOverride').catch(() => {})
        }, CLEAR_OVERRIDE_AFTER_RESIZE_MS)
      })
      .catch(() => {
        if (generation === this.textureScaleGeneration) abandonToFullSize()
      })
  }

  /** A popup's texture is its own size, not the viewport's, so it reports the host's. */
  private paintedCssSize(info: Electron.OffscreenSharedTexture['textureInfo']): {
    cssWidth: number
    cssHeight: number
  } {
    const { width, height } = this.currentSize
    if (info.widgetType === 'popup') return { cssWidth: width, cssHeight: height }
    const devicePxPerCssPx = this.appliedTextureScale * this.deviceScaleFactor
    return {
      cssWidth: paintedCssLength(info.codedSize.width, width, devicePxPerCssPx),
      cssHeight: paintedCssLength(info.codedSize.height, height, devicePxPerCssPx),
    }
  }

  private isAtAppliedTextureSize(codedWidth: number): boolean {
    const viewWidth = Math.max(1, Math.round(this.currentSize.width * this.appliedTextureScale))
    return (
      Math.abs(codedWidth - Math.round(viewWidth * this.deviceScaleFactor)) <=
      TEXTURE_SIZE_TOLERANCE_PX
    )
  }

  /**
   * Asks for the clean frame twice. Under load — every page on screen
   * resizing together — the forced frame can itself be a blank capture of a
   * surface still re-rastering, and a static page would paint nothing after
   * it; main cannot see into a texture to tell, so the second ask is the
   * guard.
   */
  private endTextureTransition(): void {
    this.textureScaleInFlight = false
    if (this.textureNudgeTimer) clearInterval(this.textureNudgeTimer)
    this.textureNudgeTimer = setTimeout(() => {
      this.textureNudgeTimer = null
      this.requestFrame()
    }, TEXTURE_CLEAN_FRAME_RETRY_MS)
    this.requestFrame()
  }

  /**
   * Send a fresh frame of an unchanged page, if it paints at all.
   * `invalidate()` alone can emit a paint with no shared texture for an
   * unchanged document; restarting capture forces a GPU frame.
   */
  requestFrame(): void {
    if (this.win.isDestroyed() || !this.isPainting || this.isIdle) return
    const contents = this.win.webContents
    contents.stopPainting()
    contents.startPainting()
    contents.invalidate()
  }

  /** A static page whose frame was lost in transfer has no next paint of its own. */
  private retryFrameSoon(): void {
    if (this.sendRetryTimer) return
    this.sendRetryTimer = setTimeout(() => {
      this.sendRetryTimer = null
      this.requestFrame()
    }, SEND_FAILURE_RETRY_MS)
  }

  /**
   * A page paints only when both someone is looking at it and at the app —
   * and only then does it earn its full frame rate. The two move together:
   * stopping delivery without dropping the rate leaves an animating page
   * compositing 60fps of discarded frames.
   */
  private applyPainting(): void {
    const contents = this.win.webContents
    const shouldPaint = this.isPainting && !this.isIdle
    contents.setFrameRate(shouldPaint ? this.tierFrameRate : IDLE_FRAME_RATE)
    if (shouldPaint === contents.isPainting()) return
    if (shouldPaint) {
      contents.startPainting()
      // A resumed page has nothing dirty; ask for one frame so it reappears.
      contents.invalidate()
    } else {
      contents.stopPainting()
    }
  }

  destroy(): void {
    if (this.textureScaleTimer) clearTimeout(this.textureScaleTimer)
    if (this.clearOverrideTimer) clearTimeout(this.clearOverrideTimer)
    if (this.textureNudgeTimer) clearInterval(this.textureNudgeTimer)
    if (this.sendRetryTimer) clearTimeout(this.sendRetryTimer)
    this.textureNudgeTimer = null
    this.sendRetryTimer = null
    this.textureScaleTimer = null
    this.clearOverrideTimer = null
    if (this.win.isDestroyed()) return
    this.win.destroy()
  }

  isDestroyed(): boolean {
    return this.win.isDestroyed()
  }

  private deliver(texture: Electron.OffscreenSharedTexture): void {
    const paintedAt = performance.now()
    const stats = this.stats
    const target = frameTarget
    if (!target || target.isDestroyed()) {
      texture.release()
      return
    }
    const info = texture.textureInfo
    // Ahead of the pool check: a transition's frames are never sent, so a
    // full pool must not keep the one that ends it from being seen.
    if (this.textureScaleInFlight && info.widgetType !== 'popup') {
      texture.release()
      if (this.isAtAppliedTextureSize(info.codedSize.width)) this.endTextureTransition()
      return
    }
    if (stats.outstandingTextures >= MAX_OUTSTANDING_TEXTURES) {
      stats.framesDroppedForPoolPressure++
      texture.release()
      return
    }
    stats.outstandingTextures++
    stats.maxOutstandingTextures = Math.max(stats.maxOutstandingTextures, stats.outstandingTextures)
    if (info.widgetType === 'popup') stats.popupFrames++
    else stats.framesReceived++

    const meta: PageFrameMeta = {
      pageId: this.id,
      widgetType: info.widgetType,
      width: info.codedSize.width,
      height: info.codedSize.height,
      ...this.paintedCssSize(info),
      frameRate: this.tierFrameRate,
      inputSeq: inputCountForPage(this.id),
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
        // A transfer the canvas never acknowledged (it times out after a
        // second when the canvas is swamped) never reports its references
        // released either. Left counted, a few of them fill this host's pool
        // allowance and every later frame is dropped for good.
        finish()
        this.retryFrameSoon()
      })
      .finally(() => {
        // Main's own reference goes now; the renderer's reference keeps the
        // texture alive until it has copied the frame, then `finish` runs.
        imported.release()
      })
  }
}

const hosts = new Set<OffscreenPageHost>()

/** Static pages may finish painting before the surface has mounted or seated its scene. */
export function requestPageFrames(sender: WebContents, pageIds: readonly string[]): void {
  if (sender !== frameTarget || sender.isDestroyed()) return
  const requested = new Set(pageIds)
  for (const host of hosts) {
    if (requested.has(host.id)) host.requestFrame()
  }
}

export function createPageHost(options: {
  id: string
  width: number
  height: number
}): PageHost {
  const host = new OffscreenPageHost(options)
  hosts.add(host)
  host.webContents.once('destroyed', () => {
    hosts.delete(host)
    forgetInputCountForPage(host.id)
  })
  return host
}

/**
 * Destroy every page host. Page hosts are top-level windows, so while any of
 * them lives Electron counts a window as open: `window-all-closed` never
 * fires, and a quit stalls on hosts that refuse to close. Called when the main
 * window goes away and again on `before-quit`, after the autosave flush that
 * still wants live pages.
 */
export function destroyAllPageHosts(): void {
  for (const host of [...hosts]) host.destroy()
}

export function pageHostStats(): PageHostStats[] {
  return [...hosts]
    .filter((host) => !host.isDestroyed())
    .map((host) => ({ ...host.stats, ...host.textureState() }))
}
