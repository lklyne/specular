import type { NativeImage } from 'electron'
import { ipcChannels } from '../../shared/ipc-contract'
import type {
  ZoomSnapshotFrame,
  ZoomSnapshotState,
} from '../../shared/types'
import { pages } from './runtime-context'
import type { Page } from './runtime-entities'
import { safeSend } from './safe-send'
import { bgView } from './view-refs'
import {
  frameMeetsTarget,
  pickBetterFrame,
  snapshotCaptureStillValid,
  snapshotTargetScale,
} from '../../shared/zoom-snapshot-lifecycle'
import { boundEffectivePageContentSize } from './runtime-geometry'
import { screen } from 'electron'
import { CANVAS_MAX_ZOOM } from '../../shared/zoom'
import { zoom } from './runtime-context'
import { captureViaCdp, type CdpCapture } from './zoom-snapshot-cdp-capture'

let preparedFrames: ZoomSnapshotFrame[] = []
let active = false
let forced = false
let revision = 0
let rendererReadyRevision = 0
let preparationTimer: ReturnType<typeof setTimeout> | null = null
let preparedContentSignature = ''
let preparedCaptureSignature = ''
let captureLease = 0
const readyWaiters = new Map<number, (ready: boolean) => void>()

function publish(state: ZoomSnapshotState): void {
  if (!bgView || bgView.webContents.isDestroyed()) return
  safeSend(bgView.webContents, ipcChannels.zoomSnapshotState, state)
}

export function isZoomSnapshotFreezeActive(): boolean {
  return active
}

/**
 * Whether `pageId` should be parked while the freeze is active. Only pages on
 * screen at capture time have a bitmap; a page that scrolls into view during a
 * zoom-out has none, so it stays live rather than parking into a blank hole.
 */
export function isPageParkedByZoomSnapshot(pageId: string): boolean {
  return active && preparedFrames.some((frame) => frame.pageId === pageId)
}

export type ZoomSnapshotParking = 'hidden' | 'warm' | null

/**
 * How the layout pass should place a parked page: `hidden` (zero bounds) for
 * the gesture body, `warm` (full size, off-screen, compositing) during the
 * settle handoff so it re-rasters at the settled scale before it is revealed.
 */
export function zoomSnapshotParkingFor(pageId: string): ZoomSnapshotParking {
  if (!isPageParkedByZoomSnapshot(pageId)) return null
  return handoff ? 'warm' : 'hidden'
}

export function markZoomSnapshotRendererReady(readyRevision: number): void {
  rendererReadyRevision = Math.max(rendererReadyRevision, readyRevision)
  for (const [waitingRevision, resolve] of readyWaiters) {
    if (waitingRevision > rendererReadyRevision) continue
    readyWaiters.delete(waitingRevision)
    resolve(true)
  }
}

function waitForRendererReady(
  waitingRevision: number,
  timeoutMs = 1_000,
): Promise<boolean> {
  if (rendererReadyRevision >= waitingRevision) return Promise.resolve(true)
  return new Promise((resolve) => {
    const timeout = setTimeout(() => {
      readyWaiters.delete(waitingRevision)
      resolve(false)
    }, timeoutMs)
    readyWaiters.set(waitingRevision, (ready) => {
      clearTimeout(timeout)
      resolve(ready)
    })
  })
}

/** Identifies one page's content state: what a frame of it is a picture of. */
function pageContentKey(page: Page): string {
  const viewport = boundEffectivePageContentSize(page)
  return [
    page.id,
    viewport.width,
    viewport.height,
    page.navGeneration,
    page.scrollX ?? 0,
    page.scrollY ?? 0,
  ].join(':')
}

function currentContentSignature(): string {
  return pages.map(pageContentKey).join('|')
}

/**
 * Merges freshly captured frames over the prepared set. A page keeps its
 * existing frame when that frame pictures the same content at a higher
 * resolution; frames for pages that no longer exist are dropped.
 */
