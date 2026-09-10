/**
 * The lab's page host: one hidden offscreen `BrowserWindow` per URL, its
 * `paint` events forwarded to the lab renderer as GPU textures (shared-texture
 * mode) or JPEG bytes (bitmap mode), and the input the renderer forwards
 * dispatched back into the page.
 *
 * Uses `BrowserWindow` rather than `WebContentsView` because an offscreen
 * WebContentsView paints at its parent window's size (electron#45864).
 */

import { app, BrowserWindow, sharedTexture, type WebContents } from 'electron'
import type {
  OsrLabConfig,
  OsrLabFrameMeta,
  OsrLabKeyPayload,
  OsrLabMainStats,
  OsrLabPageInfo,
  OsrLabPageStats,
  OsrLabPointerPayload,
  OsrLabWheelPayload,
} from '../../shared/osr-lab'
import {
  cdpModifiersFor,
  electronKeyCodeFor,
  electronModifiersFor,
  meanOf,
  pushBounded,
  windowsVirtualKeyCodeFor,
} from '../../shared/osr-lab'
import { ipcChannels } from '../../shared/ipc-contract'

/** Chromium's OSR frame pool holds 10 textures per page; keep headroom. */
const MAX_OUTSTANDING_TEXTURES = 6

interface LabPage {
  id: string
  index: number
  url: string
  win: BrowserWindow
  stats: OsrLabPageStats
  releaseLatencies: number[]
  cdpAttached: boolean
  painting: boolean
}

let config: OsrLabConfig | null = null
let pages: LabPage[] = []
let target: WebContents | null = null
let enteredPageId: string | null = null

function wc(page: LabPage): WebContents {
  return page.win.webContents
}

function findPage(pageId: string): LabPage | null {
  const page = pages.find((candidate) => candidate.id === pageId) ?? null
  if (!page || page.win.isDestroyed()) return null
  return page
}

function pageInfo(page: LabPage): OsrLabPageInfo {
  const contents = wc(page)
  let pid: number | null = null
  try {
    pid = contents.getOSProcessId() || null
  } catch {
    pid = null
  }
  return {
    id: page.id,
    index: page.index,
    url: page.url,
    title: contents.isDestroyed() ? '' : contents.getTitle(),
    loading: contents.isDestroyed() ? false : contents.isLoading(),
    pid,
  }
}

function broadcastPages(): void {
  if (!target || target.isDestroyed()) return
  target.send(
    ipcChannels.osrLabPagesChanged,
    pages.filter((page) => !page.win.isDestroyed()).map(pageInfo),
  )
}

function emptyStats(pageId: string): OsrLabPageStats {
  return {
    pageId,
    framesReceived: 0,
    popupFrames: 0,
    framesWithoutTexture: 0,
    framesDroppedForPoolPressure: 0,
    sendFailures: 0,
    lastFrameAt: null,
    lastTextureWidth: 0,
    lastTextureHeight: 0,
    lastEncodeMs: null,
    painting: true,
    outstandingTextures: 0,
    maxOutstandingTextures: 0,
    releaseLatencyMs: null,
    cursorChanges: 0,
    lastCursor: null,
  }
}

// --- Frame delivery ---------------------------------------------------------

function handleSharedTexturePaint(
  page: LabPage,
  texture: Electron.OffscreenSharedTexture,
): void {
  const paintedAt = performance.now()
  const stats = page.stats
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
  stats.lastTextureWidth = info.codedSize.width
  stats.lastTextureHeight = info.codedSize.height
  if (info.widgetType === 'popup') stats.popupFrames++
  else stats.framesReceived++
  stats.lastFrameAt = Date.now()

  const meta: OsrLabFrameMeta = {
    pageId: page.id,
    widgetType: info.widgetType,
    width: info.codedSize.width,
    height: info.codedSize.height,
    paintedAt,
    frameCount: info.metadata.frameCount ?? null,
  }

  let released = false
  const finish = () => {
    if (released) return
    released = true
    stats.outstandingTextures = Math.max(0, stats.outstandingTextures - 1)
    pushBounded(page.releaseLatencies, performance.now() - paintedAt)
    stats.releaseLatencyMs = meanOf(page.releaseLatencies)
    texture.release()
  }

  let imported: Electron.SharedTextureImported
  try {
    imported = sharedTexture.importSharedTexture({
      textureInfo: info,
      allReferencesReleased: finish,
    })
  } catch (error) {
    console.error('[osr-lab] importSharedTexture failed', error)
    stats.sendFailures++
    finish()
    return
  }

  void sharedTexture
    .sendSharedTexture({ frame: target.mainFrame, importedSharedTexture: imported }, meta)
    .catch((error: unknown) => {
      stats.sendFailures++
      console.error('[osr-lab] sendSharedTexture failed', error)
    })
    .finally(() => {
      // Main's own reference goes now; the renderer's reference keeps the
      // texture alive until it has copied the frame, then `finish` runs.
      imported.release()
    })
}

