/**
 * Comment tool's page-paints contract + live-bbox round-trip (ADR 0006).
 *
 * Pointer state lifecycle:
 *
 *   renderer (above-view) ─send─▶ main (this module)
 *      ─throttle to ~60 Hz─▶ each page (intersected to page-local coords)
 *
 * The page paints outlines directly in its own DOM. When the comment tool
 * deactivates or the renderer reports a null state, every page receives an
 * `active: false` snapshot that clears its overlay.
 *
 * Live-bbox lifecycle:
 *
 *   renderer ─setAnnotationBboxSubscriptions(pageId, subs)─▶ main (this module)
 *      ─forward─▶ specific page
 *   page (on scroll/resize) ─annotation-bbox-update─▶ main
 *      ─fold into `annotationBboxes`─▶ runtime patch bus
 */

import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import type { AnnotationBboxReport, AnnotationBboxSubscription } from '../../shared/types'
import { pages } from '../runtime/page-runtime'
import { findPageByPageView } from '../runtime/runtime-context'
import { boundEffectivePageContentSize, boundScreenBoundsForPage } from '../runtime/runtime-geometry'
import { intersectRegionWithPage, pointerInPage } from '../runtime/comment-hover-math'
import { safeSend } from '../runtime/safe-send'
import {
  annotationLiveBboxes,
  applyAnnotationBboxReports,
  retainAnnotationBboxes,
} from '../runtime/annotation-bbox-state'
import { broadcastRuntimePatch } from '../runtime/runtime-patch-broadcast'

const POINTER_BROADCAST_INTERVAL_MS = 16

type PointerStateInput =
  | {
      windowX: number
      windowY: number
      regionRect: { x: number; y: number; width: number; height: number } | null
    }
  | null

let latestPointerState: PointerStateInput = null
let lastBroadcastAt = 0
let scheduledFlush: NodeJS.Timeout | null = null
let activeBroadcastInFlight = false
// Tracks whether we last broadcast an "active" frame to pages so we can emit
// a single trailing `active: false` clear when the tool deactivates.
let lastActiveFrame = false
// Per-page signature of the most recent payload sent. Suppresses no-op IPC
// when the renderer holds a static rect (composer open) or the pointer hasn't
// moved inside this page. Entries are evicted on webContents `destroyed`
// (see trackPageForDedup) so the map doesn't accumulate stale page ids over
// the lifetime of the session.
const lastPayloadByPage = new Map<string, string>()
const dedupCleanupBoundFor = new WeakSet<Electron.WebContents>()

function trackPageForDedup(pageId: string, webContents: Electron.WebContents): void {
  if (dedupCleanupBoundFor.has(webContents)) return
  dedupCleanupBoundFor.add(webContents)
  webContents.once('destroyed', () => {
    lastPayloadByPage.delete(pageId)
  })
}

function payloadSignature(payload: {
  active: boolean
  pointer: { x: number; y: number } | null
  regionRect: { x: number; y: number; width: number; height: number } | null
  displayScale?: number
}): string {
  const p = payload.pointer ? `${payload.pointer.x},${payload.pointer.y}` : '-'
  const r = payload.regionRect
    ? `${payload.regionRect.x},${payload.regionRect.y},${payload.regionRect.width},${payload.regionRect.height}`
    : '-'
  return `${payload.active ? '1' : '0'}|${p}|${r}|${payload.displayScale ?? 1}`
}

function sendIfChanged(
  pageId: string,
  webContents: Electron.WebContents,
  payload: {
    active: boolean
    pointer: { x: number; y: number } | null
    regionRect: { x: number; y: number; width: number; height: number } | null
    displayScale?: number
  },
): void {
  const sig = payloadSignature(payload)
  if (lastPayloadByPage.get(pageId) === sig) return
  lastPayloadByPage.set(pageId, sig)
  trackPageForDedup(pageId, webContents)
  safeSend(webContents, ipcChannels.commentToolPagePreview, payload)
}

