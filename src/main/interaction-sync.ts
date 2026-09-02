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
import type {
  InteractionSyncDragMoveEvent,
  InteractionSyncDragStartEvent,
  InteractionSyncEvent,
  InteractionSyncPointEvent,
  LocatorResolveResponse,
  PresenceLabelKey,
} from '../shared/types'
import type { LocatorBundle, LocatorCandidate, LocatorRect } from '../shared/locator-kernel'
import { dispatchPointForCandidate } from '../shared/locator-kernel'
import { ipcChannels } from '../shared/ipc-contract'
import { safeSend } from './runtime/safe-send'
import {
  type Page,
  pages,
  findPageById,
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
  removeAllSyncedCursors,
  removeSyncedCursorForPeer,
  setSyncedCursorLivenessProbe,
} from './presence-cursor'
import {
  type PeerDragSession,
  dispatchPeerHover,
  dispatchPeerClick,
  dispatchPeerDragStart,
  dispatchPeerDragMove,
  dispatchPeerDragEnd,
} from './cdp-peer-dispatch'

// One outstanding resolve request per peer PER KIND (D7). A fresh request of a
// kind supersedes the earlier one of that kind; a response is honoured only if
// its requestId still matches. Hover and click are tracked separately so a
// hover captured one frame after a click can't supersede the click's request
// and drop its response (a hover never dispatches an action, a click does).
interface PendingResolve {
  requestId: number
  kind: 'hover' | 'click' | 'drag'
  bundle: LocatorBundle
  viewportX: number
  viewportY: number
}

interface PeerPending {
  hover: PendingResolve | null
  click: PendingResolve | null
  drag: PendingResolve | null
}

/**
 * A peer's in-flight surface drag (ADR 0030, opaque-surface carve-out). The
 * surface is resolved once at drag-start and its rect latched here for the rest
 * of the gesture: every move maps the source's within-surface fraction onto
 * this rect with no further resolution, because inside an opaque surface there
 * is no sub-element the mapping could be wrong about.
 */
interface PeerDrag {
  /** The gesture this belongs to. An entry from an older gesture is stale —
   *  its late-arriving press must be released rather than continued. */
  generation: number
  rect: LocatorRect
  /** Latest offsets seen, applied as soon as the press lands (a drag that
   *  moves during the resolve round-trip must not start stale). */
  offsetX: number
  offsetY: number
  session: PeerDragSession | null
}

// The last confident resolution per peer, keyed by the bundle's identity
// (id/testId/fullPath). An offset-only hover over the SAME element reuses this
// rect to anchor the cursor and dispatch a hover in main — no resolve
// round-trip, no peer DOM walk (A9). Read for hover display only: clicks always
// re-resolve so "confident-or-skip" stays honest for actions (ADR 0030).
interface CachedResolution {
  identity: string
  candidate: LocatorCandidate
}

const pendingByPeer = new Map<string, PeerPending>()
const resolutionCacheByPeer = new Map<string, CachedResolution>()
const dragByPeer = new Map<string, PeerDrag>()
// Bumped on every drag-start and drag-end, so an async press that lands after
// its gesture ended can tell it is stale and release immediately.
let dragGeneration = 0
let dragActive = false
// Per-page current URL → origin, so the hot per-event origin gate parses each
// URL once. Self-validating: a navigation changes the URL and forces a
// recompute (an in-page nav keeps the origin, so a needless recompute there is
// harmless).
const originCache = new Map<string, { url: string; origin: string | null }>()
// Peers that currently carry a live synced cursor. Diffed on capture refresh to
// retire a peer that left the set (A2) — one peer leaving a 3+ set never
// changes the source identity, so nothing else catches it.
const syncedPeerIds = new Set<string>()
let resolveRequestSeq = 0

// The page whose captured input is currently mirrored (entered + synced), if
// any. Tracked so exiting/unsyncing/dissolving can retire its synced cursors,
// and so the presence idle sweep can tell "still capturing" from "gone" (A8).
let capturingSourcePageId: string | null = null

/** The peer's live URL origin, or null if it has no usable origin — an
 *  unparseable URL, or an opaque origin (file:, about:blank, data:) whose
 *  `origin` serializes to the literal string 'null'. Opaque documents must not
 *  all alias to one origin and cross-mirror, so they are skipped entirely (D3). */
