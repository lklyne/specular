import type { NativeImage } from 'electron'
import type { FrozenPageFrame } from '../../shared/types'
import { pages } from './runtime-context'
import type { Page } from './runtime-entities'
import {
  capturePageFrame,
  currentRevision,
  encodePageFrame,
  nextRevision,
  pageClaimedByOtherFreeze,
  pageContentKey,
  publish,
  readyRevision,
  registerFreeze,
  releaseFreeze,
  waitForRendererReady,
} from './page-freeze'
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
import { withCaptureMetrics } from './page-emulation'
import { msSinceCameraInput } from './camera-input-clock'

const FREEZE_TARGET = 'bg'
const FREEZE_ID = 'zoom'

let preparedFrames: FrozenPageFrame[] = []
let active = false
let forced = false
let preparationTimer: ReturnType<typeof setTimeout> | null = null
let preparedContentSignature = ''
let captureLease = 0

/**
 * Mirrors `active`/`handoff`/`preparedFrames` into the shared parking
 * registry so the layout engine can place a parked page without knowing
 * which freeze owns it. Called at every point those three change.
 */
function syncFreezeRegistry(): void {
  if (!active) {
    releaseFreeze(FREEZE_ID)
    return
  }
  registerFreeze(FREEZE_ID, {
    target: FREEZE_TARGET,
    pageIds: preparedFrames.map((frame) => frame.pageId),
    parking: handoff ? 'warm' : 'hidden',
  })
}

/**
 * Whether `pageId` is parked as part of the zoom freeze right now. Only
 * pages on screen at capture time have a bitmap; a page that scrolls into
 * view during a zoom-out has none, so it stays live rather than parking
 * into a blank hole.
 */
function isParkedByZoom(pageId: string): boolean {
  return active && preparedFrames.some((frame) => frame.pageId === pageId)
}

/** Identifies one page's content state: what a frame of it is a picture of. */
function currentContentSignature(): string {
  return pages.map(pageContentKey).join('|')
}

/**
 * Merges freshly captured frames over the prepared set. A page keeps its
 * existing frame when that frame pictures the same content at a higher
 * resolution; frames for pages that no longer exist are dropped.
 */
function mergeFrames(incoming: FrozenPageFrame[]): FrozenPageFrame[] {
  const existingById = new Map(preparedFrames.map((frame) => [frame.pageId, frame]))
  const merged = new Map(existingById)
  for (const frame of incoming) {
    merged.set(frame.pageId, pickBetterFrame(existingById.get(frame.pageId), frame))
  }
  const liveIds = new Set(pages.map((page) => page.id))
  return [...merged.values()].filter((frame) => liveIds.has(frame.pageId))
}

/** Pages a capture can succeed on: alive, and occupying screen space. */
function onScreenPages(): Page[] {
  return pages.filter((page) => {
    if (page.pageView.webContents.isDestroyed()) return false
    const bounds = page.pageView.getBounds()
    return bounds.width > 0 && bounds.height > 0
  })
}

/**
 * Whether the prepared set still pictures every page that needs a frame, at no
 * less detail than the page occupies on screen right now.
 *
 * Deliberately says nothing about where a page sits. Frames are drawn into the
 * rect the renderer projects from the camera, so a pan moves the picture
 * without changing what it is a picture of. Keying on position instead would
 * discard the whole set every time panning stopped, and re-capture and
 * re-encode every page to arrive back at the same frames.
 *
 * A page that leaves the screen needs no frame, and keeps the one it has
 * through `mergeFrames`; a page that arrives has none, which is what makes the
 * set incomplete and schedules the capture that covers it.
 */
