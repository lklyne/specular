import type { NativeImage } from 'electron'
import { ipcChannels } from '../../shared/ipc-contract'
import type { FreezeTarget, FrozenPageFrame, FrozenPagesState } from '../../shared/types'
import { safeSend } from './safe-send'
import { aboveView, bgView } from './view-refs'
import { boundEffectivePageContentSize } from './runtime-geometry'
import type { Page } from './runtime-entities'

/**
 * Per-target publish/ack channel. Each freeze consumer (zoom today, a future
 * drag freeze) owns its own revision sequence and ready-ack, so one target's
 * renderer never blocks another's.
 */
const revisionByTarget = new Map<FreezeTarget, number>()
const rendererReadyRevisionByTarget = new Map<FreezeTarget, number>()
const readyWaitersByTarget = new Map<FreezeTarget, Map<number, (ready: boolean) => void>>()

function waitersFor(target: FreezeTarget): Map<number, (ready: boolean) => void> {
  let waiters = readyWaitersByTarget.get(target)
  if (!waiters) {
    waiters = new Map()
    readyWaitersByTarget.set(target, waiters)
  }
  return waiters
}

/** Allocates the next revision for `target`. Callers stamp it onto the state they publish. */
export function nextRevision(target: FreezeTarget): number {
  const next = (revisionByTarget.get(target) ?? 0) + 1
  revisionByTarget.set(target, next)
  return next
}

export function currentRevision(target: FreezeTarget): number {
  return revisionByTarget.get(target) ?? 0
}

export function readyRevision(target: FreezeTarget): number {
  return rendererReadyRevisionByTarget.get(target) ?? 0
}

/** Sends frozen-page state to the WebContentsView backing `target`. */
export function publish(target: FreezeTarget, state: FrozenPagesState): void {
  const view = target === 'above' ? aboveView : bgView
  if (!view || view.webContents.isDestroyed()) return
  safeSend(view.webContents, ipcChannels.frozenPagesState, state)
}

export function markRendererReady(target: FreezeTarget, revision: number): void {
  const current = Math.max(rendererReadyRevisionByTarget.get(target) ?? 0, revision)
  rendererReadyRevisionByTarget.set(target, current)
  const waiters = waitersFor(target)
  for (const [waitingRevision, resolve] of waiters) {
    if (waitingRevision > current) continue
    waiters.delete(waitingRevision)
    resolve(true)
  }
}

export function waitForRendererReady(
  target: FreezeTarget,
  waitingRevision: number,
  timeoutMs = 1_000,
): Promise<boolean> {
  if (readyRevision(target) >= waitingRevision) return Promise.resolve(true)
  return new Promise((resolve) => {
    const timeout = setTimeout(() => {
      waitersFor(target).delete(waitingRevision)
      resolve(false)
    }, timeoutMs)
    waitersFor(target).set(waitingRevision, (ready) => {
      clearTimeout(timeout)
      resolve(ready)
    })
  })
}

/**
 * Parking registry. A freeze registers the pages it has parked so the layout
 * engine can look up how to place any given page without knowing which
 * freeze (zoom, or a future drag freeze) owns it.
 */
export type PageParking = 'hidden' | 'warm' | null

interface FreezeRegistration {
  target: FreezeTarget
  pageIds: Set<string>
  parking: 'hidden' | 'warm'
}

const freezesById = new Map<string, FreezeRegistration>()

export function registerFreeze(
  id: string,
  options: { target: FreezeTarget; pageIds: string[]; parking: 'hidden' | 'warm' },
): void {
  freezesById.set(id, {
    target: options.target,
    pageIds: new Set(options.pageIds),
    parking: options.parking,
  })
}

export function updateFreezeParking(id: string, parking: 'hidden' | 'warm'): void {
  const freeze = freezesById.get(id)
  if (freeze) freeze.parking = parking
}

export function releaseFreeze(id: string): void {
  freezesById.delete(id)
}

/**
 * How the layout pass should place a parked page: `hidden` (zero bounds) for
 * a gesture body, `warm` (full size, off-screen, compositing) while the page
 * re-rasters at a settled scale before it is revealed. The first freeze that
 * claims a page wins.
 */
export function pageParkingFor(pageId: string): PageParking {
  for (const freeze of freezesById.values()) {
    if (freeze.pageIds.has(pageId)) return freeze.parking
  }
  return null
}

/** Identifies one page's content state: what a frame of it is a picture of. */
export function pageContentKey(page: Page): string {
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

/** Encodes a captured page image into a publishable frame. JPEG: PNG encode
 *  runs synchronously on this thread and costs 120 to 220ms per prepare at
 *  normal zooms; JPEG-85 is ~6x cheaper and decodes faster in the renderer
 *  (perf-zoom-pan-log Exp D). */
export function encodePageFrame(page: Page, image: NativeImage): FrozenPageFrame {
  const size = image.getSize()
  return {
    pageId: page.id,
    contentKey: pageContentKey(page),
    dataUrl: `data:image/jpeg;base64,${image.toJPEG(85).toString('base64')}`,
    capturedWidth: size.width,
    capturedHeight: size.height,
  }
}

/** Captures one page's on-screen surface as a frozen-page frame. Null for a
 *  destroyed view, zero bounds, or an empty compositor copy. */
export async function capturePageFrame(page: Page): Promise<FrozenPageFrame | null> {
  if (page.pageView.webContents.isDestroyed()) return null
  const bounds = page.pageView.getBounds()
  if (bounds.width <= 0 || bounds.height <= 0) return null
  const image = await page.pageView.webContents.capturePage()
  if (image.isEmpty()) return null
  return encodePageFrame(page, image)
}