function originOf(page: Page): string | null {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return null
  const url = wc.getURL()
  const cached = originCache.get(page.id)
  if (cached && cached.url === url) return cached.origin
  let origin: string | null
  try {
    const parsed = new URL(url).origin
    origin = parsed === 'null' ? null : parsed
  } catch {
    origin = null
  }
  originCache.set(page.id, { url, origin })
  return origin
}

function bundleName(bundle: LocatorBundle): string {
  return bundle.name ?? bundle.text ?? bundle.testId ?? bundle.id ?? bundle.tag
}

/** Gerund-voice label for the synced cursor, expressed in the shared presence
 *  vocabulary (labelKey + targetName) so the presence label vocabulary renders it
 *  as 'Clicking "X"' / 'Pointing at "X"'. */
const LABEL_KEY_BY_KIND: Record<'hover' | 'click' | 'drag', PresenceLabelKey> = {
  hover: 'point_target',
  click: 'click_target',
  drag: 'drag_target',
}

function labelInfo(
  kind: 'hover' | 'click' | 'drag',
  bundle: LocatorBundle | null,
): { labelKey: PresenceLabelKey | null; targetName: string | null } {
  if (!bundle) return { labelKey: null, targetName: null }
  return { labelKey: LABEL_KEY_BY_KIND[kind], targetName: bundleName(bundle) }
}

/** The identity a resolution cache entry is keyed by — the same keys the kernel
 *  treats as unique (id, then testId), falling back to the structural fullPath.
 *  Null when the bundle carries none (nothing stable to reuse a rect against). */
function bundleIdentity(bundle: LocatorBundle): string | null {
  if (bundle.id) return `id:${bundle.id}`
  if (bundle.testId) return `testId:${bundle.testId}`
  if (bundle.fullPath) return `path:${bundle.fullPath}`
  return null
}

function peerPending(peerId: string): PeerPending {
  let entry = pendingByPeer.get(peerId)
  if (!entry) {
    entry = { hover: null, click: null, drag: null }
    pendingByPeer.set(peerId, entry)
  }
  return entry
}

/** Release a peer's held button and forget the gesture. Every teardown path
 *  runs through here: a peer left pressed reads its next mirrored move as a
 *  continuing drag, which is worse than no sync at all. */
function endPeerDrag(peerId: string): void {
  const drag = dragByPeer.get(peerId)
  if (!drag) return
  dragByPeer.delete(peerId)
  const pending = pendingByPeer.get(peerId)
  if (pending) pending.drag = null
  if (!drag.session) return
  const peer = findPageById(peerId)
  if (peer) void dispatchPeerDragEnd(peer, drag.session)
}

/** End the whole gesture across every peer. */
function endAllPeerDrags(): void {
  dragActive = false
  dragGeneration += 1
  for (const peerId of [...dragByPeer.keys()]) endPeerDrag(peerId)
}

function retirePeer(peerId: string): void {
  endPeerDrag(peerId)
  removeSyncedCursorForPeer(peerId)
  pendingByPeer.delete(peerId)
  resolutionCacheByPeer.delete(peerId)
  syncedPeerIds.delete(peerId)
}

function retireSource(): void {
  endAllPeerDrags()
  removeAllSyncedCursors()
  pendingByPeer.clear()
  resolutionCacheByPeer.clear()
  syncedPeerIds.clear()
}

/** Reuse a cached confident rect for an offset-only hover over the same element
 *  (A9): anchor the cursor and dispatch a trusted hover with no resolve
 *  round-trip. Returns false (fall through to a fresh resolve) on a cache miss
 *  or a bundle with no stable identity. */
function tryCachedHover(
  peer: Page,
  bundle: LocatorBundle,
  viewportX: number,
  viewportY: number,
): boolean {
  const identity = bundleIdentity(bundle)
  if (!identity) return false
  const cached = resolutionCacheByPeer.get(peer.id)
  if (!cached || cached.identity !== identity) return false

  const { rect } = cached.candidate
  const { labelKey, targetName } = labelInfo('hover', bundle)
  upsertSyncedCursor({
    peerPageId: peer.id,
    position: { viewportX, viewportY, anchor: { rect, offsetX: bundle.offsetX, offsetY: bundle.offsetY } },
    labelKey,
    targetName,
  })
  void dispatchPeerHover(peer, dispatchPointForCandidate(rect, bundle.offsetX, bundle.offsetY))
  return true
}