function mergeFrames(incoming: ZoomSnapshotFrame[]): ZoomSnapshotFrame[] {
  const existingById = new Map(preparedFrames.map((frame) => [frame.pageId, frame]))
  const merged = new Map(existingById)
  for (const frame of incoming) {
    merged.set(frame.pageId, pickBetterFrame(existingById.get(frame.pageId), frame))
  }
  const liveIds = new Set(pages.map((page) => page.id))
  return [...merged.values()].filter((frame) => liveIds.has(frame.pageId))
}

function currentCaptureSignature(): string {
  return pages.map((page) => {
    const bounds = page.pageView.getBounds()
    return [
      page.id,
      bounds.x,
      bounds.y,
      bounds.width,
      bounds.height,
    ].join(':')
  }).join('|')
}

/**
 * Captures every currently visible live page in parallel. This is deliberately
 * an explicit perf-spike preparation step: capture cost is excluded from the
 * gesture trace so the experiment measures the frozen compositor substrate.
 */
export async function prepareZoomSnapshotFreeze(options?: {
  force?: boolean
}): Promise<{
  frameCount: number
  encodedBytes: number
  captureMs: number
  rendererReady: boolean
  reused: boolean
  discarded: boolean
}> {
  const contentSignature = currentContentSignature()
  const captureSignature = currentCaptureSignature()
  if (
    options?.force !== true &&
    preparedFrames.length > 0 &&
    preparedCaptureSignature === captureSignature
  ) {
    // If the renderer never acked this revision (e.g. it published before the
    // renderer booted), the frames are unusable until re-delivered. Republish
    // and wait again instead of returning a stale "ready" set.
    let rendererReady = rendererReadyRevision >= revision
    if (!rendererReady) {
      publish({ revision, active, frames: preparedFrames })
      rendererReady = await waitForRendererReady(revision)
    }
    return {
      frameCount: preparedFrames.length,
      encodedBytes: preparedFrames.reduce(
        (total, frame) => total + frame.dataUrl.length,
        0,
      ),
      captureMs: 0,
      rendererReady,
      reused: true,
      discarded: false,
    }
  }

  const startedAt = performance.now()
  const captureLeaseAtStart = captureLease
  const frames = await Promise.all(
    pages.map(async (page): Promise<ZoomSnapshotFrame | null> => {
      if (page.pageView.webContents.isDestroyed()) return null
      const bounds = page.pageView.getBounds()
      if (bounds.width <= 0 || bounds.height <= 0) return null

      const image = await page.pageView.webContents.capturePage()
      if (image.isEmpty()) return null
      return encodeFrame(page, image)
    }),
  )

  const capturedFrames = frames.filter(
    (frame): frame is ZoomSnapshotFrame => frame !== null,
  )
  const captureMs = performance.now() - startedAt
  // Culled (off-screen) pages sit at zero bounds and can never capture, so a
  // whole-set discard would leave the prepared frames permanently stale on any
  // canvas with an off-screen page, and they'd then be shown scaled far past
  // their captured resolution. Instead, capture what's visible and carry the
  // prior frame forward for pages that can't capture right now. A page with
  // no prior frame stays absent (blank only if it scrolls into view
  // mid-gesture, which live-view restore covers at gesture end).
  const capturableCount = pages.filter((page) => {
    if (page.pageView.webContents.isDestroyed()) return false
    const bounds = page.pageView.getBounds()
    return bounds.width > 0 && bounds.height > 0
  }).length
  const candidateFrames = mergeFrames(capturedFrames)
  // A page that was visible but still failed to capture (mid-teardown, empty
  // paint) is a transient state. Keep the prior set and retry later.
  if (capturedFrames.length < capturableCount) {
    return {
      frameCount: preparedFrames.length,
      encodedBytes: preparedFrames.reduce(
        (total, frame) => total + frame.dataUrl.length,
        0,
      ),
      captureMs,
      rendererReady: rendererReadyRevision >= revision,
      reused: false,
      discarded: true,
    }
  }
  if (!snapshotCaptureStillValid({
    captureLeaseAtStart,
    currentCaptureLease: captureLease,
    signatureAtStart: contentSignature,
    currentSignature: currentContentSignature(),
  })) {
    return {
      frameCount: preparedFrames.length,
      encodedBytes: preparedFrames.reduce(
        (total, frame) => total + frame.dataUrl.length,
        0,
      ),
      captureMs,
      rendererReady: rendererReadyRevision >= revision,
      reused: false,
      discarded: true,
    }
  }

  preparedFrames = candidateFrames
  preparedContentSignature = contentSignature
  preparedCaptureSignature = captureSignature
  revision += 1
  const preparedRevision = revision
  publish({ revision, active: false, frames: preparedFrames })
  const rendererReady = await waitForRendererReady(preparedRevision)
  return {
    frameCount: preparedFrames.length,
    encodedBytes: preparedFrames.reduce(
      (total, frame) => total + frame.dataUrl.length,
      0,
    ),
    captureMs,
    rendererReady,
    reused: false,
    discarded: false,
  }
}

