/**
 * Interaction sync relay against the real runtime, in-process (ADR 0030).
 *
 * The relay in src/main/interaction-sync.ts fans a source page's captured
 * hover/click out to same-origin sync peers: it drives each peer's synced
 * cursor, round-trips a locator resolution, and — only for confident peer
 * resolutions — replays trusted input via the CDP dispatcher. These tests
 * assert the EXTERNAL effects a peer would observe (synced-cursor presence
 * state, resolve requests sent to the peer, Input.dispatchMouseEvent on the
 * peer's debugger), never the relay's internal call order.
 *
 * The final block covers the ADR's required D5 scenario — "one click on a link
 * in a 3-page set produces exactly one navigation per peer" — by driving the
 * real navigation-sync machinery (did-navigate → propagateNavigationFromPage,
 * suppression windows, URL-equality guards) in both race orderings.
 *
 * Mutation-verified by:
 *  - dropping the source-authority gate (`interactivePageId() !== source.id`
 *    early return in handleInteractionSyncEvent) — the non-interactive-sender
 *    test then mirrors an event it should have ignored.
 *  - dropping the origin gate (`originOf(peer) !== sourceOrigin`) — the
 *    cross-origin peer then gets a cursor + resolve request.
 *  - making `wiggleSyncedCursor` a no-op — the ambiguous-click test no longer
 *    sees the 'refused' activity.
 *  - dropping the requestId staleness check in
 *    handleResolveInteractionLocatorResponse — the stale response then
 *    dispatches.
 *  - making `removeAllSyncedCursors` a no-op — the unsync test then still
 *    finds a synced cursor after the set dissolves.
 *  - removing the `currentUrl === action.url` early return in
 *    applyNavigationAction (navigation-sync.ts) — the D5 "peer navigates first"
 *    ordering then double-navigates the source.
 *  - dropping the opaque-origin guard (`parsed === 'null' ? null`) in originOf —
 *    the two file: peers alias and the event mirrors.
 *  - dropping the peer automation `continue` in handleInteractionSyncEvent — the
 *    agent-driven peer then gets a cursor + resolve request.
 *  - collapsing the per-kind pending slots to one (a hover overwriting the click
 *    slot) — the click's confident answer is then dropped, dispatching nothing.
 *  - dropping the null-bundle `slots.hover = null` — the stale hover answer then
 *    dispatches a mouseMoved after the source left the element.
 *  - dropping the per-peer retire loop in refreshInteractionSyncCapture — the
 *    unsynced peer's cursor survives in the 3-page set.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import {
  findPageById,
  addAutomationInteractivePageId,
  clearAutomationInteractivePageIds,
} from '../../src/main/runtime/runtime-context'
import { setSyncForSelection, unsyncPage } from '../../src/main/navigation-sync'
import {
  enterPageInteractive,
  exitPageInteractive,
} from '../../src/main/runtime/overlay-manager'
import {
  handleInteractionSyncEvent,
  handleResolveInteractionLocatorResponse,
} from '../../src/main/interaction-sync'
import {
  getPresenceCursors,
  removeAllSyncedCursors,
} from '../../src/main/presence-cursor'
import { ipcChannels } from '../../src/shared/ipc-contract'
import type { LocatorBundle } from '../../src/shared/locator-kernel'
import type { InteractionSyncEvent, LocatorResolveResponse } from '../../src/shared/types'

let harness: WorkspaceHarness

const SAME_ORIGIN = 'https://site.test'
const OTHER_ORIGIN = 'https://other.test'

const BUNDLE: LocatorBundle = {
  id: 'cta',
  tag: 'button',
  name: 'Sign up',
  text: 'Sign up',
  elementPath: 'button#cta',
  fullPath: 'body>main>button#cta',
  offsetX: 0.5,
  offsetY: 0.5,
}

function makePage(url: string, x: number): string {
  const result = applyCanvasPatch({
    entities: [{ kind: 'page', url, canvasX: x, canvasY: 0, presetIndex: 2 }],
  })
  return result.created[0]
}

function wc(id: string) {
  return findPageById(id)!.pageView.webContents as unknown as {
    id: number
    loadURL(url: string): Promise<void>
    getURL(): string
    emit(event: string, ...args: unknown[]): boolean
    loadedUrls: string[]
    debuggerCommands: Array<{ method: string; params: unknown }>
  }
}

function clickEvent(bundle: LocatorBundle | null = BUNDLE): InteractionSyncEvent {
  return { kind: 'click', bundle, viewportX: 0.5, viewportY: 0.5 }
}

function hoverEvent(bundle: LocatorBundle | null = BUNDLE): InteractionSyncEvent {
  return { kind: 'hover', bundle, viewportX: 0.25, viewportY: 0.75 }
}

function syncedCursors() {
  return getPresenceCursors().filter((c) => c.source === 'interaction-sync')
}

function syncedCursorFor(peerId: string) {
  return syncedCursors().find((c) => c.pageId === peerId)
}

function resolveRequestsTo(peerId: string) {
  return harness.broadcasts.filter(
    (b) => b.channel === ipcChannels.resolveInteractionLocator && b.webContentsId === wc(peerId).id,
  )
}

function requestIdFor(peerId: string): number {
  const req = resolveRequestsTo(peerId).at(-1)
  return (req!.args[0] as { requestId: number }).requestId
}

function requestIdAt(peerId: string, index: number): number {
  const req = resolveRequestsTo(peerId).at(index)
  return (req!.args[0] as { requestId: number }).requestId
}

function dispatchesOn(peerId: string) {
  return wc(peerId).debuggerCommands.filter((c) => c.method === 'Input.dispatchMouseEvent')
}

function confidentResponse(
  requestId: number,
  point = { x: 100, y: 60 },
  rect = { x: 80, y: 40, width: 120, height: 40 },
): LocatorResolveResponse {
  return {
    requestId,
    resolution: {
      kind: 'confident',
      candidate: {
        id: 'cta',
        testId: null,
        role: null,
        name: 'Sign up',
        text: 'Sign up',
        tag: 'button',
        elementPath: 'button#cta',
        fullPath: 'body>main>button#cta',
        interactive: true,
        rect,
      },
      point,
    },
  }
}

function ambiguousResponse(requestId: number): LocatorResolveResponse {
  return { requestId, resolution: { kind: 'ambiguous' } }
}

/** Create a synced set at the given URLs and return their ids; the first is the
 *  source. Does NOT enter interactive — callers that need capture authority
 *  call enterPageInteractive on the source. */
