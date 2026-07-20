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
        dataUrl: image.toDataURL(),
        capturedWidth: size.width,
        capturedHeight: size.height,
      }
    }),
  )

  const candidateFrames = frames.filter(
    (frame): frame is ZoomSnapshotFrame => frame !== null,
  )
  const captureMs = performance.now() - startedAt
  // Never replace the prepared set with a partial capture: a page parked at
  // hidden bounds (frozen gesture, focus presentation) captures nothing, and
  // publishing fewer frames than pages guarantees a dropout for the rest.
  if (candidateFrames.length < pages.length) {
    slog('prepare-partial-discarded', {
      revision,
      captured: candidateFrames.length,
      expected: pages.length,
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

let lastBeginOutcome = ''

export function beginAutomaticZoomSnapshotFreeze(): boolean {
  captureLease += 1
  if (forced) return active
  let liveReason: string | null = null
  if (preparedFrames.length === 0) liveReason = 'no-frames'
  else if (rendererReadyRevision < revision) liveReason = 'renderer-not-ready'
  else if (preparedContentSignature !== currentContentSignature()) {
    liveReason = 'signature-mismatch'
  }
  if (liveReason) {
    const outcome = `live:${liveReason}`
    if (lastBeginOutcome !== outcome) {
      lastBeginOutcome = outcome
      slog('begin-live-fallback', {
        reason: liveReason,
        wasActive: active,
        revision,
        rendererReadyRevision,
        ...(liveReason === 'signature-mismatch'
          ? {
              prepared: preparedContentSignature,
              current: currentContentSignature(),
            }
          : {}),
      })
    }
    return false
  }
  if (lastBeginOutcome !== 'frozen') {
    lastBeginOutcome = 'frozen'
    slog('begin-frozen', { revision, frameCount: preparedFrames.length })
  }
  active = true
  publish({ revision, active: true, frames: preparedFrames })
  return true
}

export function endAutomaticZoomSnapshotFreeze(): void {
  lastBeginOutcome = ''
  if (forced || !active) return
  slog('freeze-end', { revision })
  active = false
  publish({ revision, active: false, frames: preparedFrames })
}

export function scheduleZoomSnapshotPreparation(delayMs = 400): void {
  if (forced || active) return
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
  lastBeginOutcome = ''
  forced = false
  active = false
  preparedFrames = []
  preparedContentSignature = ''
  preparedCaptureSignature = ''
  revision += 1
  publish({ revision, active: false, frames: [] })
}