/** Mount or reveal the prepared bitmaps while live views are still visible. */
export function showPreparedZoomSnapshots(): void {
  publish({ revision, active: true, frames: preparedFrames })
}

/** Controls the layout-engine gate that parks live page views at zero bounds. */
export function setZoomSnapshotFreezeActive(next: boolean): void {
  if (next) captureLease += 1
  forced = next
  active = next && preparedFrames.length > 0
}

/**
 * Gesture rule. The substrate is chosen once per zoom gesture and leased to
 * its generation, never re-decided per tick:
 *
 *   - frames ready at gesture start → frozen for the whole gesture;
 *   - otherwise live, with a capture kicked off immediately; if it lands while
 *     the same gesture is still running, the gesture adopts the frames
 *     (live→frozen is seamless: the bitmap matches the live surface);
 *   - frozen→live never happens before settle. That switch is the dropout
 *     frame the freeze exists to prevent.
 */
let gestureGen = 0
let gestureRunning = false
let handoff = false

function liveFallbackReason(): string | null {
  if (preparedFrames.length === 0) return 'no-frames'
  if (rendererReadyRevision < revision) return 'renderer-not-ready'
  if (preparedContentSignature !== currentContentSignature()) return 'signature-mismatch'
  return null
}

function freezeForGesture(): boolean {
  const reason = liveFallbackReason()
  if (reason) {
    return false
  }
  // Parking the live views invalidates any capture still in flight.
  captureLease += 1
  active = true
  publish({ revision, active: true, frames: preparedFrames })
  return true
}

export function beginZoomGesture(gen: number): boolean {
  gestureGen = gen
  gestureRunning = true
  // A gesture that starts mid-handoff keeps the frames; the pending reveal
  // sees the generation change and stands down.
  handoff = false
  if (forced) return active
  if (freezeForGesture()) return true
  void prepareZoomSnapshotFreeze({ force: true })
    .then((result) => {
      if (!gestureRunning || gestureGen !== gen || active || forced) return
      if (result.discarded || !result.rendererReady || result.frameCount === 0) return
      freezeForGesture()
    })
    .catch((error) => {
      console.warn('[zoom-snapshot] gesture-start preparation failed:', error)
    })
  return false
}

/**
 * Starts the settle handoff for a frozen gesture: parked pages move to the
 * warm off-screen park so the next layout pass sizes and re-emulates them.
 * Returns false when there is nothing to hand off (live gesture, forced
 * freeze) and the caller should end the gesture directly.
 */
export function beginZoomSnapshotHandoff(gen: number): boolean {
  if (gen !== gestureGen || forced || !active) return false
  handoff = true
  return true
}

const DOUBLE_RAF = 'new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r(true))))'
/**
 * Upper bound on one page's settle re-raster plus hi-res capture. The raster
 * stays up while we wait, so a long wait costs interactivity, not a wrong
 * frame; the cap only exists so a hung or throttled page (an occluded window
 * stops its frames) cannot hold the reveal forever.
 */
const HANDOFF_TIMEOUT_MS = 1_000

export interface HandoffCapture {
  page: Page
  contentKey: string
  /** Null when the page timed out or returned an empty capture. */
  image: NativeImage | null
  /** Renderer-side raster above on-screen resolution; null when not needed or failed. */
  hiRes: CdpCapture | null
}

