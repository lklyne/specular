/**
 * Offscreen-rendering lab — the types and pure math shared by the lab window's
 * main-process page host, its preload, and its renderer.
 *
 * The lab is a spike, not a product surface. It answers two questions from
 * docs/one-live-view-research.md: what does a canvas of offscreen pages cost
 * when every page is a GPU texture, and can the one interactive page be an
 * offscreen page too. Everything here is deliberately independent of the real
 * canvas runtime so the numbers it produces are about Electron, not Specular.
 */

import { CANVAS_MAX_ZOOM, CANVAS_MIN_ZOOM } from './zoom'

export type OsrLabMode = 'shared-texture' | 'bitmap-jpeg'
export type OsrLabInputTransport = 'send-input-event' | 'cdp'

export interface OsrLabConfig {
  mode: OsrLabMode
  urls: string[]
  /** CSS size of every page. The texture is this × deviceScaleFactor. */
  pageWidth: number
  pageHeight: number
  deviceScaleFactor: number
  /** Paint events per second per page; 0 leaves the OSR frame rate uncapped. */
  frameRateCap: number
  pointerTransport: OsrLabInputTransport
  keyboardTransport: OsrLabInputTransport
  /** `stopPainting()` pages whose rect is outside the viewport. */
  stopPaintingOffscreen: boolean
}

export const DEFAULT_OSR_LAB_URLS: readonly string[] = [
  'https://en.wikipedia.org/wiki/Compositing_window_manager',
  'https://developer.mozilla.org/en-US/docs/Web/API/VideoFrame',
  'https://www.electronjs.org/docs/latest/tutorial/offscreen-rendering',
  'https://news.ycombinator.com',
  'https://tldraw.com',
  'https://github.com/electron/electron',
  'https://caniuse.com/?search=videoframe',
  'https://www.wikipedia.org',
  'https://example.com',
]

export const DEFAULT_OSR_LAB_CONFIG: OsrLabConfig = {
  mode: 'shared-texture',
  urls: [...DEFAULT_OSR_LAB_URLS],
  pageWidth: 1280,
  pageHeight: 800,
  deviceScaleFactor: 2,
  frameRateCap: 0,
  pointerTransport: 'send-input-event',
  keyboardTransport: 'cdp',
  stopPaintingOffscreen: true,
}

export interface OsrLabPageInfo {
  id: string
  index: number
  url: string
  title: string
  loading: boolean
  /** OS pid of the page's renderer, once it has one. */
  pid: number | null
}

export interface OsrLabPageStats {
  pageId: string
  framesReceived: number
  popupFrames: number
  /** Paint events that carried no texture (GPU unavailable, or bitmap mode). */
  framesWithoutTexture: number
  /** Textures released without being sent because too many were in flight. */
  framesDroppedForPoolPressure: number
  sendFailures: number
  lastFrameAt: number | null
  lastTextureWidth: number
  lastTextureHeight: number
  /** Main-thread JPEG encode of the last frame (bitmap mode only). */
  lastEncodeMs: number | null
  painting: boolean
  outstandingTextures: number
  maxOutstandingTextures: number
  /** Mean paint → allReferencesReleased over the last frames, ms. */
  releaseLatencyMs: number | null
  cursorChanges: number
  lastCursor: string | null
}

export interface OsrLabMainStats {
  sampledAt: number
  pages: OsrLabPageStats[]
  gpuWorkingSetMb: number | null
  pageRenderersWorkingSetMb: number
  /** How many pages hold a CDP debugger session right now. */
  cdpAttached: number
}

/** Metadata that travels with every frame the renderer receives. */
export interface OsrLabFrameMeta {
  pageId: string
  widgetType: 'frame' | 'popup'
  width: number
  height: number
  /** `performance.timeOrigin`-relative ms in main when the paint event fired. */
  paintedAt: number
  frameCount: number | null
}

/**
 * What the lab preload posts to the page world for every frame, with the
 * ImageBitmap attached as a transferred `bitmap` property.
 */
export interface OsrLabFrameMessage {
  source: 'osr-lab'
  kind: 'frame'
  meta: OsrLabFrameMeta
  /** Time the preload spent turning the texture into an ImageBitmap, ms. */
  copyMs: number
  receivedAt: number
}

export interface OsrLabPointerPayload {
  pageId: string
  kind: 'down' | 'up' | 'move'
  /** Page-local CSS coordinates. */
  x: number
  y: number
  button: 'left' | 'middle' | 'right'
  buttons: number
  clickCount: number
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}

export interface OsrLabWheelPayload {
  pageId: string
  x: number
  y: number
  deltaX: number
  deltaY: number
  hasPreciseScrollingDeltas: boolean
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}

export interface OsrLabKeyPayload {
  pageId: string
  kind: 'down' | 'up'
  key: string
  code: string
  /** Printable text for this key press, when there is any. */
  text: string | null
  repeat: boolean
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}