function handleBitmapPaint(page: LabPage, image: Electron.NativeImage): void {
  const paintedAt = performance.now()
  const stats = page.stats
  if (!target || target.isDestroyed()) return
  const size = image.getSize()
  const encodeStart = performance.now()
  const jpeg = image.toJPEG(80)
  stats.lastEncodeMs = performance.now() - encodeStart
  stats.framesReceived++
  stats.lastFrameAt = Date.now()
  stats.lastTextureWidth = size.width
  stats.lastTextureHeight = size.height
  const meta: OsrLabFrameMeta = {
    pageId: page.id,
    widgetType: 'frame',
    width: size.width,
    height: size.height,
    paintedAt,
    frameCount: null,
  }
  target.send(ipcChannels.osrLabFrameJpeg, { meta, jpeg: new Uint8Array(jpeg) })
}

// --- Lifecycle --------------------------------------------------------------

function createPage(index: number, url: string, labConfig: OsrLabConfig): LabPage {
  const shared = labConfig.mode === 'shared-texture'
  const win = new BrowserWindow({
    show: false,
    width: labConfig.pageWidth,
    height: labConfig.pageHeight,
    webPreferences: {
      offscreen: {
        useSharedTexture: shared,
        deviceScaleFactor: labConfig.deviceScaleFactor,
      },
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      // An offscreen page is never "visible" to Chromium; without this it
      // would idle at background cadence the moment it loads.
      backgroundThrottling: false,
    },
  })
  const page: LabPage = {
    id: `lab-${index}`,
    index,
    url,
    win,
    stats: emptyStats(`lab-${index}`),
    releaseLatencies: [],
    cdpAttached: false,
    painting: true,
  }
  const contents = win.webContents
  if (labConfig.frameRateCap > 0) contents.setFrameRate(labConfig.frameRateCap)

  contents.on('paint', (event, _dirty, image) => {
    if (win.isDestroyed()) return
    const texture = (event as Electron.Event<Electron.WebContentsPaintEventParams>).texture
    if (shared) {
      if (texture) handleSharedTexturePaint(page, texture)
      else page.stats.framesWithoutTexture++
      return
    }
    handleBitmapPaint(page, image)
  })
  contents.on('cursor-changed', (_event, type) => {
    page.stats.cursorChanges++
    page.stats.lastCursor = type
    if (target && !target.isDestroyed()) {
      target.send(ipcChannels.osrLabCursorChanged, { pageId: page.id, type })
    }
  })
  contents.on('page-title-updated', broadcastPages)
  contents.on('did-start-loading', broadcastPages)
  contents.on('did-stop-loading', broadcastPages)
  contents.on('did-finish-load', broadcastPages)
  contents.setWindowOpenHandler(() => ({ action: 'deny' }))
  void contents.loadURL(url).catch((error: unknown) => {
    console.error('[osr-lab] load failed', url, error)
  })
  return page
}

function destroyPage(page: LabPage): void {
  if (page.win.isDestroyed()) return
  if (page.cdpAttached) {
    try {
      wc(page).debugger.detach()
    } catch {
      // Already detached.
    }
  }
  page.win.destroy()
}

export function attachOsrLabTarget(contents: WebContents | null): void {
  target = contents
}