/**
 * Recompute and resend the per-page capture flag (D1): a page captures iff it
 * is the user-entered page AND it has a live sync peer. This is the single
 * chokepoint for every interactive/sync-membership transition — reached via
 * `sendInteractiveState` (enter/exit/focus/selection/undo/page-delete) and
 * `dissolveOrphanSyncSets` (sync set/unset). When the capturing source changes,
 * the previous source's synced cursors are retired wholesale; when it stays but
 * a single peer has left the set, only that peer is retired (A2).
 */
export function refreshInteractionSyncCapture(): void {
  ensureLivenessProbe()
  const enteredId = interactivePageId()
  let nextSource: Page | null = null
  for (const page of pages) {
    const enabled = enteredId === page.id && isPageSynced(page)
    if (enabled) nextSource = page
    safeSend(page.pageView.webContents, ipcChannels.setInteractionSyncCapture, { enabled })
  }

  if (!nextSource || nextSource.id !== capturingSourcePageId) {
    if (capturingSourcePageId) retireSource()
    capturingSourcePageId = nextSource?.id ?? null
    return
  }

  // Same source still capturing: retire any peer that has dropped out of the
  // set (unsync or close of one peer in a 3+ set) — its cursor and per-peer
  // bookkeeping would otherwise ghost until the idle sweep.
  const currentPeerIds = new Set(syncPeersOf(nextSource).map((peer) => peer.id))
  for (const peerId of [...syncedPeerIds]) {
    if (!currentPeerIds.has(peerId)) retirePeer(peerId)
  }
}

/** Drop the resolution cache (and any outstanding resolve) for a page that has
 *  navigated — its element rects are stale and identity keys may no longer
 *  resolve. Called from the page's did-navigate handler. */
export function invalidateInteractionSyncResolution(pageId: string): void {
  endPeerDrag(pageId)
  resolutionCacheByPeer.delete(pageId)
  originCache.delete(pageId)
}

/**
 * Relay input captured on the source page to its same-origin peers. Drops
 * events unless the sender is the user-entered page and is not currently driven
 * by agent automation (D1). Hover/click take the semantic path (resolve, then
 * confident-or-skip); a surface drag takes the opaque-surface path (resolve the
 * surface once at drag-start, then replay the gesture against that latched
 * rect). A drag-end must survive every gate that a drag-start passed, or a peer
 * is left holding a pressed button — so it is handled before the origin gate,
 * whose answer can change mid-gesture if the peer navigates.
 */
export function handleInteractionSyncEvent(
  sender: WebContents,
  event: InteractionSyncEvent,
): void {
  const source = findPageByPageView(sender)
  if (!source) return
  if (interactivePageId() !== source.id) return
  if (automationInteractivePageCounts.has(source.id)) return

  if (event.kind === 'drag-move') {
    relayDragMove(event)
    return
  }
  if (event.kind === 'drag-end') {
    endAllPeerDrags()
    return
  }

  const sourceOrigin = originOf(source)
  if (!sourceOrigin) return

  if (event.kind === 'drag-start') {
    relayDragStart(source, sourceOrigin, event)
    return
  }
  relayPointEvent(source, sourceOrigin, event)
}