export interface OsrLabCursorPayload {
  pageId: string
  type: string
}

export type OsrLabBenchmarkPhaseId = 'slow-pan' | 'slow-zoom' | 'fast-pan' | 'pan-zoom'

export interface OsrLabBenchmarkPhaseResult {
  phase: OsrLabBenchmarkPhaseId
  durationMs: number
  draws: number
  drawFps: number
  /** Mean and worst rAF-to-rAF interval while drawing, ms. */
  meanFrameMs: number
  maxFrameMs: number
  /** rAF intervals over 1.5× the display's refresh interval. */
  longFrames: number
  framesReceived: number
}

export interface OsrLabBenchmarkResult {
  config: OsrLabConfig
  pageCount: number
  refreshMs: number
  phases: OsrLabBenchmarkPhaseResult[]
  tracePath: string | null
  mainStats: OsrLabMainStats | null
}

// --- Camera -----------------------------------------------------------------

export interface LabCamera {
  x: number
  y: number
  zoom: number
}

export interface LabRect {
  x: number
  y: number
  width: number
  height: number
}

export function screenToCanvas(camera: LabCamera, sx: number, sy: number): { x: number; y: number } {
  return { x: (sx - camera.x) / camera.zoom, y: (sy - camera.y) / camera.zoom }
}

export function canvasToScreen(camera: LabCamera, cx: number, cy: number): { x: number; y: number } {
  return { x: cx * camera.zoom + camera.x, y: cy * camera.zoom + camera.y }
}

export function projectRect(camera: LabCamera, rect: LabRect): LabRect {
  const origin = canvasToScreen(camera, rect.x, rect.y)
  return {
    x: origin.x,
    y: origin.y,
    width: rect.width * camera.zoom,
    height: rect.height * camera.zoom,
  }
}

export function clampLabZoom(zoom: number): number {
  return Math.max(CANVAS_MIN_ZOOM, Math.min(CANVAS_MAX_ZOOM, zoom))
}

/**
 * Zooms about a screen point, so the canvas point under the pointer stays
 * put. Same sensitivity as the app's wheel zoom (`viewport-input.ts`).
 */
export function zoomCameraAt(camera: LabCamera, deltaY: number, sx: number, sy: number): LabCamera {
  const nextZoom = clampLabZoom(camera.zoom - deltaY * 0.002)
  if (nextZoom === camera.zoom) return camera
  const anchor = screenToCanvas(camera, sx, sy)
  return {
    zoom: nextZoom,
    x: sx - anchor.x * nextZoom,
    y: sy - anchor.y * nextZoom,
  }
}

export function panCamera(camera: LabCamera, dx: number, dy: number): LabCamera {
  return { ...camera, x: camera.x + dx, y: camera.y + dy }
}

/** Camera that fits `bounds` into a viewport with a margin, centered. */
export function fitCamera(
  bounds: LabRect,
  viewport: { width: number; height: number },
  margin = 40,
): LabCamera {
  const availableW = Math.max(1, viewport.width - margin * 2)
  const availableH = Math.max(1, viewport.height - margin * 2)
  const zoom = clampLabZoom(Math.min(availableW / bounds.width, availableH / bounds.height))
  return {
    zoom,
    x: (viewport.width - bounds.width * zoom) / 2 - bounds.x * zoom,
    y: (viewport.height - bounds.height * zoom) / 2 - bounds.y * zoom,
  }
}

// --- Layout -----------------------------------------------------------------

export const LAB_PAGE_GAP = 80

/** Pages in a near-square grid, reading order, in canvas coordinates. */
export function layoutPages(
  count: number,
  pageWidth: number,
  pageHeight: number,
  gap = LAB_PAGE_GAP,
): LabRect[] {
  const columns = Math.max(1, Math.ceil(Math.sqrt(count)))
  const rects: LabRect[] = []
  for (let i = 0; i < count; i++) {
    const column = i % columns
    const row = Math.floor(i / columns)
    rects.push({
      x: column * (pageWidth + gap),
      y: row * (pageHeight + gap),
      width: pageWidth,
      height: pageHeight,
    })
  }
  return rects
}

export function boundsOf(rects: readonly LabRect[]): LabRect {
  if (rects.length === 0) return { x: 0, y: 0, width: 1, height: 1 }
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const rect of rects) {
    minX = Math.min(minX, rect.x)
    minY = Math.min(minY, rect.y)
    maxX = Math.max(maxX, rect.x + rect.width)
    maxY = Math.max(maxY, rect.y + rect.height)
  }
  return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
}

