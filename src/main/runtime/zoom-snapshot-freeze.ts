import { ipcChannels } from '../../shared/ipc-contract'
import type {
  ZoomSnapshotFrame,
  ZoomSnapshotState,
} from '../../shared/types'
import { pages } from './runtime-context'
import { safeSend } from './safe-send'
import { bgView } from './view-refs'
import { snapshotCaptureStillValid } from '../../shared/zoom-snapshot-lifecycle'
import { boundEffectivePageContentSize } from './runtime-geometry'

// ponytail: console diagnostics for the snapshot lifecycle; delete once the
// dropout/fallback races are understood.
export function slog(event: string, data?: Record<string, unknown>): void {
  const t = performance.now().toFixed(0)
  console.log(`[zoom-snap +${t}ms] ${event}${data ? ` ${JSON.stringify(data)}` : ''}`)
}

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
  slog('renderer-ready', { readyRevision, revision, active })
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

function currentContentSignature(): string {
  return pages.map((page) => {
    const viewport = boundEffectivePageContentSize(page)
    return [
      page.id,
      viewport.width,
      viewport.height,
      page.navGeneration,
      page.scrollX ?? 0,
      page.scrollY ?? 0,
    ].join(':')
  }).join('|')
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
    // renderer booted), the frames are unusable until re-delivered — republish
    // and wait again instead of returning a stale "ready" set.
    let rendererReady = rendererReadyRevision >= revision
    if (!rendererReady) {
      slog('prepare-reused-republish', { revision, rendererReadyRevision })
      publish({ revision, active, frames: preparedFrames })
      rendererReady = await waitForRendererReady(revision)
      if (!rendererReady) slog('renderer-ready-timeout', { revision })
    } else {
      slog('prepare-reused', { revision, frameCount: preparedFrames.length })
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
      const size = image.getSize()
      return {
        pageId: page.id,
        // JPEG: PNG encode runs synchronously on this thread and costs
        // 120–220ms per prepare at normal zooms; JPEG-85 is ~6× cheaper and
        // decodes faster in the renderer (perf-zoom-pan-log Exp D).
        dataUrl: `data:image/jpeg;base64,${image.toJPEG(85).toString('base64')}`,
        capturedWidth: size.width,
        capturedHeight: size.height,
      }
    }),
  )

  const capturedFrames = frames.filter(
    (frame): frame is ZoomSnapshotFrame => frame !== null,
  )
  const captureMs = performance.now() - startedAt
  // Culled (off-screen) pages sit at zero bounds and can never capture, so a
  // whole-set discard would leave the prepared frames permanently stale on any
  // canvas with an off-screen page — they'd then be shown scaled far past
  // their captured resolution. Instead, capture what's visible and carry the
  // prior frame forward for pages that can't capture right now. A page with
  // no prior frame stays absent (blank only if it scrolls into view
  // mid-gesture, which live-view restore covers at gesture end).
  const capturableCount = pages.filter((page) => {
    if (page.pageView.webContents.isDestroyed()) return false
    const bounds = page.pageView.getBounds()
    return bounds.width > 0 && bounds.height > 0
  }).length
  const capturedIds = new Set(capturedFrames.map((frame) => frame.pageId))
  const candidateFrames = [
    ...capturedFrames,
    ...preparedFrames.filter((frame) => !capturedIds.has(frame.pageId)),
  ]
  // A page that was visible but still failed to capture (mid-teardown, empty
  // paint) is a transient state — keep the prior set and retry later.
  if (capturedFrames.length < capturableCount) {
    slog('prepare-partial-discarded', {
      revision,
      captured: capturedFrames.length,
      expected: capturableCount,
      captureMs: Math.round(captureMs),
    })
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
    slog('prepare-discarded', {
      revision,
      captureMs: Math.round(captureMs),
      leaseAtStart: captureLeaseAtStart,
      leaseNow: captureLease,
      signatureAtStart: contentSignature,
      signatureNow: currentContentSignature(),
    })
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
  slog('prepare-published', {
    revision,
    frameCount: preparedFrames.length,
    droppedPages: pages.length - preparedFrames.length,
    captureMs: Math.round(captureMs),
    activeDuringPublish: active,
  })
  publish({ revision, active: false, frames: preparedFrames })
  const rendererReady = await waitForRendererReady(preparedRevision)
  if (!rendererReady) slog('renderer-ready-timeout', { revision: preparedRevision })
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

function freezeForGesture(stage: 'gesture-start' | 'mid-gesture'): boolean {
  const reason = liveFallbackReason()
  if (reason) {
    slog('gesture-live', { gen: gestureGen, stage, reason, revision, rendererReadyRevision })
    return false
  }
  // Parking the live views invalidates any capture still in flight.
  captureLease += 1
  active = true
  slog('gesture-frozen', { gen: gestureGen, stage, revision, frameCount: preparedFrames.length })
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
  if (freezeForGesture('gesture-start')) return true
  void prepareZoomSnapshotFreeze({ force: true })
    .then((result) => {
      if (!gestureRunning || gestureGen !== gen || active || forced) return
      if (result.discarded || !result.rendererReady || result.frameCount === 0) return
      freezeForGesture('mid-gesture')
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
  slog('handoff-begin', { gen, revision })
  return true
}

const DOUBLE_RAF = 'new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r(true))))'
const RERASTER_TIMEOUT_MS = 250

/**
 * Resolves once every parked page has committed a frame at its current
 * emulation: two animation frames after the change means the renderer has
 * laid out and submitted at the new scale. A page that cannot answer (hung,
 * throttled, mid-navigation) is released by the timeout rather than holding
 * the reveal.
 */
export function waitForParkedPagesRerastered(): Promise<void> {
  const parked = pages.filter(
    (page) => isPageParkedByZoomSnapshot(page.id) && !page.pageView.webContents.isDestroyed(),
  )
  const startedAt = performance.now()
  return Promise.all(
    parked.map((page) =>
      Promise.race([
        page.pageView.webContents.executeJavaScript(DOUBLE_RAF).catch(() => undefined),
        new Promise<void>((resolve) => setTimeout(resolve, RERASTER_TIMEOUT_MS)),
      ]),
    ),
  ).then(() => {
    slog('handoff-rerastered', { pages: parked.length, waitMs: Math.round(performance.now() - startedAt) })
  })
}

export function endZoomGesture(gen: number): void {
  if (gen !== gestureGen) return
  gestureRunning = false
  handoff = false
  if (forced || !active) return
  slog('gesture-end', { gen, revision })
  active = false
  publish({ revision, active: false, frames: preparedFrames })
}

/**
 * Scheduled after the settle handoff, so parked pages have already committed
 * a frame at the exact scale; the delay covers one compositor frame for
 * capturePage to pick up the revealed surface. Any longer just widens the
 * window in which a new gesture finds no snapshot.
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
      slog('prepare-skipped-active', { revision })
      return
    }
    void prepareZoomSnapshotFreeze({ force: false }).catch((error) => {
      console.warn('[zoom-snapshot] background preparation failed:', error)
    })
  }, delayMs)
}

/** Releases renderer and main-process references after live views return. */
export function clearZoomSnapshotFreeze(): void {
  slog('clear', { revision, wasActive: active })
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