async function makeSyncedSet(urls: string[]): Promise<string[]> {
  const ids = urls.map((url, i) => makePage(url, i * 500))
  setSyncForSelection(ids)
  await settleSync()
  return ids
}

describe('interaction sync relay', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    clearAutomationInteractivePageIds()
    exitPageInteractive()
    // Belt-and-braces: retire any synced cursors a prior test left behind.
    removeAllSyncedCursors()
    harness.clearBroadcasts()
  })

  afterAll(() => harness?.dispose())

  describe('source-authority gating (D1)', () => {
    it('ignores an event whose sender is not the interactive page', async () => {
      const [a, b] = await makeSyncedSet([`${SAME_ORIGIN}/a`, `${SAME_ORIGIN}/b`])
      enterPageInteractive(a)
      harness.clearBroadcasts()

      // b is a peer, not the entered page — its captured input must not mirror.
      handleInteractionSyncEvent(findPageById(b)!.pageView.webContents as never, clickEvent())

      expect(syncedCursors()).toHaveLength(0)
      expect(resolveRequestsTo(a)).toHaveLength(0)
      expect(dispatchesOn(a)).toHaveLength(0)
    })

    it('ignores an event from a page under agent automation even when entered', async () => {
      const [a, b] = await makeSyncedSet([`${SAME_ORIGIN}/a`, `${SAME_ORIGIN}/b`])
      enterPageInteractive(a)
      // An agent is driving the entered page: its CDP traffic must not mirror.
      addAutomationInteractivePageId(a)
      harness.clearBroadcasts()

      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, clickEvent())

      expect(syncedCursors()).toHaveLength(0)
      expect(resolveRequestsTo(b)).toHaveLength(0)
      expect(dispatchesOn(b)).toHaveLength(0)
    })
  })

  describe('origin gating (D3)', () => {
    it('mirrors to a same-origin peer but skips a different-origin peer', async () => {
      const [a, sameOrigin, crossOrigin] = await makeSyncedSet([
        `${SAME_ORIGIN}/a`,
        `${SAME_ORIGIN}/b`,
        `${OTHER_ORIGIN}/c`,
      ])
      enterPageInteractive(a)
      harness.clearBroadcasts()

      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())

      expect(syncedCursorFor(sameOrigin)).toBeDefined()
      expect(resolveRequestsTo(sameOrigin)).toHaveLength(1)

      expect(syncedCursorFor(crossOrigin)).toBeUndefined()
      expect(resolveRequestsTo(crossOrigin)).toHaveLength(0)
      expect(dispatchesOn(crossOrigin)).toHaveLength(0)
    })

    it('drops the whole event when the source has an opaque origin', async () => {
      // Two different file: documents both serialize their origin to the literal
      // string 'null'. Aliasing them would cross-mirror unrelated pages, so an
      // opaque source origin is treated as no-origin and the event is dropped.
      const [a, b] = await makeSyncedSet(['file:///a.html', 'file:///b.html'])
      enterPageInteractive(a)
      harness.clearBroadcasts()

      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())

      expect(syncedCursors()).toHaveLength(0)
      expect(resolveRequestsTo(b)).toHaveLength(0)
      expect(dispatchesOn(b)).toHaveLength(0)
    })
  })

  describe('peer automation gating (A6)', () => {
    it('skips a peer currently driven by agent automation', async () => {
      const [a, driven, free] = await makeSyncedSet([
        `${SAME_ORIGIN}/a`,
        `${SAME_ORIGIN}/b`,
        `${SAME_ORIGIN}/c`,
      ])
      enterPageInteractive(a)
      // An agent owns input on `driven`; mirrored trusted input must not
      // interleave with it.
      addAutomationInteractivePageId(driven)
      harness.clearBroadcasts()

      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())

      expect(syncedCursorFor(driven)).toBeUndefined()
      expect(resolveRequestsTo(driven)).toHaveLength(0)
      expect(dispatchesOn(driven)).toHaveLength(0)
      // The un-driven same-origin peer still mirrors.
      expect(syncedCursorFor(free)).toBeDefined()
      expect(resolveRequestsTo(free)).toHaveLength(1)
    })
  })

  describe('kind-aware pending (A3/A4)', () => {
    it('a later hover does not supersede an outstanding click', async () => {
      const [a, b] = await makeSyncedSet([`${SAME_ORIGIN}/a`, `${SAME_ORIGIN}/b`])
      enterPageInteractive(a)
      harness.clearBroadcasts()

      // Click, then a hover one frame later — the hover must not clobber the
      // click's pending resolve.
      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, clickEvent())
      const clickRequestId = requestIdAt(b, 0)
      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())
      const hoverRequestId = requestIdAt(b, 1)
      expect(hoverRequestId).not.toBe(clickRequestId)

      // The click's confident answer still dispatches the trusted press+release.
      handleResolveInteractionLocatorResponse(
        findPageById(b)!.pageView.webContents as never,
        confidentResponse(clickRequestId),
      )
      await settleSync()
      expect(dispatchesOn(b)).toHaveLength(2)
    })

    it('a null-bundle hover invalidates the peer’s pending hover', async () => {
      const [a, b] = await makeSyncedSet([`${SAME_ORIGIN}/a`, `${SAME_ORIGIN}/b`])
      enterPageInteractive(a)
      harness.clearBroadcasts()

      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())
      const staleHoverId = requestIdFor(b)

      // The cursor leaves every element before the peer answers.
      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent(null))
      // Proportional again — no anchor.
      expect(syncedCursorFor(b)?.targetRect).toBeNull()

      // The now-stale confident answer must not re-anchor or dispatch.
      handleResolveInteractionLocatorResponse(
        findPageById(b)!.pageView.webContents as never,
        confidentResponse(staleHoverId),
      )
      await settleSync()
      expect(dispatchesOn(b)).toHaveLength(0)
    })
  })

  describe('per-peer retire (A2)', () => {
    it('retires only the unsynced peer when one leaves a 3-page set', async () => {
      const [a, b, c] = await makeSyncedSet([
        `${SAME_ORIGIN}/a`,
        `${SAME_ORIGIN}/b`,
        `${SAME_ORIGIN}/c`,
      ])
      enterPageInteractive(a)
      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())
      expect(syncedCursorFor(b)).toBeDefined()
      expect(syncedCursorFor(c)).toBeDefined()

      // c leaves; a and b remain a valid 2-page set, so only c's cursor and
      // per-peer state should be retired.
      unsyncPage(c)
      await settleSync()
      expect(syncedCursorFor(c)).toBeUndefined()
      expect(syncedCursorFor(b)).toBeDefined()

      harness.clearBroadcasts()
      // Further input mirrors to b only.
      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())
      expect(syncedCursorFor(b)).toBeDefined()
      expect(syncedCursorFor(c)).toBeUndefined()
      expect(resolveRequestsTo(c)).toHaveLength(0)
    })
  })

  describe('confident vs ambiguous click (D2/D4)', () => {
    it('replays a confident click as trusted input at the resolved point', async () => {
      const [a, b] = await makeSyncedSet([`${SAME_ORIGIN}/a`, `${SAME_ORIGIN}/b`])
      enterPageInteractive(a)
      harness.clearBroadcasts()

      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, clickEvent())
      const requestId = requestIdFor(b)

      handleResolveInteractionLocatorResponse(
        findPageById(b)!.pageView.webContents as never,
        confidentResponse(requestId, { x: 100, y: 60 }),
      )
      // The press+release pair is dispatched on the async CDP path; let it flush.
      await settleSync()

      const dispatches = dispatchesOn(b)
      // A trusted click is a press+release pair (D8) at the peer's own point.
      expect(dispatches).toHaveLength(2)
      for (const d of dispatches) {
        expect(d.params).toMatchObject({ x: 100, y: 60, button: 'left' })
      }
      // Confident resolution anchors the cursor to the resolved rect (the halo
      // tell), so the peer's synced cursor now carries a targetRect.
      expect(syncedCursorFor(b)?.targetRect).toEqual({ x: 80, y: 40, width: 120, height: 40 })
    })

    it('refuses an ambiguous click: no dispatch, cursor wiggles', async () => {
      const [a, b] = await makeSyncedSet([`${SAME_ORIGIN}/a`, `${SAME_ORIGIN}/b`])
      enterPageInteractive(a)
      harness.clearBroadcasts()

      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, clickEvent())
      const requestId = requestIdFor(b)

      handleResolveInteractionLocatorResponse(
        findPageById(b)!.pageView.webContents as never,
        ambiguousResponse(requestId),
      )

      expect(dispatchesOn(b)).toHaveLength(0)
      // A refused click is the one meaning of the 'refused' activity (wiggle).
      expect(syncedCursorFor(b)?.activity).toBe('refused')
    })
  })

  describe('staleness (D7)', () => {
    it('drops a resolve response whose requestId was superseded', async () => {
      const [a, b] = await makeSyncedSet([`${SAME_ORIGIN}/a`, `${SAME_ORIGIN}/b`])
      enterPageInteractive(a)
      harness.clearBroadcasts()

      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, clickEvent())
      const staleRequestId = requestIdFor(b)

      // A newer capture supersedes the first outstanding request for this peer.
      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, clickEvent())
      const freshRequestId = requestIdFor(b)
      expect(freshRequestId).not.toBe(staleRequestId)

      // The stale confident answer must be ignored — no dispatch.
      handleResolveInteractionLocatorResponse(
        findPageById(b)!.pageView.webContents as never,
        confidentResponse(staleRequestId),
      )
      await settleSync()
      expect(dispatchesOn(b)).toHaveLength(0)

      // The current answer still replays, proving only staleness was rejected.
      handleResolveInteractionLocatorResponse(
        findPageById(b)!.pageView.webContents as never,
        confidentResponse(freshRequestId),
      )
      await settleSync()
      expect(dispatchesOn(b)).toHaveLength(2)
    })
  })

  describe('unsync stops mirroring (D6)', () => {
    it('retires synced cursors and mirrors nothing after the set dissolves', async () => {
      const [a, b] = await makeSyncedSet([`${SAME_ORIGIN}/a`, `${SAME_ORIGIN}/b`])
      enterPageInteractive(a)
      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())
      expect(syncedCursorFor(b)).toBeDefined()

      // Unsync a; b is now a lone member so the whole set dissolves, which
      // retires the source's synced cursors via refreshInteractionSyncCapture.
      unsyncPage(a)
      await settleSync()
      expect(syncedCursors()).toHaveLength(0)

      harness.clearBroadcasts()
      // Further captured input from the (now unsynced) source mirrors nothing.
      handleInteractionSyncEvent(findPageById(a)!.pageView.webContents as never, hoverEvent())
      expect(syncedCursors()).toHaveLength(0)
      expect(resolveRequestsTo(b)).toHaveLength(0)
      expect(dispatchesOn(b)).toHaveLength(0)
    })
  })
})