export function rectsIntersect(a: LabRect, b: LabRect): boolean {
  return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

/** Index of the topmost page under a screen point, or -1. Later pages win. */
export function hitTestPages(camera: LabCamera, rects: readonly LabRect[], sx: number, sy: number): number {
  const point = screenToCanvas(camera, sx, sy)
  for (let i = rects.length - 1; i >= 0; i--) {
    const rect = rects[i]
    if (
      point.x >= rect.x &&
      point.x < rect.x + rect.width &&
      point.y >= rect.y &&
      point.y < rect.y + rect.height
    ) {
      return i
    }
  }
  return -1
}

/** Ids of pages whose projected rect overlaps the viewport. */
export function visiblePageIds(
  camera: LabCamera,
  pages: readonly { id: string; rect: LabRect }[],
  viewport: { width: number; height: number },
): string[] {
  const view = { x: 0, y: 0, width: viewport.width, height: viewport.height }
  return pages.filter((page) => rectsIntersect(projectRect(camera, page.rect), view)).map((page) => page.id)
}

// --- Stats ------------------------------------------------------------------

/** Summarizes rAF intervals from one benchmark phase. */
export function summarizeFrameIntervals(
  intervalsMs: readonly number[],
  refreshMs: number,
): Pick<OsrLabBenchmarkPhaseResult, 'draws' | 'drawFps' | 'meanFrameMs' | 'maxFrameMs' | 'longFrames'> {
  if (intervalsMs.length === 0) {
    return { draws: 0, drawFps: 0, meanFrameMs: 0, maxFrameMs: 0, longFrames: 0 }
  }
  let total = 0
  let max = 0
  let long = 0
  const longThreshold = refreshMs * 1.5
  for (const interval of intervalsMs) {
    total += interval
    if (interval > max) max = interval
    if (interval > longThreshold) long++
  }
  return {
    draws: intervalsMs.length,
    drawFps: total > 0 ? (intervalsMs.length / total) * 1000 : 0,
    meanFrameMs: total / intervalsMs.length,
    maxFrameMs: max,
    longFrames: long,
  }
}

/** Mean of a bounded window of recent samples, or null when empty. */
export function pushBounded(samples: number[], value: number, limit = 60): void {
  samples.push(value)
  if (samples.length > limit) samples.splice(0, samples.length - limit)
}

export function meanOf(samples: readonly number[]): number | null {
  if (samples.length === 0) return null
  let total = 0
  for (const sample of samples) total += sample
  return total / samples.length
}

/** Electron `KeyboardInputEvent.keyCode` for a DOM `key`, or null when unmapped. */
export function electronKeyCodeFor(key: string): string | null {
  if (key.length === 1) return key
  const named: Record<string, string> = {
    Enter: 'Return',
    Backspace: 'Backspace',
    Tab: 'Tab',
    Escape: 'Escape',
    Delete: 'Delete',
    ArrowLeft: 'Left',
    ArrowRight: 'Right',
    ArrowUp: 'Up',
    ArrowDown: 'Down',
    Home: 'Home',
    End: 'End',
    PageUp: 'PageUp',
    PageDown: 'PageDown',
    Shift: 'Shift',
    Control: 'Control',
    Alt: 'Alt',
    Meta: 'Meta',
    ' ': 'Space',
  }
  return named[key] ?? null
}

/** Windows virtual key code for the keys CDP needs one for. */
export function windowsVirtualKeyCodeFor(key: string): number | null {
  const named: Record<string, number> = {
    Backspace: 8,
    Tab: 9,
    Enter: 13,
    Shift: 16,
    Control: 17,
    Alt: 18,
    Escape: 27,
    ' ': 32,
    PageUp: 33,
    PageDown: 34,
    End: 35,
    Home: 36,
    ArrowLeft: 37,
    ArrowUp: 38,
    ArrowRight: 39,
    ArrowDown: 40,
    Delete: 46,
    Meta: 91,
  }
  if (key in named) return named[key]
  if (key.length === 1) {
    const upper = key.toUpperCase()
    const code = upper.charCodeAt(0)
    if ((code >= 48 && code <= 57) || (code >= 65 && code <= 90)) return code
  }
  return null
}

/** CDP `Input.*` modifier bitmask: Alt=1, Ctrl=2, Meta=4, Shift=8. */
export function cdpModifiersFor(mods: {
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}): number {
  return (mods.altKey ? 1 : 0) | (mods.ctrlKey ? 2 : 0) | (mods.metaKey ? 4 : 0) | (mods.shiftKey ? 8 : 0)
}

export function electronModifiersFor(mods: {
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}): Array<'shift' | 'control' | 'alt' | 'meta'> {
  const out: Array<'shift' | 'control' | 'alt' | 'meta'> = []
  if (mods.shiftKey) out.push('shift')
  if (mods.ctrlKey) out.push('control')
  if (mods.altKey) out.push('alt')
  if (mods.metaKey) out.push('meta')
  return out
}