function encodeFrame(page: Page, image: NativeImage): ZoomSnapshotFrame {
  const size = image.getSize()
  return {
    pageId: page.id,
    contentKey: pageContentKey(page),
    // JPEG: PNG encode runs synchronously on this thread and costs
    // 120 to 220ms per prepare at normal zooms; JPEG-85 is ~6× cheaper and
    // decodes faster in the renderer (perf-zoom-pan-log Exp D).
    dataUrl: `data:image/jpeg;base64,${image.toJPEG(85).toString('base64')}`,
    capturedWidth: size.width,
    capturedHeight: size.height,
  }
}

function encodeCdpFrame(page: Page, contentKey: string, capture: CdpCapture): ZoomSnapshotFrame {
  return {
    pageId: page.id,
    contentKey,
    dataUrl: `data:image/jpeg;base64,${capture.jpeg.toString('base64')}`,
    capturedWidth: capture.width,
    capturedHeight: capture.height,
  }
}

/**
 * The capture a settled page needs beyond its presentation frame. The
 * on-screen surface only has `css × zoom × dpr` pixels, which is nothing to
 * zoom back into from far out, so while the page is still parked behind the raster
 * we ask its renderer for a raster at the target scale. Skipped when the
 * retained frame already pictures this content at that resolution.
 */
function hiResPlan(page: Page): { scale: number; cssWidth: number; cssHeight: number; dpr: number } | null {
  const css = boundEffectivePageContentSize(page)
  const dpr = screen.getPrimaryDisplay().scaleFactor
  const scale = snapshotTargetScale({
    zoom,
    cssWidth: css.width,
    cssHeight: css.height,
    devicePixelRatio: dpr,
    maxZoom: CANVAS_MAX_ZOOM,
  })
  // At or above the target the presentation capture already is the best frame.
  if (scale <= zoom) return null
  const existing = preparedFrames.find((frame) => frame.pageId === page.id)
  if (frameMeetsTarget(existing, {
    contentKey: pageContentKey(page),
    cssWidth: css.width,
    devicePixelRatio: dpr,
    targetScale: scale,
  })) return null
  return { scale, cssWidth: css.width, cssHeight: css.height, dpr }
}

/**
 * Waits for each warm-parked page to present a frame at the settled scale,
 * and returns that frame. Two animation frames mean the page has laid out
 * and committed at the new emulation; they do not mean the compositor has
 * drawn it. `capturePage` is a copy request on the view's surface, fulfilled
 * only once a frame containing that commit is drawn, so its resolution is
 * the presentation signal the reveal needs. The copy doubles as the next
 * gesture's snapshot when the page is already at or above the target
 * resolution; otherwise the renderer-side hi-res capture taken in between
 * is the snapshot, and a second pair of animation frames confirms the page
 * has re-presented at the settled scale after it.
 */
export function captureParkedPagesAtSettle(): Promise<HandoffCapture[]> {
  const parked = pages.filter(
    (page) => isPageParkedByZoomSnapshot(page.id) && !page.pageView.webContents.isDestroyed(),
  )
  return Promise.all(
    parked.map(async (page): Promise<HandoffCapture> => {
      const contents = page.pageView.webContents
      const contentKey = pageContentKey(page)
      let hiRes: CdpCapture | null = null
      const presented = (async () => {
        await contents.executeJavaScript(DOUBLE_RAF).catch(() => undefined)
        if (contents.isDestroyed()) return null
        const plan = hiResPlan(page)
        if (plan) {
          // The screenshot re-lays the page out at the target scale and back;
          // off-screen that is invisible, and the restore commit is what the
          // presentation capture below then waits on.
          hiRes = await captureViaCdp(page, {
            scale: plan.scale,
            cssWidth: plan.cssWidth,
            cssHeight: plan.cssHeight,
            emulation: { deviceScaleFactor: plan.dpr, scale: zoom },
          }).catch((error) => {
            return null
          })
          await contents.executeJavaScript(DOUBLE_RAF).catch(() => undefined)
          if (contents.isDestroyed()) return null
        }
        const image = await contents.capturePage()
        return image.isEmpty() ? null : image
      })()
      const image = await Promise.race([
        presented.catch(() => null),
        new Promise<null>((resolve) => setTimeout(() => resolve(null), HANDOFF_TIMEOUT_MS)),
      ])
      return { page, contentKey, image, hiRes }
    }),
  )
}