function relayPointEvent(
  source: Page,
  sourceOrigin: string,
  event: InteractionSyncPointEvent,
): void {
  const { labelKey, targetName } = labelInfo(event.kind, event.bundle)
  for (const peer of syncPeersOf(source)) {
    // The agent owns input on a page it is driving; mirroring trusted input
    // into it would interleave with the automation (A6).
    if (automationInteractivePageCounts.has(peer.id)) continue
    if (originOf(peer) !== sourceOrigin) continue

    syncedPeerIds.add(peer.id)
    upsertSyncedCursor({
      peerPageId: peer.id,
      position: { viewportX: event.viewportX, viewportY: event.viewportY, anchor: null },
      labelKey,
      targetName,
    })

    if (!event.bundle) {
      // Cursor moved off any element: drop the peer's outstanding hover resolve
      // (A4) and its cached anchor (A9) so a late/stale response — or a reused
      // rect — can't re-anchor where the source no longer points. A pending
      // click is untouched (D4).
      peerPending(peer.id).hover = null
      resolutionCacheByPeer.delete(peer.id)
      continue
    }

    if (event.kind === 'hover' && tryCachedHover(peer, event.bundle, event.viewportX, event.viewportY)) {
      // Served from cache — no round-trip, so nothing is left pending.
      peerPending(peer.id).hover = null
      continue
    }

    const requestId = ++resolveRequestSeq
    const pending: PendingResolve = {
      requestId,
      kind: event.kind,
      bundle: event.bundle,
      viewportX: event.viewportX,
      viewportY: event.viewportY,
    }
    const slots = peerPending(peer.id)
    if (event.kind === 'click') {
      // A newer click supersedes anything: also drop a stale hover so its late
      // response can't dispatch a mouseMoved after the click lands.
      slots.click = pending
      slots.hover = null
    } else {
      slots.hover = pending
    }
    safeSend(peer.pageView.webContents, ipcChannels.resolveInteractionLocator, {
      requestId,
      bundle: event.bundle,
    })
  }
}

/**
 * Open a surface drag: ask every eligible peer to resolve the surface, exactly
 * once. Confidence is decided here, before the gesture starts — a peer that
 * cannot single out the surface never gets a press, so the gesture is either
 * mirrored whole or not at all rather than half-landing mid-stream.
 */
function relayDragStart(
  source: Page,
  sourceOrigin: string,
  event: InteractionSyncDragStartEvent,
): void {
  // A prior gesture that never saw its release (a peer torn down mid-drag)
  // must not outlive this one.
  endAllPeerDrags()
  dragActive = true

  const { labelKey, targetName } = labelInfo('drag', event.bundle)
  for (const peer of syncPeersOf(source)) {
    if (automationInteractivePageCounts.has(peer.id)) continue
    if (originOf(peer) !== sourceOrigin) continue

    syncedPeerIds.add(peer.id)
    upsertSyncedCursor({
      peerPageId: peer.id,
      position: { viewportX: event.viewportX, viewportY: event.viewportY, anchor: null },
      labelKey,
      targetName,
    })

    const requestId = ++resolveRequestSeq
    const slots = peerPending(peer.id)
    // The gesture owns this peer now: a hover or click resolved mid-drag would
    // dispatch into the middle of it.
    slots.hover = null
    slots.click = null
    slots.drag = {
      requestId,
      kind: 'drag',
      bundle: event.bundle,
      viewportX: event.viewportX,
      viewportY: event.viewportY,
    }
    safeSend(peer.pageView.webContents, ipcChannels.resolveInteractionLocator, {
      requestId,
      bundle: event.bundle,
    })
  }
}

/**
 * Continue the gesture on every peer already pressed. The move maps the
 * source's within-surface fraction onto the peer's latched rect — no resolve,
 * no DOM walk — so a pan tracks at pointer speed and a peer at a different
 * breakpoint pans the same *fraction* of its own surface.
 */
function relayDragMove(event: InteractionSyncDragMoveEvent): void {
  for (const [peerId, drag] of dragByPeer) {
    drag.offsetX = event.offsetX
    drag.offsetY = event.offsetY
    if (!drag.session) continue
    const peer = findPageById(peerId)
    if (!peer) continue
    const point = dispatchPointForCandidate(drag.rect, event.offsetX, event.offsetY)
    upsertSyncedCursor({
      peerPageId: peerId,
      position: {
        viewportX: event.viewportX,
        viewportY: event.viewportY,
        anchor: { rect: drag.rect, offsetX: event.offsetX, offsetY: event.offsetY },
      },
      labelKey: 'drag_target',
      targetName: null,
    })
    void dispatchPeerDragMove(peer, drag.session, point)
  }
}

/**
 * Press the button on a peer whose surface resolved confidently. The press is
 * async, so the gesture may already have ended by the time it lands: the entry
 * is generation-stamped and a stale one is released immediately rather than
 * left pressed forever.
 */
