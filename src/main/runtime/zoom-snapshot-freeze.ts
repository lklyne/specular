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
    return {
      frameCount: preparedFrames.length,
      encodedBytes: preparedFrames.reduce(
        (total, frame) => total + frame.dataUrl.length,
        0,
      ),
      captureMs: 0,
      rendererReady: rendererReadyRevision >= revision,
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

export function beginAutomaticZoomSnapshotFreeze(): boolean {
  captureLease += 1
  if (forced) return active
  if (
    preparedFrames.length === 0 ||
    rendererReadyRevision < revision ||
    preparedContentSignature !== currentContentSignature()
  ) {
    return false
  }
  active = true
  publish({ revision, active: true, frames: preparedFrames })
  return true
}

export function endAutomaticZoomSnapshotFreeze(): void {
  if (forced || !active) return
  active = false
  publish({ revision, active: false, frames: preparedFrames })
}

export function scheduleZoomSnapshotPreparation(delayMs = 400): void {
  if (forced || active) return
  if (preparationTimer) clearTimeout(preparationTimer)
  preparationTimer = setTimeout(() => {
    preparationTimer = null
    void prepareZoomSnapshotFreeze({ force: false }).catch((error) => {
      console.warn('[zoom-snapshot] background preparation failed:', error)
    })
  }, delayMs)
}

/** Releases renderer and main-process references after live views return. */
export function clearZoomSnapshotFreeze(): void {
  captureLease += 1
  forced = false
  active = false
  preparedFrames = []
  preparedContentSignature = ''
  preparedCaptureSignature = ''
  revision += 1
  publish({ revision, active: false, frames: [] })
}