export async function configureOsrLab(next: OsrLabConfig): Promise<OsrLabPageInfo[]> {
  teardownOsrLab()
  config = next
  pages = next.urls.map((url, index) => createPage(index, url, next))
  broadcastPages()
  return pages.map(pageInfo)
}

export function teardownOsrLab(): void {
  for (const page of pages) destroyPage(page)
  pages = []
  enteredPageId = null
  config = null
}

export function osrLabPages(): OsrLabPageInfo[] {
  return pages.filter((page) => !page.win.isDestroyed()).map(pageInfo)
}

// --- Stats ------------------------------------------------------------------

export function sampleOsrLabStats(): OsrLabMainStats {
  const pids = new Set<number>()
  for (const page of pages) {
    const info = pageInfo(page)
    if (info.pid) pids.add(info.pid)
  }
  let gpuMb: number | null = null
  let renderersMb = 0
  for (const metric of app.getAppMetrics()) {
    const mb = metric.memory.workingSetSize / 1024
    if (metric.type === 'GPU') gpuMb = mb
    else if (pids.has(metric.pid)) renderersMb += mb
  }
  return {
    sampledAt: Date.now(),
    pages: pages.map((page) => ({ ...page.stats, painting: page.painting })),
    gpuWorkingSetMb: gpuMb,
    pageRenderersWorkingSetMb: renderersMb,
    cdpAttached: pages.filter((page) => page.cdpAttached).length,
  }
}

export async function captureOsrLabPage(pageId: string): Promise<string | null> {
  const page = findPage(pageId)
  if (!page) return null
  const image = await wc(page).capturePage()
  if (image.isEmpty()) return null
  return image.resize({ width: 320 }).toDataURL()
}

export function openOsrLabDevTools(pageId: string): void {
  const page = findPage(pageId)
  if (!page) return
  wc(page).openDevTools({ mode: 'detach' })
}

// --- Painting policy --------------------------------------------------------

export function setOsrLabVisiblePages(pageIds: string[]): void {
  if (!config?.stopPaintingOffscreen) {
    for (const page of pages) resumePainting(page)
    return
  }
  const visible = new Set(pageIds)
  for (const page of pages) {
    if (visible.has(page.id) || page.id === enteredPageId) resumePainting(page)
    else pausePainting(page)
  }
}

function resumePainting(page: LabPage): void {
  if (page.painting || page.win.isDestroyed()) return
  page.painting = true
  wc(page).startPainting()
  // A resumed page has nothing dirty; ask for one frame so it reappears.
  wc(page).invalidate()
}

function pausePainting(page: LabPage): void {
  if (!page.painting || page.win.isDestroyed()) return
  page.painting = false
  wc(page).stopPainting()
}

// --- Input ------------------------------------------------------------------

function ensureCdp(page: LabPage): boolean {
  const contents = wc(page)
  if (contents.debugger.isAttached()) {
    page.cdpAttached = true
    return true
  }
  try {
    contents.debugger.attach('1.3')
    page.cdpAttached = true
    contents.debugger.once('detach', () => {
      page.cdpAttached = false
    })
    return true
  } catch (error) {
    console.error('[osr-lab] debugger attach failed', error)
    return false
  }
}

function cdp(page: LabPage, method: string, params: Record<string, unknown>): void {
  if (!ensureCdp(page)) return
  wc(page)
    .debugger.sendCommand(method, params)
    .catch((error: unknown) => console.error(`[osr-lab] ${method} failed`, error))
}

export function setOsrLabEnteredPage(pageId: string | null): void {
  const previous = enteredPageId ? findPage(enteredPageId) : null
  enteredPageId = pageId
  if (previous && previous.id !== pageId) {
    cdp(previous, 'Emulation.setFocusEmulationEnabled', { enabled: false })
  }
  const page = pageId ? findPage(pageId) : null
  if (!page) return
  // The OSR widget host's Focus() is a no-op, so the page would otherwise
  // never believe it has focus: no caret, inactive selection color.
  cdp(page, 'Emulation.setFocusEmulationEnabled', { enabled: true })
  wc(page).focus()
  resumePainting(page)
}