function broadcastPointerState(state: PointerStateInput): void {
  if (!state) {
    if (!lastActiveFrame) return
    lastActiveFrame = false
    for (const page of pages) {
      if (page.pageView.webContents.isDestroyed()) continue
      sendIfChanged(page.id, page.pageView.webContents, {
        active: false,
        pointer: null,
        regionRect: null,
      })
    }
    return
  }

  lastActiveFrame = true
  for (const page of pages) {
    if (page.pageView.webContents.isDestroyed()) continue
    const screen = boundScreenBoundsForPage(page).page
    if (screen.width <= 0 || screen.height <= 0) {
      sendIfChanged(page.id, page.pageView.webContents, {
        active: true,
        pointer: null,
        regionRect: null,
      })
      continue
    }
    // Page is rendered at `displayZoom` (canvas zoom, or 1 in fill-browser
    // mode). The page's CSS coordinate space — what `elementFromPoint`
    // operates on — is the host rect divided by displayZoom. Derive the
    // ratio from the rendered rect so we don't need to know which mode
    // we're in.
    const cssWidth = boundEffectivePageContentSize(page).width
    const cssScale = cssWidth > 0 ? cssWidth / screen.width : 1
    const pointer = pointerInPage(state.windowX, state.windowY, screen, cssScale)
    const regionRect = state.regionRect
      ? intersectRegionWithPage(state.regionRect, screen, cssScale)
      : null
    sendIfChanged(page.id, page.pageView.webContents, {
      active: true,
      pointer,
      regionRect,
      // cssScale = pageCSSpx / onScreenPx = 1 / displayZoom — the factor that
      // counter-scales the in-page label back to a constant screen size.
      displayScale: cssScale,
    })
  }
}

function flushPointerState(): void {
  scheduledFlush = null
  if (activeBroadcastInFlight) return
  activeBroadcastInFlight = true
  try {
    broadcastPointerState(latestPointerState)
    lastBroadcastAt = Date.now()
  } finally {
    activeBroadcastInFlight = false
  }
}

function schedulePointerFlush(): void {
  if (scheduledFlush) return
  const elapsed = Date.now() - lastBroadcastAt
  const wait = Math.max(0, POINTER_BROADCAST_INTERVAL_MS - elapsed)
  scheduledFlush = setTimeout(flushPointerState, wait)
}

export function registerCommentHoverIpc(): void {
  ipcMain.on(
    ipcChannels.commentToolPointerState,
    (
      _event,
      payload:
        | {
            windowX?: number
            windowY?: number
            regionRect?: { x: number; y: number; width: number; height: number } | null
          }
        | null
        | undefined,
    ) => {
      if (
        !payload ||
        typeof payload.windowX !== 'number' ||
        typeof payload.windowY !== 'number'
      ) {
        latestPointerState = null
      } else {
        latestPointerState = {
          windowX: payload.windowX,
          windowY: payload.windowY,
          regionRect: payload.regionRect ?? null,
        }
      }
      schedulePointerFlush()
    },
  )

  ipcMain.on(
    ipcChannels.commentToolBboxSubscriptions,
    (
      _event,
      payload: { pageId?: string; subscriptions?: AnnotationBboxSubscription[] } | undefined,
    ) => {
      const pageId = typeof payload?.pageId === 'string' ? payload.pageId : null
      if (!pageId) return
      const subscriptions = Array.isArray(payload?.subscriptions) ? payload.subscriptions : []
      // The subscription set is also the retention set: a popover that closed
      // stops subscribing, and its last known bbox is state nothing can render.
      if (retainAnnotationBboxes(pageId, subscriptions.map((sub) => sub.annotationId))) {
        broadcastAnnotationBboxes()
      }
      const page = pages.find((candidate) => candidate.id === pageId)
      if (!page || page.pageView.webContents.isDestroyed()) return
      safeSend(page.pageView.webContents, ipcChannels.annotationBboxSubscriptions, {
        subscriptions,
      })
    },
  )

  ipcMain.on(
    ipcChannels.annotationBboxUpdate,
    (event, payload: { updates?: AnnotationBboxReport[] } | undefined) => {
      const page = findPageByPageView(event.sender)
      if (!page) return
      const updates = Array.isArray(payload?.updates) ? payload.updates : []
      if (!updates.length) return
      if (!applyAnnotationBboxReports(page.id, updates)) return
      broadcastAnnotationBboxes()
    },
  )
}

function broadcastAnnotationBboxes(): void {
  broadcastRuntimePatch({
    kind: 'slice',
    slice: 'annotationBboxes',
    value: annotationLiveBboxes(),
  })
}