async function beginPeerDrag(peer: Page, drag: PeerDrag, point: { x: number; y: number }): Promise<void> {
  const session = await dispatchPeerDragStart(peer, point)
  if (!session) {
    dragByPeer.delete(peer.id)
    return
  }
  if (!dragActive || drag.generation !== dragGeneration || dragByPeer.get(peer.id) !== drag) {
    void dispatchPeerDragEnd(peer, session)
    dragByPeer.delete(peer.id)
    return
  }
  drag.session = session
  // The pointer kept moving during the resolve round-trip; catch the peer up so
  // the gesture doesn't start from a stale point.
  const current = dispatchPointForCandidate(drag.rect, drag.offsetX, drag.offsetY)
  if (current.x !== point.x || current.y !== point.y) {
    void dispatchPeerDragMove(peer, session, current)
  }
}

/**
 * Apply a peer's resolution. Correlated by requestId against the matching kind's
 * pending slot; a superseded/stale response finds no slot and is dropped (D7).
 * Confident → cache the rect, anchor the synced cursor, and replay trusted input
 * (hover mouseMoved, click press+release, or the opening press of a surface
 * drag) at the peer's own point (D4/D5). Ambiguous/none → the cursor stays
 * proportional and the stale cache is dropped; a refused click or drag wiggles.
 */
export function handleResolveInteractionLocatorResponse(
  sender: WebContents,
  response: LocatorResolveResponse,
): void {
  const peer = findPageByPageView(sender)
  if (!peer) return
  if (automationInteractivePageCounts.has(peer.id)) return

  const slots = pendingByPeer.get(peer.id)
  if (!slots) return
  let pending: PendingResolve | null = null
  if (slots.hover?.requestId === response.requestId) {
    pending = slots.hover
    slots.hover = null
  } else if (slots.click?.requestId === response.requestId) {
    pending = slots.click
    slots.click = null
  } else if (slots.drag?.requestId === response.requestId) {
    pending = slots.drag
    slots.drag = null
  }
  if (!pending) return

  const { resolution } = response
  if (pending.kind === 'drag') {
    // The gesture may have ended while the peer was resolving; a press now
    // would never be released by anything but teardown.
    if (!dragActive) return
    if (resolution.kind !== 'confident') {
      // Refused before a single press: nothing to undo, and the refusal reads
      // like any other — the cursor wiggles and stays proportional.
      wiggleSyncedCursor(peer.id)
      return
    }
    const drag: PeerDrag = {
      generation: dragGeneration,
      rect: resolution.candidate.rect,
      offsetX: pending.bundle.offsetX,
      offsetY: pending.bundle.offsetY,
      session: null,
    }
    dragByPeer.set(peer.id, drag)
    upsertSyncedCursor({
      peerPageId: peer.id,
      position: {
        viewportX: pending.viewportX,
        viewportY: pending.viewportY,
        anchor: {
          rect: drag.rect,
          offsetX: pending.bundle.offsetX,
          offsetY: pending.bundle.offsetY,
        },
      },
      labelKey: 'drag_target',
      targetName: bundleName(pending.bundle),
    })
    void beginPeerDrag(peer, drag, resolution.point)
    return
  }

  if (resolution.kind === 'confident') {
    const identity = bundleIdentity(pending.bundle)
    if (identity) {
      resolutionCacheByPeer.set(peer.id, { identity, candidate: resolution.candidate })
    }
    const { labelKey, targetName } = labelInfo(pending.kind, pending.bundle)
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
      labelKey,
      targetName,
    })
    if (pending.kind === 'hover') void dispatchPeerHover(peer, resolution.point)
    else void dispatchPeerClick(peer, resolution.point)
    return
  }

  // Ambiguous or unmatched: the proportional cursor (set when the request went
  // out) stands, and any prior confident rect is now suspect — drop it so it
  // can't re-anchor a later hover. A refused click wiggles; a refused hover
  // does nothing.
  resolutionCacheByPeer.delete(peer.id)
  if (pending.kind === 'click') wiggleSyncedCursor(peer.id)
}

// The presence idle sweep treats a synced cursor as alive whenever a source is
// still capturing (A8) — a still mouse over a tooltip sends no events but must
// not depart. Registered on first capture refresh rather than at import time:
// presence-cursor reaches this module through a cycle, so an import-time call
// would hit its module bindings before they initialize.
let livenessProbeRegistered = false
function ensureLivenessProbe(): void {
  if (livenessProbeRegistered) return
  livenessProbeRegistered = true
  setSyncedCursorLivenessProbe(() => capturingSourcePageId !== null)
}