function preparedFramesCoverScreen(): boolean {
  if (preparedFrames.length === 0) return false
  const frameByPageId = new Map(preparedFrames.map((frame) => [frame.pageId, frame]))
  const devicePixelRatio = screen.getPrimaryDisplay().scaleFactor
  return onScreenPages().every((page) =>
    frameMeetsTarget(frameByPageId.get(page.id), {
      contentKey: pageContentKey(page),
      cssWidth: boundEffectivePageContentSize(page).width,
      devicePixelRatio,
      targetScale: zoom,
    }),
  )
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
  if (options?.force !== true && preparedFramesCoverScreen()) {
    // If the renderer never acked this revision (e.g. it published before the
    // renderer booted), the frames are unusable until re-delivered. Republish
    // and wait again instead of returning a stale "ready" set.
    const revision = currentRevision(FREEZE_TARGET)
    let rendererReady = readyRevision(FREEZE_TARGET) >= revision
    if (!rendererReady) {
      publish(FREEZE_TARGET, { revision, target: FREEZE_TARGET, active, frames: preparedFrames })
      rendererReady = await waitForRendererReady(FREEZE_TARGET, revision)
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
  // A page parked by another freeze (a drag mid-gesture) sits at zero
  // bounds, so capturing it yields an empty frame that would count toward a
  // discard below. A page lives in at most one freeze; zoom leaves it out
  // and picks it up again once the owner releases it.
  const unclaimedPages = pages.filter((page) => !pageClaimedByOtherFreeze(page.id, FREEZE_ID))
  const frames = await Promise.all(unclaimedPages.map((page) => capturePageFrame(page)))

  const capturedFrames = frames.filter(
    (frame): frame is FrozenPageFrame => frame !== null,
  )
  const captureMs = performance.now() - startedAt
  // Culled (off-screen) pages sit at zero bounds and can never capture, so a
  // whole-set discard would leave the prepared frames permanently stale on any
  // canvas with an off-screen page, and they'd then be shown scaled far past
  // their captured resolution. Instead, capture what's visible and carry the
  // prior frame forward for pages that can't capture right now. A page with
  // no prior frame stays absent (blank only if it scrolls into view
  // mid-gesture, which live-view restore covers at gesture end).
  const unclaimedIds = new Set(unclaimedPages.map((page) => page.id))
  const capturableCount = onScreenPages().filter((page) => unclaimedIds.has(page.id)).length
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
      rendererReady: readyRevision(FREEZE_TARGET) >= currentRevision(FREEZE_TARGET),
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
      rendererReady: readyRevision(FREEZE_TARGET) >= currentRevision(FREEZE_TARGET),
      reused: false,
      discarded: true,
    }
  }

  preparedFrames = candidateFrames
  preparedContentSignature = contentSignature
  const preparedRevision = nextRevision(FREEZE_TARGET)
  publish(FREEZE_TARGET, { revision: preparedRevision, target: FREEZE_TARGET, active: false, frames: preparedFrames })
  const rendererReady = await waitForRendererReady(FREEZE_TARGET, preparedRevision)
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
  publish(FREEZE_TARGET, { revision: currentRevision(FREEZE_TARGET), target: FREEZE_TARGET, active: true, frames: preparedFrames })
}

/** Controls the layout-engine gate that parks live page views at zero bounds. */
export function setZoomSnapshotFreezeActive(next: boolean): void {
  if (next) captureLease += 1
  forced = next
  active = next && preparedFrames.length > 0
  syncFreezeRegistry()
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
  if (readyRevision(FREEZE_TARGET) < currentRevision(FREEZE_TARGET)) return 'renderer-not-ready'
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
  syncFreezeRegistry()
  publish(FREEZE_TARGET, { revision: currentRevision(FREEZE_TARGET), target: FREEZE_TARGET, active: true, frames: preparedFrames })
  return true
}

export function beginZoomGesture(gen: number): boolean {
  gestureGen = gen
  gestureRunning = true
  // A gesture that starts mid-handoff keeps the frames; the pending reveal
  // sees the generation change and stands down.
  handoff = false
  syncFreezeRegistry()
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
  syncFreezeRegistry()
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
  /** Surface copy at raised pixel density; null when not needed or failed. */
  hiRes: NativeImage | null
}

/**
 * The capture a settled page needs beyond its presentation frame. The
 * on-screen surface only has `css × zoom × dpr` pixels, which is nothing to
 * zoom back into from far out, so while the page is still parked behind the raster
 * it re-rasters at a raised pixel density and we copy that surface. Skipped when the
 * retained frame already pictures this content at that resolution.
 */
function hiResPlan(page: Page): { densityFactor: number; expectedWidth: number } | null {
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
  // The page renders at css × zoom × dpr pixels; raising density by
  // scale / zoom gives css × scale × dpr, the target.
  return {
    densityFactor: scale / zoom,
    expectedWidth: Math.floor(css.width * scale * dpr),
  }
}

/**
 * Waits for each warm-parked page to present a frame at the settled scale,
 * and returns that frame. Two animation frames mean the page has laid out
 * and committed at the new emulation; they do not mean the compositor has
 * drawn it. `capturePage` is a copy request on the view's surface, fulfilled
 * only once a frame containing that commit is drawn, so its resolution is
 * the presentation signal the reveal needs. The copy doubles as the next
 * gesture's snapshot when the page is already at or above the target
 * resolution; otherwise the raised-density capture taken in between
 * is the snapshot, and a second pair of animation frames confirms the page
 * has re-presented at the settled scale after it.
 */