/**
 * D5 — a mirrored click on a link cascades into nav sync. Exactly one
 * navigation per peer, and the source never double-navigates, regardless of
 * whether the source's own navigation or a peer's mirrored-click navigation
 * reaches main first. Both orderings are driven through the real machinery:
 * a guest navigation is a `did-navigate` on the stub webContents; a
 * sync-induced navigation is a loadURL, which the stub records.
 */
describe('interaction sync — navigation race (D5)', () => {
  const START = `${SAME_ORIGIN}/home`
  const TARGET = `${SAME_ORIGIN}/next`

  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    clearAutomationInteractivePageIds()
    exitPageInteractive()
    removeAllSyncedCursors()
    harness.clearBroadcasts()
  })

  afterAll(() => harness?.dispose())

  function emitDidNavigate(id: string, url: string): void {
    wc(id).emit('did-navigate', {}, url)
  }

  /** Represent a guest's own navigation: the browser is now at `url`. */
  function guestNavigated(id: string, url: string): void {
    wc(id).loadURL(url)
  }

  /** Zero the loadURL counters after setup so only race-induced (sync)
   *  navigations are counted, not page creation or guest-nav bookkeeping. */
  function clearNavCounters(ids: string[]): void {
    for (const id of ids) wc(id).loadedUrls.length = 0
  }

  async function makeThreePageSet(): Promise<[string, string, string]> {
    const [s, p1, p2] = await makeSyncedSet([START, START, START])
    return [s, p1, p2]
  }

  it('source navigates first: each peer navigates once, source does not', async () => {
    const [s, p1, p2] = await makeThreePageSet()

    // Source's own link click navigates its guest, then main processes it.
    guestNavigated(s, TARGET)
    clearNavCounters([s, p1, p2])
    emitDidNavigate(s, TARGET)
    // The mirrored clicks land on the peers (already loaded to TARGET by the
    // propagate); their echo did-navigate must be absorbed by suppression.
    emitDidNavigate(p1, TARGET)
    emitDidNavigate(p2, TARGET)

    expect(wc(s).loadedUrls).toHaveLength(0)
    expect(wc(p1).loadedUrls).toEqual([TARGET])
    expect(wc(p2).loadedUrls).toEqual([TARGET])
    expect(wc(p1).getURL()).toBe(TARGET)
    expect(wc(p2).getURL()).toBe(TARGET)
  })

  it('a peer navigates first: source is not re-navigated, each peer once', async () => {
    const [s, p1, p2] = await makeThreePageSet()

    // The source guest already navigated (synchronous user click) but main
    // has not processed its did-navigate yet; a peer's mirrored-click
    // navigation reaches main first.
    guestNavigated(s, TARGET)
    guestNavigated(p1, TARGET)
    clearNavCounters([s, p1, p2])

    emitDidNavigate(p1, TARGET)
    // p1's propagate hits the source, which is already at TARGET: the
    // URL-equality guard must skip it so the source is not re-navigated.
    emitDidNavigate(s, TARGET)
    emitDidNavigate(p2, TARGET)

    expect(wc(s).loadedUrls).toHaveLength(0)
    expect(wc(p1).loadedUrls).toHaveLength(0)
    expect(wc(p2).loadedUrls).toEqual([TARGET])
    expect(wc(s).getURL()).toBe(TARGET)
    expect(wc(p1).getURL()).toBe(TARGET)
    expect(wc(p2).getURL()).toBe(TARGET)
  })
})
