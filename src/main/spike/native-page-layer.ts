/**
 * Throwaway measurement spike (ADR 0038 territory, not itself an ADR): gives
 * a page's IOSurface straight to a native Core Animation layer instead of
 * routing it through canvas-bg. Everything here is gated on
 * `nativePagesEnabled` so the default path is unaffected, and the whole file
 * is meant to be deleted once the arm has been measured.
 *
 * The addon (`spike/native-page-layer/`) is built separately and may not
 * exist on disk — it is only ever loaded lazily, at runtime, behind the flag.
 */

import { createRequire } from 'node:module'
import { join } from 'node:path'
import { app, type BaseWindow } from 'electron'

export const nativePagesEnabled =
  process.env.SPECULAR_SPIKE_NATIVE_PAGES === '1' && process.platform === 'darwin'

interface NativePageLayerAddon {
  attach(contentViewHandle: Buffer, insertIndex: number): string[]
  present(pageId: string, ioSurface: Buffer): void
  setRects(pageIds: string[], rects: number[]): void
  remove(pageId: string): void
  stats(): { presents: number; layers: number; rectUpdates: number }
  detach(): void
}

/**
 * Core Animation keeps sampling a surface for a moment after `present()`
 * swaps it for the next one, and Electron's OSR pool reuses a released
 * texture immediately (6 slots/page, `MAX_OUTSTANDING_TEXTURES`) — so the
 * previous frame's release is delayed past that window rather than dropped
 * the instant it is superseded.
 */
const HELD_FRAME_RELEASE_DELAY_MS = 34

let addon: NativePageLayerAddon | null = null
let sessionDisabled = false
let attached = false
const loggedKinds = new Set<string>()

const heldFrames = new Map<string, { finish: () => void }>()
let errorCount = 0
let lastRectsPayload: string | null = null

function logOnce(kind: string, message: string, error: unknown): void {
  if (loggedKinds.has(kind)) return
  loggedKinds.add(kind)
  console.error(`[native-pages] ${message}`, error)
}

function disableSession(message: string, error: unknown): void {
  sessionDisabled = true
  logOnce('disable', message, error)
}

function getAddon(): NativePageLayerAddon | null {
  if (sessionDisabled) return null
  if (addon) return addon
  try {
    const requireFromAppPath = createRequire(join(app.getAppPath(), 'package.json'))
    addon = requireFromAppPath(
      join(app.getAppPath(), 'spike/native-page-layer/native_page_layer.node'),
    ) as NativePageLayerAddon
    return addon
  } catch (error) {
    disableSession('failed to load addon', error)
    return null
  }
}

/** Whether frames should currently be routed to the native layer at all. */
export function nativePagesActive(): boolean {
  return nativePagesEnabled && !sessionDisabled
}

/** Insert the addon's host view into the window. Idempotent. */
export function attachNativePageLayer(win: BaseWindow): void {
  if (!nativePagesEnabled || attached) return
  const instance = getAddon()
  if (!instance) return
  try {
    const insertIndex = Number(process.env.SPECULAR_SPIKE_NATIVE_INDEX ?? 1)
    const subviews = instance.attach(win.getNativeWindowHandle(), insertIndex)
    attached = true
    for (const line of subviews) console.log(`[native-pages] ${line}`)
  } catch (error) {
    disableSession('attach failed', error)
  }
}

/**
 * The texture-lifetime owner for a page's native frame. Presents the surface
 * and holds exactly one frame's `finish()` per page at rest, releasing the
 * previous one only after the delay above.
 */
export function presentNativePageFrame(
  pageId: string,
  texture: Electron.OffscreenSharedTexture,
  finish: () => void,
): void {
  const instance = getAddon()
  const ioSurface = texture.textureInfo.handle.ioSurface
  if (!instance || !ioSurface) {
    errorCount++
    finish()
    return
  }
  try {
    instance.present(pageId, ioSurface)
  } catch (error) {
    logOnce('present', 'present failed', error)
    errorCount++
    finish()
    return
  }
  const previous = heldFrames.get(pageId)
  heldFrames.set(pageId, { finish })
  if (previous) setTimeout(() => previous.finish(), HELD_FRAME_RELEASE_DELAY_MS)
}

export function removeNativePage(pageId: string): void {
  const instance = getAddon()
  if (instance) {
    try {
      instance.remove(pageId)
    } catch (error) {
      logOnce('remove', 'remove failed', error)
    }
  }
  const held = heldFrames.get(pageId)
  heldFrames.delete(pageId)
  held?.finish()
}

export interface NativePageRectEntry {
  pageId: string
  x: number
  y: number
  width: number
  height: number
  visible: boolean
}

export function setNativePageRects(entries: readonly NativePageRectEntry[]): void {
  const instance = getAddon()
  if (!instance) return
  const pageIds: string[] = []
  const rects: number[] = []
  for (const entry of entries) {
    pageIds.push(entry.pageId)
    // The contract hides a page by w <= 0 rather than by omitting it, so a
    // presented-but-off-screen page keeps its z-order slot in the layer list.
    rects.push(entry.x, entry.y, entry.visible ? entry.width : 0, entry.height)
  }
  // Passes run before the layer attaches; recording a payload the addon never
  // took would dedupe every later pass while the camera sits still.
  if (!attached) return
  const payload = pageIds.join('\u0000') + '|' + rects.join(',')
  if (payload === lastRectsPayload) return
  try {
    instance.setRects(pageIds, rects)
    lastRectsPayload = payload
  } catch (error) {
    logOnce('setRects', 'setRects failed', error)
  }
}

export function nativePageLayerStats(): {
  held: number
  errors: number
  enabled: boolean
  presents?: number
  layers?: number
  rectUpdates?: number
} {
  const base = { held: heldFrames.size, errors: errorCount, enabled: nativePagesEnabled }
  if (!addon) return base
  try {
    return { ...addon.stats(), ...base }
  } catch (error) {
    logOnce('stats', 'stats failed', error)
    return base
  }
}