export function captureParkedPagesAtSettle(): Promise<HandoffCapture[]> {
  const parked = pages.filter(
    (page) => isParkedByZoom(page.id) && !page.pageView.webContents.isDestroyed(),
  )
  return Promise.all(
    parked.map(async (page): Promise<HandoffCapture> => {
      const contents = page.pageView.webContents
      const contentKey = pageContentKey(page)
      let hiRes: NativeImage | null = null
      const presented = (async () => {
        await contents.executeJavaScript(DOUBLE_RAF).catch(() => undefined)
        if (contents.isDestroyed()) return null
        const plan = hiResPlan(page)
        if (plan) {
          // The page re-rasters at the raised density and back; off-screen
          // that is invisible, and the restore commit is what the
          // presentation capture below then waits on.
          hiRes = await withCaptureMetrics(contents, plan.densityFactor, async () => {
            await contents.executeJavaScript(DOUBLE_RAF).catch(() => undefined)
            if (contents.isDestroyed()) return null
            const image = await contents.capturePage()
            return image.isEmpty() ? null : image
          }).catch(() => null)
          if (hiRes && hiRes.getSize().width < plan.expectedWidth) {
            // Fewer pixels than the density bump should yield: the frame
            // still merges (it may beat what we have) but the target is
            // never met, so every settle would re-capture. Worth knowing.
            console.warn(
              `[zoom-snapshot] hi-res capture of ${page.id} is ${hiRes.getSize().width}px, expected ${plan.expectedWidth}px`,
            )
          }
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
 * How quiet the camera must be before a synchronous JPEG encode may run, and
 * how often to re-check while it is not. The threshold sits above one
 * trackpad input interval so a continuous pan reads as "moving" between its
 * ticks, and below the settle lease so a pause that ends a gesture also
 * releases the encodes promptly.
 */
const ENCODE_CAMERA_QUIET_MS = 120
const ENCODE_QUIET_RECHECK_MS = 60

async function waitForCameraQuiet(): Promise<void> {
  await new Promise<void>((resolve) => setImmediate(resolve))
  while (msSinceCameraInput() < ENCODE_CAMERA_QUIET_MS) {
    await new Promise<void>((resolve) => setTimeout(resolve, ENCODE_QUIET_RECHECK_MS))
  }
}

/**
 * Publishes the settle captures as the prepared set. Encoding waits for a
 * pause in camera input and takes one frame per macrotask, so the reveal's
 * setBounds and any in-flight pan ticks are never queued behind the
 * synchronous JPEG encodes. Returns false when the set is not a complete
 * picture of the visible pages (a timed-out page, a page that stayed live
 * through the gesture, a gesture or scroll that landed while we waited) and
 * the caller should fall back to a full background prepare.
 */
export async function adoptHandoffCaptures(captures: HandoffCapture[]): Promise<boolean> {
  const leaseAtCapture = captureLease
  const contentSignatureAtCapture = currentContentSignature()
  const stillValid = (): boolean =>
    !forced &&
    !active &&
    !gestureRunning &&
    snapshotCaptureStillValid({
      captureLeaseAtStart: leaseAtCapture,
      currentCaptureLease: captureLease,
      signatureAtStart: contentSignatureAtCapture,
      currentSignature: currentContentSignature(),
    })
  const captured = captures.filter(
    (capture): capture is HandoffCapture & { image: NativeImage } => capture.image !== null,
  )
  if (captured.length === 0) return false
  // The JPEG encode is synchronous and can cost tens of milliseconds per
  // frame, so it never runs while the camera is moving: a pan tick queued
  // behind a block of encodes lands as one visible jump. Each frame waits for
  // a pause in camera input and takes its own macrotask; the reveal's
  // setBounds calls reach the window server before the first encode can
  // block this thread. A gesture or content change while we wait hands the
  // work back to the caller's background prepare.
  const encoded: FrozenPageFrame[] = []
  for (const capture of captured) {
    await waitForCameraQuiet()
    if (!stillValid()) return false
    if (capture.page.pageView.webContents.isDestroyed()) continue
    encoded.push(encodePageFrame(capture.page, capture.hiRes ?? capture.image))
  }
  if (encoded.length === 0) return false
  preparedFrames = mergeFrames(encoded)
  preparedContentSignature = contentSignatureAtCapture
  // A settle that left any on-screen page without a usable frame is not a
  // whole picture; the caller schedules the prepare that fills the gap.
  const complete = preparedFramesCoverScreen()
  const adoptedRevision = nextRevision(FREEZE_TARGET)
  publish(FREEZE_TARGET, { revision: adoptedRevision, target: FREEZE_TARGET, active: false, frames: preparedFrames })
  await waitForRendererReady(FREEZE_TARGET, adoptedRevision)
  return complete
}

export function endZoomGesture(gen: number): void {
  if (gen !== gestureGen) return
  gestureRunning = false
  handoff = false
  syncFreezeRegistry()
  if (forced || !active) return
  active = false
  syncFreezeRegistry()
  publish(FREEZE_TARGET, { revision: currentRevision(FREEZE_TARGET), target: FREEZE_TARGET, active: false, frames: preparedFrames })
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
  syncFreezeRegistry()
  const clearedRevision = nextRevision(FREEZE_TARGET)
  publish(FREEZE_TARGET, { revision: clearedRevision, target: FREEZE_TARGET, active: false, frames: [] })
}