/**
 * Publishes the settle captures as the prepared set. Deferred one macrotask
 * so the reveal's setBounds calls reach the window server before the
 * synchronous JPEG encode blocks this thread. Returns false when the set is
 * not a complete picture of the visible pages (a timed-out page, a page that
 * stayed live through the gesture, a gesture or scroll that landed while we
 * waited) and the caller should fall back to a full background prepare.
 */
export async function adoptHandoffCaptures(captures: HandoffCapture[]): Promise<boolean> {
  const leaseAtCapture = captureLease
  const contentSignatureAtCapture = currentContentSignature()
  await new Promise<void>((resolve) => setImmediate(resolve))
  if (forced || active || gestureRunning) return false
  if (!snapshotCaptureStillValid({
    captureLeaseAtStart: leaseAtCapture,
    currentCaptureLease: captureLease,
    signatureAtStart: contentSignatureAtCapture,
    currentSignature: currentContentSignature(),
  })) {
    return false
  }
  const captured = captures.filter(
    (capture): capture is HandoffCapture & { image: NativeImage } =>
      capture.image !== null && !capture.page.pageView.webContents.isDestroyed(),
  )
  if (captured.length === 0) return false
  const capturedIds = new Set(captured.map((capture) => capture.page.id))
  preparedFrames = mergeFrames(
    captured.map((capture) =>
      capture.hiRes
        ? encodeCdpFrame(capture.page, capture.contentKey, capture.hiRes)
        : encodeFrame(capture.page, capture.image),
    ),
  )
  preparedContentSignature = contentSignatureAtCapture
  const visibleIds = pages.filter((page) => {
    if (page.pageView.webContents.isDestroyed()) return false
    const bounds = page.pageView.getBounds()
    return bounds.width > 0 && bounds.height > 0
  }).map((page) => page.id)
  const complete = visibleIds.every((id) => capturedIds.has(id))
  // Only a complete set may claim the current bounds; anything less leaves
  // the signature stale so the scheduled prepare recaptures.
  preparedCaptureSignature = complete ? currentCaptureSignature() : ''
  revision += 1
  const adoptedRevision = revision
  publish({ revision, active: false, frames: preparedFrames })
  const rendererReady = await waitForRendererReady(adoptedRevision)
  return complete
}

export function endZoomGesture(gen: number): void {
  if (gen !== gestureGen) return
  gestureRunning = false
  handoff = false
  if (forced || !active) return
  active = false
  publish({ revision, active: false, frames: preparedFrames })
}

/**
 * Background prepare for everything the settle handoff did not cover: a live
 * (unfrozen) gesture, a page that stayed live or timed out, any layout that
 * moved a page. The delay covers one compositor frame so capturePage picks
 * up the surface at its new bounds. Any longer just widens the window in
 * which a new gesture finds no snapshot.
 */
const PREPARE_DELAY_MS = 50

export function scheduleZoomSnapshotPreparation(delayMs = PREPARE_DELAY_MS): void {
  if (forced || active || gestureRunning) return
  if (preparationTimer) clearTimeout(preparationTimer)
  preparationTimer = setTimeout(() => {
    preparationTimer = null
    // A gesture may have started during the delay; capturing parked (hidden)
    // views would yield empty frames and clobber the prepared set.
    if (forced || active) {
      return
    }
    void prepareZoomSnapshotFreeze({ force: false }).catch((error) => {
      console.warn('[zoom-snapshot] background preparation failed:', error)
    })
  }, delayMs)
}

/** Releases renderer and main-process references after live views return. */
export function clearZoomSnapshotFreeze(): void {
  captureLease += 1
  gestureRunning = false
  handoff = false
  forced = false
  active = false
  preparedFrames = []
  preparedContentSignature = ''
  preparedCaptureSignature = ''
  revision += 1
  publish({ revision, active: false, frames: [] })
}
