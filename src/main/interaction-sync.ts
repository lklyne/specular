// fallow-ignore-file circular-dependencies
// Suppressed: navigation-sync's dissolveOrphanSyncSets calls back into
// refreshInteractionSyncCapture, and this relay reads syncPeersOf/isPageSynced.
// The cycle is call-time only (no module-eval use), matching navigation-sync.
//
// Interaction sync relay (ADR 0030, module 3). Main stays a dumb relay: no DOM
// knowledge, no scoring (the kernel runs in each peer's preload), no persisted
// state (D10). It fans a source page's captured hover/click out to same-origin
// peers, drives each peer's synced cursor, and — only for confident peer
// resolutions — replays trusted input via the CDP dispatcher.
import type { WebContents } from 'electron'
import type { InteractionSyncEvent, LocatorResolveResponse } from '../shared/types'
import type { LocatorBundle } from '../shared/locator-kernel'
import { ipcChannels } from '../shared/ipc-contract'
import { safeSend } from './runtime/safe-send'
import {
  type Page,
  pages,
  findPageByPageView,
} from './runtime/page-runtime'
import {
  interactivePageId,
  automationInteractivePageCounts,
} from './runtime/runtime-context'
import { syncPeersOf, isPageSynced } from './navigation-sync'
import {
  upsertSyncedCursor,
  wiggleSyncedCursor,
  removeSyncedCursorsForSource,
} from './presence-cursor'
import { dispatchPeerHover, dispatchPeerClick } from './cdp-peer-dispatch'

// One outstanding resolve request per peer (D7): a fresh request supersedes any
// earlier one, and a response is honoured only if its requestId still matches.
interface PendingResolve {
  requestId: number
  sourcePageId: string
  kind: 'hover' | 'click'
  bundle: LocatorBundle
  viewportX: number
  viewportY: number
}

const pendingByPeer = new Map<string, PendingResolve>()
let resolveRequestSeq = 0

// The page whose captured input is currently mirrored (entered + synced), if
// any. Tracked so exiting/unsyncing/dissolving can retire its synced cursors.
let capturingSourcePageId: string | null = null

/** The peer's live URL origin, or null if it has no parseable origin
 *  (about:blank, file:, invalid) — such peers are skipped entirely (D3). */
function originOf(page: Page): string | null {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return null
  try {
    return new URL(wc.getURL()).origin
  } catch {
    return null
  }
}

function bundleName(bundle: LocatorBundle): string {
  return bundle.name ?? bundle.text ?? bundle.testId ?? bundle.id ?? bundle.tag
}

/** Gerund-voice synced-cursor label from the bundle's best human name. */
function labelFor(kind: 'hover' | 'click', bundle: LocatorBundle | null): string {
  if (!bundle) return ''
  const name = bundleName(bundle)
  return kind === 'click' ? `clicking ${name}` : `pointing at ${name}`
}

function retireSource(sourcePageId: string): void {
  removeSyncedCursorsForSource(sourcePageId)
  for (const [peerId, pending] of pendingByPeer) {
    if (pending.sourcePageId === sourcePageId) pendingByPeer.delete(peerId)
  }
}

/**
 * Recompute and resend the per-page capture flag (D1): a page captures iff it
 * is the user-entered page AND it has a live sync peer. Call on
 * enter/exit-interactive, sync-membership changes, and page (re)loads. When the
 * capturing source changes, the previous source's synced cursors are retired.
 */
export function refreshInteractionSyncCapture(): void {
  const enteredId = interactivePageId()
  let nextSource: string | null = null
  for (const page of pages) {
    const enabled = enteredId === page.id && isPageSynced(page)
    if (enabled) nextSource = page.id
    safeSend(page.pageView.webContents, ipcChannels.setInteractionSyncCapture, { enabled })
  }
  if (capturingSourcePageId && capturingSourcePageId !== nextSource) {
    retireSource(capturingSourcePageId)
  }
  capturingSourcePageId = nextSource
}

/**
 * Relay a hover/click captured on the source page to its same-origin peers.
 * Drops events unless the sender is the user-entered page and is not currently
 * driven by agent automation (D1). Every eligible peer's synced cursor gets the
 * proportional base; peers additionally resolve the bundle (if any) to decide
 * whether to anchor + replay.
 */
export function handleInteractionSyncEvent(
  sender: WebContents,
  event: InteractionSyncEvent,
): void {
  const source = findPageByPageView(sender)
  if (!source) return
  if (interactivePageId() !== source.id) return
  if (automationInteractivePageCounts.has(source.id)) return

  const sourceOrigin = originOf(source)
  if (!sourceOrigin) return

  const label = labelFor(event.kind, event.bundle)
  for (const peer of syncPeersOf(source)) {
    if (originOf(peer) !== sourceOrigin) continue

    upsertSyncedCursor({
      peerPageId: peer.id,
      position: { viewportX: event.viewportX, viewportY: event.viewportY, anchor: null },
      label,
    })

    if (!event.bundle) continue
    const requestId = ++resolveRequestSeq
    pendingByPeer.set(peer.id, {
      requestId,
      sourcePageId: source.id,
      kind: event.kind,
      bundle: event.bundle,
      viewportX: event.viewportX,
      viewportY: event.viewportY,
    })
    safeSend(peer.pageView.webContents, ipcChannels.resolveInteractionLocator, {
      requestId,
      bundle: event.bundle,
    })
  }
}

/**
 * Apply a peer's resolution. Correlated by requestId; a superseded response is
 * dropped (D7). Confident → anchor the synced cursor and replay trusted input
 * (hover mouseMoved, or click press+release) at the peer's own point (D4/D5).
 * Ambiguous/none → the cursor stays proportional; a refused click wiggles.
 */
export function handleResolveInteractionLocatorResponse(
  sender: WebContents,
  response: LocatorResolveResponse,
): void {
  const peer = findPageByPageView(sender)
  if (!peer) return
  const pending = pendingByPeer.get(peer.id)
  if (!pending || pending.requestId !== response.requestId) return
  pendingByPeer.delete(peer.id)

  const { resolution } = response
  if (resolution.kind === 'confident') {
    upsertSyncedCursor({
      peerPageId: peer.id,
      position: {
        viewportX: pending.viewportX,
        viewportY: pending.viewportY,
        anchor: {
          rect: resolution.candidate.rect,
          offsetX: pending.bundle.offsetX,
          offsetY: pending.bundle.offsetY,
        },
      },
      label: labelFor(pending.kind, pending.bundle),
    })
    if (pending.kind === 'hover') void dispatchPeerHover(peer, resolution.point)
    else void dispatchPeerClick(peer, resolution.point)
    return
  }

  // Ambiguous or unmatched: the proportional cursor (set when the request went
  // out) stands. A refused click wiggles; a refused hover does nothing.
  if (pending.kind === 'click') wiggleSyncedCursor(peer.id)
}