export function forwardOsrLabPointer(payload: OsrLabPointerPayload): void {
  const page = findPage(payload.pageId)
  if (!page || !config) return
  const x = Math.round(payload.x)
  const y = Math.round(payload.y)
  if (config.pointerTransport === 'cdp') {
    const moveButton =
      payload.buttons & 1 ? 'left' : payload.buttons & 2 ? 'right' : payload.buttons & 4 ? 'middle' : 'none'
    cdp(page, 'Input.dispatchMouseEvent', {
      type: payload.kind === 'down' ? 'mousePressed' : payload.kind === 'up' ? 'mouseReleased' : 'mouseMoved',
      x,
      y,
      button: payload.kind === 'move' ? moveButton : payload.button,
      buttons: payload.buttons,
      clickCount: payload.kind === 'move' ? 0 : payload.clickCount,
      modifiers: cdpModifiersFor(payload),
    })
    return
  }
  const modifiers = electronModifiersFor(payload)
  if (payload.buttons & 1) modifiers.push('leftbuttondown' as never)
  wc(page).sendInputEvent({
    type: payload.kind === 'down' ? 'mouseDown' : payload.kind === 'up' ? 'mouseUp' : 'mouseMove',
    x,
    y,
    button: payload.button,
    clickCount: payload.kind === 'move' ? 0 : payload.clickCount,
    modifiers,
  })
}

export function forwardOsrLabWheel(payload: OsrLabWheelPayload): void {
  const page = findPage(payload.pageId)
  if (!page || !config) return
  const x = Math.round(payload.x)
  const y = Math.round(payload.y)
  if (config.pointerTransport === 'cdp') {
    cdp(page, 'Input.dispatchMouseEvent', {
      type: 'mouseWheel',
      x,
      y,
      deltaX: payload.deltaX,
      deltaY: payload.deltaY,
      modifiers: cdpModifiersFor(payload),
    })
    return
  }
  wc(page).sendInputEvent({
    type: 'mouseWheel',
    x,
    y,
    deltaX: -payload.deltaX,
    deltaY: -payload.deltaY,
    wheelTicksX: payload.hasPreciseScrollingDeltas ? 0 : -payload.deltaX,
    wheelTicksY: payload.hasPreciseScrollingDeltas ? 0 : -payload.deltaY,
    hasPreciseScrollingDeltas: payload.hasPreciseScrollingDeltas,
    canScroll: true,
    modifiers: electronModifiersFor(payload),
  })
}

export function forwardOsrLabKey(payload: OsrLabKeyPayload): void {
  const page = findPage(payload.pageId)
  if (!page || !config) return
  if (config.keyboardTransport === 'cdp') {
    const vk = windowsVirtualKeyCodeFor(payload.key)
    const base: Record<string, unknown> = {
      key: payload.key,
      code: payload.code,
      modifiers: cdpModifiersFor(payload),
      autoRepeat: payload.repeat,
      ...(vk !== null ? { windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk } : {}),
    }
    if (payload.kind === 'down') {
      cdp(page, 'Input.dispatchKeyEvent', {
        ...base,
        type: payload.text ? 'keyDown' : 'rawKeyDown',
        ...(payload.text ? { text: payload.text, unmodifiedText: payload.text } : {}),
      })
    } else {
      cdp(page, 'Input.dispatchKeyEvent', { ...base, type: 'keyUp' })
    }
    return
  }
  const keyCode = electronKeyCodeFor(payload.key)
  if (!keyCode) return
  const modifiers = electronModifiersFor(payload)
  const contents = wc(page)
  if (payload.kind === 'down') {
    contents.sendInputEvent({ type: 'keyDown', keyCode, modifiers })
    if (payload.text) contents.sendInputEvent({ type: 'char', keyCode: payload.text, modifiers })
  } else {
    contents.sendInputEvent({ type: 'keyUp', keyCode, modifiers })
  }
}

/** IME commits arrive as whole strings; `Input.insertText` is the only lever. */
export function insertOsrLabText(pageId: string, text: string): void {
  const page = findPage(pageId)
  if (!page || !text) return
  cdp(page, 'Input.insertText', { text })
}
